use atty::Stream;
use slog::Drain;
use std::sync::Mutex;

use slog::{o, Logger};
use slog_term::{TermDecorator, CompactFormat, FullFormat};

pub mod prelude {
    pub use slog::{info, warn, error, debug, trace, crit, o, Logger};
}

/**
 * Initialise a logger which writes to stdout, and which does the right thing on
 * both an interactive terminal and when stdout is not a tty.
 */
pub fn init_log() -> Logger {
    let dec = TermDecorator::new().stdout().build();
    if atty::is(Stream::Stdout) {
        let dr = Mutex::new(CompactFormat::new(dec).build())
            .fuse();
        Logger::root(dr, o!())
    } else {
        let dr = Mutex::new(FullFormat::new(dec).use_original_order().build())
            .fuse();
        Logger::root(dr, o!())
    }
}
