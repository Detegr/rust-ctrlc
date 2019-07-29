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
use crate::signalevent::SignalEvent;
use byteorder::{ByteOrder, LittleEndian};
use error::Error;
use platform::unix::nix;
use platform::unix::nix::sys::signal::Signal;
use signal::SignalType;
use signalmap::SIGNALS;

pub type ChannelType = UnixChannel;

pub struct UnixChannel {
    platform_signals: Box<[nix_signal::Signal]>,
}
impl UnixChannel {
    extern "C" fn os_handler(signum: nix::libc::c_int) {
        let signal = Signal::from_c_int(signum).ok();
        if let Some(signal) = signal {
            let pipes = SIGNALS.get_emitter(&signal);
            if let Some(pipes) = pipes {
                pipes.emit(&signal);
            }
        }
    }
    pub fn new(platform_signals: impl Iterator<Item = Signal>) -> Result<UnixChannel, Error> {
        use self::nix::fcntl;
        use self::nix::sys::signal;

        let signals = platform_signals.collect::<Vec<_>>();
        for platform_signal in signals.iter() {
            unsafe {
                if !SIGNALS.has_emitter(&platform_signal) {
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

                    let pipes = SIGNALS.get_emitter_mut(&platform_signal).unwrap();
                    pipes.0 = pipe.0;
                    pipes.1 = pipe.1;
                }

                let handler = signal::SigHandler::Handler(UnixChannel::os_handler);
                let new_action = signal::SigAction::new(
                    handler,
                    signal::SaFlags::SA_RESTART,
                    signal::SigSet::empty(),
                );

                match signal::sigaction(*platform_signal, &new_action) {
                    Ok(old) => {
                        if old.handler() != nix_signal::SigHandler::SigDfl {
                            return Err(Error::MultipleHandlers);
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
        Ok(UnixChannel {
            platform_signals: signals.into_boxed_slice(),
        })
    }
    fn recv_inner(&self, wait: bool) -> Result<SignalType, Error> {
        use self::nix::sys::select::{select, FdSet};
        use self::nix::sys::time::{TimeVal, TimeValLike};
        use std::io;
        let mut read_set = FdSet::new();
        let mut pipe_handles = vec![];
        for sig in self.platform_signals.iter() {
            match SIGNALS.get_emitter(sig) {
                None => {
                    return Err(Error::NoSuchSignal((*sig).into()));
                }
                Some(pipe) => pipe_handles.push(pipe.0),
            }
        }
        for handle in pipe_handles.iter() {
            read_set.insert(*handle);
        }
        let mut buf = [0u8; 4];
        let mut total_bytes = 0;
        let ready_ids_result = if wait {
            let mut timeout = TimeVal::zero();
            select(None, Some(&mut read_set), None, None, Some(&mut timeout))
        } else {
            select(None, Some(&mut read_set), None, None, None)
        };
        let some_ready = match ready_ids_result {
            Ok(num) => num != 0,
            Err(nix::Error::Sys(_)) => true,
            Err(e) => Err(e)?,
        };

        if some_ready {
            for handle in pipe_handles.iter() {
                if read_set.contains(*handle) {
                    loop {
                        match unistd::read(*handle, &mut buf[total_bytes..]) {
                            Ok(i) if i <= 4 => {
                                total_bytes += i;
                                if total_bytes < 4 {
                                    continue;
                                } else {
                                    total_bytes = 0;
                                    let signum = LittleEndian::read_i32(&buf);
                                    let signal = nix_signal::Signal::from_c_int(signum)?;
                                    for &sig in self.platform_signals.iter() {
                                        if signal == sig {
                                            return Ok(signal.into());
                                        }
                                    }
                                }
                                continue;
                            }
                            Ok(_) => {
                                return Err(Error::System(io::ErrorKind::UnexpectedEof.into()))
                            }
                            Err(nix::Error::Sys(nix::errno::Errno::EINTR)) => {}
                            Err(e) => return Err(e.into()),
                        }
                    }
                }
            }
        }
        Err(Error::ChannelEmpty)
    }
    pub fn recv(&self) -> Result<SignalType, Error> {
        self.recv_inner(false)
    }
    pub fn try_recv(&self) -> Result<SignalType, Error> {
        self.recv_inner(true)
    }
}

impl Drop for UnixChannel {
    /// Dropping the channel unregisters the signal handlers attached to the channel.
    fn drop(&mut self) {
        let new_action = nix_signal::SigAction::new(
            nix_signal::SigHandler::SigDfl,
            nix_signal::SaFlags::empty(),
            nix_signal::SigSet::empty(),
        );
        for sig in self.platform_signals.iter() {
            let _old = unsafe { nix_signal::sigaction(*sig, &new_action) };
        }
    }
}
