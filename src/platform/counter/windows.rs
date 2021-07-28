// Copyright (c) 2018 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use crate::error::Error;
use crate::platform;
use crate::signalmap::SIGMAP;
use platform::winapi::um::consoleapi::SetConsoleCtrlHandler;
use platform::windows::winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
use platform::windows::Signal;
use std::convert::TryFrom;
use std::io;
use std::sync::atomic::Ordering;

// SAFETY: FFI
unsafe extern "system" fn os_handler(event: DWORD) -> BOOL {
    if let Ok(signal) = Signal::try_from(event) {
        let counter = SIGMAP.get_counter(&signal);
        if let Some(counter) = counter {
            counter.fetch_add(1, Ordering::AcqRel);
        }
    }
    TRUE
}

pub fn set_handler(platform_signal: Signal) -> Result<(), Error> {
    let sig_index = SIGMAP
        .index_of(&platform_signal)
        .expect("Validity of signal is checked earlier");
    let initialized = &SIGMAP.initialized[sig_index];
    if initialized
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(Error::MultipleHandlers);
    }
    // SAFETY: FFI
    if unsafe { SetConsoleCtrlHandler(Some(os_handler), TRUE) } == FALSE {
        let e = io::Error::last_os_error();
        return Err(e.into());
    }
    Ok(())
}

pub fn reset_handler(platform_signal: Signal) {
    let sig_index = SIGMAP
        .index_of(&platform_signal)
        .expect("Validity of signal is checked earlier");
    let initialized = &SIGMAP.initialized[sig_index];
    // SAFETY: FFI
    if unsafe { SetConsoleCtrlHandler(Some(os_handler), FALSE) } == FALSE {
        unreachable!("Should not fail");
    }
    let _ = initialized.compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire);
}
