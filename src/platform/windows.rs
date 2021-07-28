// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

pub use winapi;

use crate::signal::SignalType;
use crate::signalevent::SignalEvent;
use std::convert::TryFrom;
use std::io;
use std::ptr;
use winapi::ctypes::c_long;
use winapi::shared::minwindef::DWORD;
use winapi::shared::ntdef::HANDLE;
use winapi::um::synchapi::ReleaseSemaphore;
use winapi::um::wincon;

/// Platform specific error type
pub type Error = io::Error;

#[allow(non_camel_case_types)]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Wrapper enum for winapi's CTRL events
pub enum Signal {
    /// Variant for [CTRL_C_EVENT](../winapi/um/wincon/constant.CTRL_C_EVENT.html)
    CTRL_C_EVENT = wincon::CTRL_C_EVENT,
    /// Variant for [CTRL_BREAK_EVENT](../winapi/um/wincon/constant.CTRL_BREAK_EVENT.html)
    CTRL_BREAK_EVENT = wincon::CTRL_BREAK_EVENT,
    /// Variant for [CTRL_CLOSE_EVENT](../winapi/um/wincon/constant.CTRL_CLOSE_EVENT.html)
    CTRL_CLOSE_EVENT = wincon::CTRL_CLOSE_EVENT,
    /// Variant for [CTRL_LOGOFF_EVENT](../winapi/um/wincon/constant.CTRL_LOGOFF_EVENT.html)
    CTRL_LOGOFF_EVENT = wincon::CTRL_LOGOFF_EVENT,
    /// Variant for [CTRL_SHUTDOWN_EVENT](../winapi/um/wincon/constant.CTRL_SHUTDOWN_EVENT.html)
    CTRL_SHUTDOWN_EVENT = wincon::CTRL_SHUTDOWN_EVENT,
}

impl TryFrom<DWORD> for Signal {
    type Error = ();

    fn try_from(event: DWORD) -> Result<Self, Self::Error> {
        match event {
            wincon::CTRL_C_EVENT => Ok(Signal::CTRL_C_EVENT),
            wincon::CTRL_BREAK_EVENT => Ok(Signal::CTRL_BREAK_EVENT),
            wincon::CTRL_CLOSE_EVENT => Ok(Signal::CTRL_CLOSE_EVENT),
            wincon::CTRL_LOGOFF_EVENT => Ok(Signal::CTRL_LOGOFF_EVENT),
            wincon::CTRL_SHUTDOWN_EVENT => Ok(Signal::CTRL_SHUTDOWN_EVENT),
            _ => Err(()),
        }
    }
}

/// Platform specific signal emitter type
pub type SignalEmitter = HANDLE;
impl SignalEvent for SignalEmitter {
    fn emit(&self, _signal: &Signal) {
        // SAFETY: FFI
        unsafe { ReleaseSemaphore(*self, 1, ptr::null_mut()) };
    }
}

pub const CTRL_C_SIGNAL: Signal = Signal::CTRL_C_EVENT;
pub const UNINITIALIZED_SIGNAL_EMITTER: HANDLE = winapi::um::handleapi::INVALID_HANDLE_VALUE;

static SIGNALS: [Signal; 5] = [
    Signal::CTRL_C_EVENT,
    Signal::CTRL_BREAK_EVENT,
    Signal::CTRL_CLOSE_EVENT,
    Signal::CTRL_LOGOFF_EVENT,
    Signal::CTRL_SHUTDOWN_EVENT,
];

/// Iterator returning available signals on this system
pub fn signal_iterator() -> impl Iterator<Item = Signal> {
    SIGNALS.iter().cloned()
}

pub const MAX_SEM_COUNT: c_long = 255;

impl SignalType {
    /// Get the underlying platform specific signal
    pub fn to_platform_signal(&self) -> Signal {
        match *self {
            SignalType::Ctrlc => Signal::CTRL_C_EVENT,
            SignalType::Other(s) => s,
        }
    }
}
