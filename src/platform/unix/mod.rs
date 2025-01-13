// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

#[cfg(feature = "unix_use_semaphore")]
mod semaphore;

#[cfg(not(feature = "unix_use_semaphore"))]
mod pipe;

#[cfg(feature = "unix_use_semaphore")]
pub use self::semaphore::*;

#[cfg(not(feature = "unix_use_semaphore"))]
pub use self::pipe::*;

/// Platform specific error type
pub type Error = nix::Error;

/// Platform specific signal type
pub type Signal = nix::sys::signal::Signal;

extern "C" fn os_handler(_: nix::libc::c_int) {
    // Assuming this always succeeds. Can't really handle errors in any meaningful way.
    unsafe {
        #[cfg(feature = "unix_use_semaphore")]
        os_handler_sem();
        #[cfg(not(feature = "unix_use_semaphore"))]
        os_handler_pipe();
    }
}

/// Register os signal handler.
///
/// Must be called before calling [`block_ctrl_c()`](fn.block_ctrl_c.html)
/// and should only be called once.
///
/// # Errors
/// Will return an error if a system error occurred.
///
#[inline]
pub unsafe fn init_os_handler(overwrite: bool) -> Result<(), Error> {
    use nix::sys::signal;

    #[cfg(feature = "unix_use_semaphore")]
    init_sem()?;
    #[cfg(not(feature = "unix_use_semaphore"))]
    init_pipe()?;

    let cleanup = |e: nix::Error| -> Error {
        #[cfg(feature = "unix_use_semaphore")]
        cleanup_sem();
        #[cfg(not(feature = "unix_use_semaphore"))]
        cleanup_pipe();
        e
    };

    let handler = signal::SigHandler::Handler(os_handler);
    #[cfg(not(target_os = "nto"))]
    let new_action = signal::SigAction::new(
        handler,
        signal::SaFlags::SA_RESTART,
        signal::SigSet::empty(),
    );
    // SA_RESTART is not supported on QNX Neutrino 7.1 and before
    #[cfg(target_os = "nto")]
    let new_action =
        signal::SigAction::new(handler, signal::SaFlags::empty(), signal::SigSet::empty());

    let sigint_old = match signal::sigaction(signal::Signal::SIGINT, &new_action) {
        Ok(old) => old,
        Err(e) => return Err(cleanup(e)),
    };
    if !overwrite && sigint_old.handler() != signal::SigHandler::SigDfl {
        signal::sigaction(signal::Signal::SIGINT, &sigint_old).unwrap();
        return Err(cleanup(nix::Error::EEXIST));
    }

    #[cfg(feature = "termination")]
    {
        let sigterm_old = match signal::sigaction(signal::Signal::SIGTERM, &new_action) {
            Ok(old) => old,
            Err(e) => {
                signal::sigaction(signal::Signal::SIGINT, &sigint_old).unwrap();
                return Err(cleanup(e));
            }
        };
        if !overwrite && sigterm_old.handler() != signal::SigHandler::SigDfl {
            signal::sigaction(signal::Signal::SIGINT, &sigint_old).unwrap();
            signal::sigaction(signal::Signal::SIGTERM, &sigterm_old).unwrap();
            return Err(cleanup(nix::Error::EEXIST));
        }
        let sighup_old = match signal::sigaction(signal::Signal::SIGHUP, &new_action) {
            Ok(old) => old,
            Err(e) => {
                signal::sigaction(signal::Signal::SIGINT, &sigint_old).unwrap();
                signal::sigaction(signal::Signal::SIGTERM, &sigterm_old).unwrap();
                return Err(cleanup(e));
            }
        };
        if !overwrite && sighup_old.handler() != signal::SigHandler::SigDfl {
            signal::sigaction(signal::Signal::SIGINT, &sigint_old).unwrap();
            signal::sigaction(signal::Signal::SIGTERM, &sigterm_old).unwrap();
            signal::sigaction(signal::Signal::SIGHUP, &sighup_old).unwrap();
            return Err(cleanup(nix::Error::EEXIST));
        }
    }

    Ok(())
}
