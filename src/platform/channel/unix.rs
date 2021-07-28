// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use self::nix::sys::select::{select, FdSet};
use self::nix::sys::signal as nix_signal;
use self::nix::sys::time::{TimeVal, TimeValLike};
use self::nix::unistd;
use crate::error::Error;
use crate::platform::unix::nix::sys::signal::Signal;
use crate::platform::unix::{nix, utils};
use crate::signal::SignalType;
use crate::signalevent::SignalEvent;
use crate::signalmap::SIGNALS;
use byteorder::{ByteOrder, LittleEndian};
use std::convert::TryFrom;
use std::io;

pub type ChannelType = UnixChannel;

pub struct UnixChannel {
    platform_signals: Box<[nix_signal::Signal]>,
}
impl UnixChannel {
    extern "C" fn os_handler(signum: nix::libc::c_int) {
        let signal = Signal::try_from(signum).ok();
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
                    let pipe = utils::pipe2(fcntl::OFlag::O_CLOEXEC)?;
                    let close_pipe = |e: nix::Error| -> Error {
                        let _ = unistd::close(pipe.1);
                        let _ = unistd::close(pipe.0);
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
    fn num_of_ready_fds(mut read_set: FdSet, wait: bool) -> Result<(i32, FdSet), Error> {
        let mut zero = TimeVal::zero();
        loop {
            let timeout = if wait { Some(&mut zero) } else { None };
            match select(None, Some(&mut read_set), None, None, timeout) {
                Ok(fds) => return Ok((fds, read_set)),
                Err(nix::Error::Sys(nix::errno::Errno::EINTR)) => {
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
    fn recv_inner(&self, wait: bool) -> Result<SignalType, Error> {
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
        let (num_of_ready_fds, mut read_set) = UnixChannel::num_of_ready_fds(read_set, wait)?;
        if num_of_ready_fds != 0 {
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
                                    let signal = nix_signal::Signal::try_from(signum)?;
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
