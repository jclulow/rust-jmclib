use atty::Stream;
use slog::Drain;
use std::sync::Mutex;

use slog::{o, Discard, Logger};
use slog_term::{CompactFormat, FullFormat, TermDecorator};

pub mod prelude {
    pub use slog::{crit, debug, error, info, o, trace, warn, Logger};
}

/**
 * Initialise a logger which writes to stdout, and which does the right thing on
 * both an interactive terminal and when stdout is not a tty.
 */
pub fn init_log() -> Logger {
    let dec = TermDecorator::new().stdout().build();
    if atty::is(Stream::Stdout) {
        let dr = Mutex::new(CompactFormat::new(dec).build()).fuse();
        Logger::root(dr, o!())
    } else {
        let dr = Mutex::new(FullFormat::new(dec).use_original_order().build())
            .fuse();
        Logger::root(dr, o!())
    }
}

/**
 * Return a logger which discards all log output.
 */
pub fn discard() -> Logger {
    Logger::root(Discard, o!())
}
