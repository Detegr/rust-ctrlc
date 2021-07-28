// Copyright (c) 2021 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use crate::platform;
use std::fmt;

/// Ctrl-C error.
#[derive(Debug)]
pub enum Error {
    /// Channel is empty
    ChannelEmpty,
    /// Signal could not be found from the system.
    NoSuchSignal(crate::Signal),
    /// Ctrl-C signal handler already registered.
    MultipleHandlers,
    /// Too many signals registered for a channel.
    TooManySignals,
    /// Unexpected system error.
    System(std::io::Error),
}
impl PartialEq for Error {
    fn eq(&self, e: &Error) -> bool {
        match (self, e) {
            (&Error::ChannelEmpty, &Error::ChannelEmpty) => true,
            (&Error::NoSuchSignal(ref lhs), &Error::NoSuchSignal(ref rhs)) => lhs == rhs,
            (&Error::MultipleHandlers, &Error::MultipleHandlers) => true,
            (&Error::System(ref lhs), &Error::System(ref rhs)) => {
                if lhs.kind() != rhs.kind() {
                    return false;
                }
                lhs.raw_os_error() == rhs.raw_os_error()
            }
            _ => false,
        }
    }
}

impl Error {
    fn describe(&self) -> &str {
        match *self {
            Error::ChannelEmpty => "Channel is empty",
            Error::NoSuchSignal(_) => "Signal could not be found from the system",
            Error::MultipleHandlers => "Ctrl-C signal handler already registered",
            Error::TooManySignals => "Too many signals registered for a channel",
            Error::System(_) => "Unexpected system error",
        }
    }
}

impl From<platform::Error> for Error {
    fn from(e: platform::Error) -> Error {
        let system_error = std::io::Error::new(std::io::ErrorKind::Other, e);
        Error::System(system_error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ctrl-C error: {}", self.describe())
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        self.describe()
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match *self {
            Error::System(ref e) => Some(e),
            _ => None,
        }
    }
}
