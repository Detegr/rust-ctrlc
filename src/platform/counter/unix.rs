// Copyright (c) 2018 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use self::nix::sys::signal as nix_signal;
use crate::error::Error;
use crate::platform;
use crate::platform::unix::nix;
use crate::platform::unix::Signal;
use crate::signalmap::SIGMAP;
use std::convert::TryFrom;

extern "C" fn os_handler(signum: nix::libc::c_int) {
    use std::sync::atomic::Ordering;
    let counter = Signal::try_from(signum)
        .ok()
        .and_then(|signal| SIGMAP.get_counter(&signal));
    if let Some(counter) = counter {
        counter.fetch_add(1, Ordering::AcqRel);
    }
}

pub fn set_handler(platform_signal: Signal) -> Result<(), Error> {
    let handler = nix_signal::SigHandler::Handler(os_handler);
    let new_action = nix_signal::SigAction::new(
        handler,
        nix_signal::SaFlags::SA_RESTART,
        nix_signal::SigSet::empty(),
    );
    // SAFETY: FFI
    let old = unsafe { nix_signal::sigaction(platform_signal, &new_action)? };
    if old.handler() != nix_signal::SigHandler::SigDfl {
        platform::revert_sighandler_to_default(platform_signal);
        return Err(Error::MultipleHandlers);
    }
    Ok(())
}

pub fn reset_handler(platform_signal: Signal) {
    platform::revert_sighandler_to_default(platform_signal);
}
