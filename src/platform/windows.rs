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
use std::io;
use std::ops::Range;
use std::ptr;
use winapi::ctypes::c_long;
use winapi::shared::minwindef::DWORD;
use winapi::shared::ntdef::HANDLE;
use winapi::um::synchapi::ReleaseSemaphore;
use winapi::um::wincon::{CTRL_C_EVENT, CTRL_SHUTDOWN_EVENT};

/// Platform specific error type
pub type Error = io::Error;

/// Platform specific signal type
pub type Signal = DWORD;

/// TODO Platform specific pipe handle type
pub type SignalEmitter = HANDLE;
impl SignalEvent for SignalEmitter {
    fn emit(&self, _signal: &Signal) {
        unsafe { ReleaseSemaphore(*self, 1, ptr::null_mut()) };
    }
}

pub const CTRL_C_SIGNAL: Signal = CTRL_C_EVENT;
pub const UNINITIALIZED_SIGNAL_EMITTER: HANDLE = winapi::um::handleapi::INVALID_HANDLE_VALUE;

/// Iterator returning available signals on this system
pub fn signal_iterator() -> Range<DWORD> {
    CTRL_C_EVENT..CTRL_SHUTDOWN_EVENT + 1
}

pub const MAX_SEM_COUNT: c_long = 255;

impl SignalType {
    /// Get the underlying platform specific signal
    pub fn to_platform_signal(&self) -> Signal {
        match *self {
            SignalType::Ctrlc => CTRL_C_EVENT,
            SignalType::Other(s) => s,
        }
    }
}
