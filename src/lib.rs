
pub mod logging;

pub mod prelude {
    pub use crate::logging::LogType;
    pub use crate::log_panic;
    pub use crate::log_release;
    pub use crate::log_debug;
    pub use crate::log_verbose_debug;
}
