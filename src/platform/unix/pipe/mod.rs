use nix::unistd;
use std::os::fd::BorrowedFd;
use std::os::fd::IntoRawFd;
use std::os::unix::io::RawFd;

pub type Error = nix::Error;

use crate::error::Error as CtrlcError;

static mut PIPE: (RawFd, RawFd) = (-1, -1);

#[inline]
pub unsafe fn os_handler_pipe() -> () {
    let fd = BorrowedFd::borrow_raw(PIPE.1);
    let _ = unistd::write(fd, &[0u8]);
}

// pipe2(2) is not available on macOS, iOS, AIX or Haiku, so we need to use pipe(2) and fcntl(2)
#[inline]
#[cfg(any(
    target_os = "ios",
    target_os = "macos",
    target_os = "haiku",
    target_os = "aix",
    target_os = "nto",
))]
fn pipe2(flags: nix::fcntl::OFlag) -> nix::Result<(RawFd, RawFd)> {
    use nix::fcntl::{fcntl, FcntlArg, FdFlag, OFlag};

    let pipe = unistd::pipe()?;
    let pipe = (pipe.0.into_raw_fd(), pipe.1.into_raw_fd());

    let mut res = Ok(0);

    if flags.contains(OFlag::O_CLOEXEC) {
        res = res
            .and_then(|_| fcntl(pipe.0, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC)))
            .and_then(|_| fcntl(pipe.1, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC)));
    }

    if flags.contains(OFlag::O_NONBLOCK) {
        res = res
            .and_then(|_| fcntl(pipe.0, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)))
            .and_then(|_| fcntl(pipe.1, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)));
    }

    match res {
        Ok(_) => Ok(pipe),
        Err(e) => {
            let _ = unistd::close(pipe.0);
            let _ = unistd::close(pipe.1);
            Err(e)
        }
    }
}

#[inline]
#[cfg(not(any(
    target_os = "ios",
    target_os = "macos",
    target_os = "haiku",
    target_os = "aix",
    target_os = "nto",
)))]
fn pipe2(flags: nix::fcntl::OFlag) -> nix::Result<(RawFd, RawFd)> {
    let pipe = unistd::pipe2(flags)?;
    Ok((pipe.0.into_raw_fd(), pipe.1.into_raw_fd()))
}

#[inline]
pub unsafe fn init_pipe() -> Result<(), Error> {
    use nix::fcntl;
    PIPE = pipe2(fcntl::OFlag::O_CLOEXEC)?;
    if let Err(e) = fcntl::fcntl(PIPE.1, fcntl::FcntlArg::F_SETFL(fcntl::OFlag::O_NONBLOCK)) {
        cleanup_pipe();
        return Err(e);
    }
    Ok(())
}

#[inline]
pub unsafe fn cleanup_pipe() -> () {
    // Try to close the pipes. close() should not fail,
    // but if it does, there isn't much we can do
    let _ = unistd::close(PIPE.1);
    let _ = unistd::close(PIPE.0);
    PIPE = (-1, -1);
}

/// Blocks until a Ctrl-C signal is received.
///
/// Must be called after calling [`init_os_handler()`](fn.init_os_handler.html).
///
/// # Errors
/// Will return an error if a system error occurred.
///
#[inline]
pub unsafe fn block_ctrl_c() -> Result<(), CtrlcError> {
    use std::io;
    let mut buf = [0u8];

    loop {
        match unistd::read(PIPE.0, &mut buf[..]) {
            Ok(1) => break,
            Ok(_) => return Err(CtrlcError::System(io::ErrorKind::UnexpectedEof.into())),
            Err(nix::errno::Errno::EINTR) => {}
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}
