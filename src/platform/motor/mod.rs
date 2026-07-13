// Motor OS: no asynchronous signal delivery exists. Handler registration
// succeeds (so rustc's Ctrl-C setup is happy) and the waiter thread simply
// parks forever.

use crate::error::Error as CtrlcError;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(());

impl Error {
    /// Referenced by `crate::error`; never produced on Motor.
    pub const EEXIST: Error = Error(());
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "signals are not supported on Motor OS")
    }
}

impl std::error::Error for Error {}

pub type Signal = i32;

pub unsafe fn init_os_handler(_overwrite: bool) -> Result<(), Error> {
    Ok(())
}

pub unsafe fn block_ctrl_c() -> Result<(), CtrlcError> {
    loop {
        std::thread::park();
    }
}
