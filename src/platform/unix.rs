// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

extern crate nix;

use error::Error;
use self::nix::unistd;
use std::os::unix::io::RawFd;
use std::io;

static mut PIPE: (RawFd, RawFd) = (-1, -1);

extern "C" fn os_handler(_: nix::c_int) {
    // Assuming this always succeeds. Can't really handle errors in any meaningful way.
    unsafe {
        unistd::write(PIPE.1, &[0u8]).is_ok();
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
pub unsafe fn init_os_handler() -> Result<(), Error> {
    use self::nix::fcntl;
    use self::nix::sys::signal;

    PIPE = unistd::pipe2(fcntl::O_CLOEXEC).map_err(|e| Error::System(e.into()))?;

    let close_pipe = |e: nix::Error| -> Error {
        unistd::close(PIPE.1).is_ok();
        unistd::close(PIPE.0).is_ok();
        Error::System(e.into())
    };

    // Make sure we never block on write in the os handler.
    if let Err(e) = fcntl::fcntl(PIPE.1, fcntl::FcntlArg::F_SETFL(fcntl::O_NONBLOCK)) {
        return Err(close_pipe(e));
    }

    let handler = signal::SigHandler::Handler(os_handler);
    let new_action = signal::SigAction::new(handler, signal::SA_RESTART, signal::SigSet::empty());

    let _old = match signal::sigaction(signal::Signal::SIGINT, &new_action) {
        Ok(old) => old,
        Err(e) => return Err(close_pipe(e)),
    };

    #[cfg(feature = "termination")]
    match signal::sigaction(signal::Signal::SIGTERM, &new_action) {
        Ok(_) => {}
        Err(e) => {
            signal::sigaction(signal::Signal::SIGINT, &_old).unwrap();
            return Err(close_pipe(e));
        }
    }

    // TODO: Maybe throw an error if old action is not SigDfl.

    Ok(())
}

/// Blocks until a Ctrl-C signal is received.
///
/// Must be called after calling [`init_os_handler()`](fn.init_os_handler.html).
///
/// # Errors
/// Will return an error if a system error occurred.
///
#[inline]
pub unsafe fn block_ctrl_c() -> Result<(), Error> {
    let mut buf = [0u8];

    // TODO: Can we safely convert the pipe fd into a std::io::Read
    // with std::os::unix::io::FromRawFd, this would handle EINTR
    // and everything for us.
    loop {
        match unistd::read(PIPE.0, &mut buf[..]) {
            Ok(1) => break,
            Ok(_) => return Err(Error::System(io::ErrorKind::UnexpectedEof.into()).into()),
            Err(nix::Error::Sys(nix::Errno::EINTR)) => {}
            Err(e) => return Err(Error::System(e.into())),
        }
    }

    Ok(())
}
