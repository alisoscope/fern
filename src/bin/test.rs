use fern::*;

use prelude::*;

pub fn main() -> anyhow::Result<()> {
    let _logger = logging::GlobalLoggerContext::init(logging::LoggerConfig {
        log_out_mode: logging::LogOutMode::PrintWithAnsiCodes,
        trim_newlines: true,
    });
    log_release!(LogType::Info, "hello world");

    Ok(())
}
