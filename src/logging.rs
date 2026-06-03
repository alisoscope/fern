use std::cell::OnceCell;
use std::fmt;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, RwLock, mpsc};
use std::thread;

static GLOBAL_LOGGER: RwLock<Option<Logger>> = RwLock::new(None);

thread_local! {
    static LOCAL_THREAD_NAME: OnceCell<Option<Arc<str>>> = const { OnceCell::new() };
}

pub fn log_global(log_type: LogType, args: fmt::Arguments) {
    if let Ok(logger) = GLOBAL_LOGGER.read() {
        if let Some(logger) = logger.as_ref() {
            logger.log(log_type, args);
        } else {
            eprintln!("Attempted to log while global logger not initialised.");
        }
    } else {
        eprintln!("Attempted to log but global logger is poisoned.");
    }
}

/// Handles cleanly closing the global logger.
pub struct GlobalLoggerContext;

impl GlobalLoggerContext {
    pub fn init(config: LoggerConfig) -> Self {
        let mut guard = GLOBAL_LOGGER.write().unwrap();
        if let Some(logger) = guard.as_ref() {
            let args = format_args!(
                "Attempted to initialise global logger when it is already initialised."
            );
            logger.log(LogType::Panic, args);
            panic!("{}", args);
        }

        let logger = Logger::new(config).unwrap();
        logger.log(LogType::Info, format_args!("Initialised logging."));
        guard.replace(logger);

        Self
    }
}

impl Drop for GlobalLoggerContext {
    fn drop(&mut self) {
        let logger = GLOBAL_LOGGER.write().unwrap().take();

        assert!(logger.is_some());

        if let Some(logger) = logger {
            logger.log(LogType::Info, format_args!("Closing global logger."));
            drop(logger.send);
            if let Err(e) = logger.worker_thread.join() {
                eprintln!("Failed to join global logger thread with error: {:?}.", e);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LogType {
    Info,
    Warning,
    Error,
    Panic,
}

pub struct LogSpec {
    pub date_time: chrono::DateTime<chrono::Local>,
    pub log_type: LogType,
    pub thread_name: Option<Arc<str>>,
    pub message: Box<str>,
}

#[derive(Clone, Debug)]
pub struct LoggerConfig {
    pub log_out_mode: LogOutMode,
    pub trim_newlines: bool,
}

#[derive(Clone, Debug)]
pub enum LogOutMode {
    FileOnly,
    Print,
    PrintWithAnsiCodes,
}

pub struct Logger {
    send: mpsc::Sender<LogSpec>,
    worker_thread: thread::JoinHandle<()>,
}

macro_rules! log_fmt {
    ($($args:tt)*) => {
        format_args!(
            "[{}] \"{}\" [{}]: {}",
            $($args)*
        )
    };
}

impl Logger {
    pub const LOGGER_THREAD_STACK_SIZE: usize = 64 * 1024; // 64 KiB
    pub const LOG_DIR: &'static str = "logs";
    pub const FILENAME_FMT: &'static str = "%Y-%m-%d_%H-%M-%S.log";
    pub const LOG_TIME_FMT: &'static str = "%H:%M:%S:%f";
    pub const LATEST_LOG_FILE_NAME: &'static str = "latest.log";

    pub fn new(config: LoggerConfig) -> Result<Self, io::Error> {
        let log_dir_path = Path::new(Self::LOG_DIR);
        fs::DirBuilder::new().recursive(true).create(log_dir_path)?;

        let date_time = chrono::Local::now();

        let file_name = format!("{}", date_time.format(Self::FILENAME_FMT));
        let file_path = log_dir_path.join(&file_name);

        let file = fs::File::create_new(&file_path)?;

        let latest_path = log_dir_path.join(Self::LATEST_LOG_FILE_NAME);
            
        let mut tries = 0;
        // retry loop to recreate the hard link
        while let Err(err) = fs::hard_link(&file_path, &latest_path) {
            if err.kind() == io::ErrorKind::AlreadyExists &&
                tries < 3 {
                fs::remove_file(&latest_path)?;
            } else {
                return Err(err);
            }
            tries += 1;
        }

        let (send, recv) = mpsc::channel();

        let thread = thread::Builder::new()
            .stack_size(Self::LOGGER_THREAD_STACK_SIZE)
            .name("Global logger thread".to_owned())
            .spawn(move || Self::worker_fn(config, recv, io::BufWriter::new(file)))?;

        Ok(Self {
            send,
            worker_thread: thread,
        })
    }
    fn worker_fn(
        config: LoggerConfig,
        recv: mpsc::Receiver<LogSpec>,
        mut file: io::BufWriter<fs::File>,
    ) {
        let stderr = io::stderr();
        while let Ok(first_log) = recv.recv() {
            let mut stderr = match config.log_out_mode {
                LogOutMode::Print | LogOutMode::PrintWithAnsiCodes => Some(stderr.lock()),
                LogOutMode::FileOnly => None,
            };
            for log in [first_log].into_iter().chain(recv.try_iter()) {
                let thread_name = if let Some(name) = &log.thread_name {
                    name
                } else {
                    "<unknown>"
                };

                let message = if config.trim_newlines {
                    log.message.trim_end_matches('\n')
                } else {
                    &log.message
                };

                let fmt = log_fmt!(
                    log.date_time.format(Self::LOG_TIME_FMT),
                    thread_name,
                    log.log_type.message(),
                    message,
                );

                if let Some(stderr) = stderr.as_mut() {
                    match config.log_out_mode {
                        LogOutMode::Print => writeln!(stderr, "{}", fmt).unwrap(),
                        LogOutMode::PrintWithAnsiCodes => writeln!(stderr, "{}{}\x1b[0m", log.log_type.ansi_code(), fmt).unwrap(),
                        LogOutMode::FileOnly => unreachable!(),
                    }
                }

                if let Err(e) = file.write_fmt(format_args!("{}\n", fmt)) {
                    eprint!(
                        "Failed to write to log file. Error: {}. Log: {}\n",
                        e,
                        fmt,
                    );
                }
            }
            if let Err(e) = file.flush() {
                eprintln!("Failed to flush log file. Error: {}.", e);
            }
        }
    }
    pub fn log(&self, log_type: LogType, args: fmt::Arguments) {
        let date_time = chrono::Local::now();

        let thread_name = LOCAL_THREAD_NAME.with(|cell| {
            cell.get_or_init(|| {
                thread::current().name().map(Arc::from)
            })
            .clone()
        });

        let spec = LogSpec {
            date_time,
            log_type,
            thread_name,
            message: format!("{}", args).into_boxed_str(),
        };

        if let Err(e) = self.send.send(spec) {
            eprintln!("Failed to send log with error: {}.", e);
        }
    }
}

#[cfg(feature = "log-level-panics")]
#[macro_export]
macro_rules! log_panic {
    ($($arg:tt)*) => {
        {
            $crate::logging::log_global($crate::logging::LogType::Panic, format_args!($($arg)*));
            panic!($($arg)*);
        }
    };
}

#[cfg(not(feature = "log-level-panics"))]
#[macro_export]
macro_rules! log_panic {
    ($($arg:tt)*) => {
        panic!($($arg)*)
    };
}

#[cfg(feature = "log-level-release")]
#[macro_export]
macro_rules! log_release {
    ($type:expr, $($arg:tt)*) => {
        $crate::logging::log_global($type, format_args!($($arg)*))
    };
}

#[cfg(not(feature = "log-level-release"))]
#[macro_export]
macro_rules! log_release {
    ($type:expr, $($arg:tt)*) => {()};
}

#[cfg(feature = "log-level-debug")]
#[macro_export]
macro_rules! log_debug {
    ($type:expr, $($arg:tt)*) => {
        $crate::logging::log_global($type, format_args!($($arg)*))
    };
}

#[cfg(not(feature = "log-level-debug"))]
#[macro_export]
macro_rules! log_debug {
    ($type:expr, $($arg:tt)*) => {()};
}

#[cfg(feature = "log-level-verbose-debug")]
#[macro_export]
macro_rules! log_verbose_debug {
    ($type:expr, $($arg:tt)*) => {
        $crate::logging::log_global($type, format_args!($($arg)*))
    };
}

#[cfg(not(feature = "log-level-verbose-debug"))]
#[macro_export]
macro_rules! log_verbose_debug {
    ($type:expr, $($arg:tt)*) => {()};
}

impl LogType {
    pub fn message(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
            Self::Panic => "Panic!",
        }
    }
    pub fn ansi_code(self) -> &'static str {
        match self {
            Self::Info => "\x1b[0;37m", // white
            Self::Warning => "\x1b[1;33m", // yellow
            Self::Error => "\x1b[1;31m", // red
            Self::Panic => "\x1b[1;4;31m", // red underlined
        }
    }
}
