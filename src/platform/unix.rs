// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

pub use nix;

use self::nix::sys::signal::Signal as nix_signal;
use self::nix::unistd;
use crate::signal::SignalType;
use crate::signalevent::SignalEvent;
use byteorder::{ByteOrder, LittleEndian};
use std::os::unix::io::RawFd;

/// Platform specific error type
pub type Error = nix::Error;

/// Platform specific signal type
pub type Signal = nix::sys::signal::Signal;

/// TODO Platform specific pipe handle type
pub type SignalEmitter = (RawFd, RawFd);
impl SignalEvent for SignalEmitter {
    fn emit(&self, signal: &Signal) {
        let mut buf = [0u8; 4];
        LittleEndian::write_i32(&mut buf[..], *signal as i32);
        // Assuming this always succeeds. Can't really handle errors in any meaningful way.
        let _ = unistd::write(self.1, &buf);
    }
}

pub const CTRL_C_SIGNAL: Signal = nix_signal::SIGINT;
pub const UNINITIALIZED_SIGNAL_EMITTER: (RawFd, RawFd) = (-1, -1);

/// Iterator returning available signals on this system
pub fn signal_iterator() -> nix::sys::signal::SignalIterator {
    Signal::iterator()
}

impl SignalType {
    /// Get the underlying platform specific signal
    pub fn to_platform_signal(&self) -> Signal {
        match *self {
            SignalType::Ctrlc => nix_signal::SIGINT,
            SignalType::Other(s) => s,
        }
    }
}

pub mod utils {
    use super::{unistd, RawFd};

    // pipe2(2) is not available on macOS or iOS, so we need to use pipe(2) and fcntl(2)
    #[inline]
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    pub fn pipe2(flags: nix::fcntl::OFlag) -> nix::Result<(RawFd, RawFd)> {
        use nix::fcntl::{fcntl, FcntlArg, FdFlag, OFlag};

        let pipe = unistd::pipe()?;

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
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    pub fn pipe2(flags: nix::fcntl::OFlag) -> nix::Result<(RawFd, RawFd)> {
        unistd::pipe2(flags)
    }
}
