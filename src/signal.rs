// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use platform;

/// A cross-platform way to represent Ctrl-C or program termination signal. Other
/// signals are supported via `Other`-variant.
#[derive(Clone, Copy, Debug)]
pub enum SignalType {
    /// Ctrl-C
    /// Maps to `SIGINT` on *nix, `CTRL_C_EVENT` on Windows.
    Ctrlc,
    /// Program termination
    /// Maps to `SIGTERM` on *nix, `CTRL_CLOSE_EVENT` on Windows.
    Termination,
    /// Other signal using platform-specific data
    Other(platform::Signal),
}

impl Into<platform::Signal> for SignalType {
    fn into(self) -> platform::Signal {
        match self {
            SignalType::Ctrlc => platform::CTRL_C_SIGNAL,
            SignalType::Termination => platform::TERMINATION_SIGNAL,
            SignalType::Other(s) => s,
        }
    }
}

impl From<platform::Signal> for SignalType {
    fn from(platform_signal: platform::Signal) -> SignalType {
        match platform_signal {
            platform::CTRL_C_SIGNAL => SignalType::Ctrlc,
            platform::TERMINATION_SIGNAL => SignalType::Termination,
            s => SignalType::Other(s),
        }
    }
}
