// Copyright (c) 2018 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use error::Error;
use platform::windows::winapi::{BOOL, DWORD, FALSE, TRUE};
use platform::windows::{Signal, kernel32};
use signalmap::SIGNALS;
use std::io;
use std::sync::atomic::Ordering;

unsafe extern "system" fn os_handler(event: DWORD) -> BOOL {
    use std::sync::atomic::Ordering;
    let counter = SIGNALS.get_counter(&event);
    if let Some(counter) = counter {
        counter.fetch_add(1, Ordering::AcqRel);
    }
    TRUE
}

pub fn set_handler(platform_signal: Signal) -> Result<(), Error> {
    let sig_index = SIGNALS
        .index_of(&platform_signal)
        .expect("Validity of signal is checked earlier");
    let initialized = &SIGNALS.initialized[sig_index];
    if initialized.compare_and_swap(false, true, Ordering::AcqRel) {
        return Err(Error::MultipleHandlers);
    }
    if unsafe { kernel32::SetConsoleCtrlHandler(Some(os_handler), TRUE) } == FALSE {
        let e = io::Error::last_os_error();
        return Err(e.into());
    }
    Ok(())
}

pub fn reset_handler(platform_signal: Signal) {
    let sig_index = SIGNALS
        .index_of(&platform_signal)
        .expect("Validity of signal is checked earlier");
    let initialized = &SIGNALS.initialized[sig_index];
    if unsafe { kernel32::SetConsoleCtrlHandler(Some(os_handler), FALSE) } == FALSE {
        unreachable!("Should not fail");
    }
    initialized.compare_and_swap(true, false, Ordering::AcqRel);
}
