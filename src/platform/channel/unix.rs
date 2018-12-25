// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use self::nix::sys::signal as nix_signal;
use self::nix::unistd;
use byteorder::{ByteOrder, LittleEndian};
use error::Error;
use platform::unix::nix;
use platform::unix::nix::sys::signal::Signal;
use signal::SignalType;
use signalmap::SIGNALS;

pub type ChannelType = UnixChannel;

pub struct UnixChannel {
    platform_signal: nix_signal::Signal,
}
impl UnixChannel {
    extern "C" fn os_handler(signum: nix::libc::c_int) {
        let pipes = Signal::from_c_int(signum)
            .ok()
            .and_then(|signal| SIGNALS.get_pipe_handles(&signal));
        if let Some(pipes) = pipes {
            let mut buf = [0u8; 4];
            LittleEndian::write_i32(&mut buf[..], signum);
            // Assuming this always succeeds. Can't really handle errors in any meaningful way.
            unistd::write(pipes.1, &buf).is_ok();
        }
    }
    pub fn new(signal: SignalType) -> Result<UnixChannel, Error> {
        use self::nix::fcntl;
        use self::nix::sys::signal;

        let platform_signal = signal.into();

        unsafe {
            if !SIGNALS.has_pipe_handles(&platform_signal) {
                let pipe = unistd::pipe2(fcntl::OFlag::O_CLOEXEC)?;
                let close_pipe = |e: nix::Error| -> Error {
                    unistd::close(pipe.1).is_ok();
                    unistd::close(pipe.0).is_ok();
                    e.into()
                };

                // Make sure we never block on write in the os handler.
                if let Err(e) =
                    fcntl::fcntl(pipe.1, fcntl::FcntlArg::F_SETFL(fcntl::OFlag::O_NONBLOCK))
                {
                    return Err(close_pipe(e));
                }

                let pipes = SIGNALS.get_pipe_handles_mut(&platform_signal).unwrap();
                pipes.0 = pipe.0;
                pipes.1 = pipe.1;
            }

            let handler = signal::SigHandler::Handler(UnixChannel::os_handler);
            let new_action = signal::SigAction::new(
                handler,
                signal::SaFlags::SA_RESTART,
                signal::SigSet::empty(),
            );

            match signal::sigaction(platform_signal, &new_action) {
                Ok(old) => {
                    if old.handler() != nix_signal::SigHandler::SigDfl {
                        return Err(Error::MultipleHandlers);
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(UnixChannel { platform_signal })
    }
    pub fn recv(&self) -> Result<SignalType, Error> {
        let pipe_handles = match SIGNALS.get_pipe_handles(&self.platform_signal) {
            None => {
                return Err(Error::NoSuchSignal(self.platform_signal.into()));
            }
            Some(pipe) => pipe,
        };
        use std::io;
        let mut buf = [0u8; 4];
        let mut total_bytes = 0;
        loop {
            match unistd::read(pipe_handles.0, &mut buf[total_bytes..]) {
                Ok(i) if i <= 4 => {
                    total_bytes += i;
                    if total_bytes < 4 {
                        continue;
                    } else {
                        total_bytes = 0;
                        let signum = LittleEndian::read_i32(&buf);
                        let signal = nix_signal::Signal::from_c_int(signum)?;
                        if signal == self.platform_signal {
                            return Ok(signal.into());
                        }
                    }
                    continue;
                }
                Ok(_) => return Err(Error::System(io::ErrorKind::UnexpectedEof.into())),
                Err(nix::Error::Sys(nix::errno::Errno::EINTR)) => {}
                Err(e) => return Err(e.into()),
            }
        }
    }
}

impl Drop for UnixChannel {
    /// Dropping the channel unregisters the signal handler attached to the channel.
    fn drop(&mut self) {
        let new_action = nix_signal::SigAction::new(
            nix_signal::SigHandler::SigDfl,
            nix_signal::SaFlags::empty(),
            nix_signal::SigSet::empty(),
        );
        let _old = unsafe { nix_signal::sigaction(self.platform_signal, &new_action) };
    }
}
