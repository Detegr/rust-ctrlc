use std;
use std::fmt;

/// Ctrl-C error.
#[derive(Debug)]
pub enum Error {
    /// Signal could not be found from the system.
    NoSuchSignal(::SignalType),
    /// Ctrl-C signal handler already registered.
    MultipleHandlers,
    /// Unexpected system error.
    System(std::io::Error),
}

impl From<::platform::Error> for Error {
    fn from(e: ::platform::Error) -> Error {
        let system_error = std::io::Error::new(std::io::ErrorKind::Other, e);
        Error::System(system_error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        write!(f, "Ctrl-C error: {}", self.description())
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::NoSuchSignal(_) => "Signal could not be found from the system",
            Error::MultipleHandlers => "Ctrl-C signal handler already registered",
            Error::System(_) => "Unexpected system error",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::System(ref e) => Some(e),
            _ => None,
        }
    }
}
