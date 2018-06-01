// Copyright (c) 2018 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

#[cfg(windows)]
use std::sync::atomic::AtomicBool;

use platform::PipeHandle;
use std::cell::UnsafeCell;
use std::sync::atomic::AtomicUsize;

pub struct SignalMap<T> {
    pub signals: Box<[T]>,
    pub counters: Box<[AtomicUsize]>,
    pub pipes: Box<[UnsafeCell<(PipeHandle, PipeHandle)>]>,
    #[cfg(windows)]
    pub initialized: Box<[AtomicBool]>,
}
unsafe impl<T> Sync for SignalMap<T> {}
impl<T> SignalMap<T>
where
    T: PartialEq,
{
    pub fn get_counter(&self, signal: &T) -> Option<&AtomicUsize> {
        self.signals
            .iter()
            .zip(self.counters.iter())
            .find(|&(sig, _)| sig == signal)
            .map(|sigmap| sigmap.1)
    }
    pub fn get_pipe_handles_mut(&self, signal: &T) -> Option<&mut (PipeHandle, PipeHandle)> {
        self.signals
            .iter()
            .zip(self.pipes.iter())
            .find(|&(sig, _)| sig == signal)
            .map(|sigmap| unsafe { &mut *sigmap.1.get() })
    }
    pub fn get_pipe_handles(&self, signal: &T) -> Option<&(PipeHandle, PipeHandle)> {
        self.signals
            .iter()
            .zip(self.pipes.iter())
            .find(|&(sig, _)| sig == signal)
            .map(|sigmap| unsafe { &*sigmap.1.get() })
    }
    pub fn has_pipe_handles(&self, signal: &T) -> bool {
        match self.get_pipe_handles(signal) {
            Some(pipes) => !(pipes.0 == -1 && pipes.1 == -1),
            None => false,
        }
    }
    #[cfg(windows)]
    pub fn index_of(&self, signal: &T) -> Option<usize> {
        self.signals
            .iter()
            .enumerate()
            .find(|&(_, sig)| sig == signal)
            .map(|s| s.0)
    }
}

lazy_static! {
    pub static ref SIGNALS: SignalMap<::platform::Signal> = {
        let signals = ::platform::signal_iterator().collect::<Vec<_>>();
        let counters = signals
            .clone()
            .into_iter()
            .map(|_| AtomicUsize::new(0))
            .collect::<Vec<_>>();
        #[cfg(unix)]
        {
            let pipes = signals
                .clone()
                .into_iter()
                .map(|_| UnsafeCell::new((-1, -1)))
                .collect::<Vec<_>>();

            SignalMap {
                signals: signals.into_boxed_slice(),
                counters: counters.into_boxed_slice(),
                pipes: pipes.into_boxed_slice(),
            }
        }

        #[cfg(windows)]
        {
            use std::ptr;

            let pipes = signals
                .clone()
                .into_iter()
                .map(|_| UnsafeCell::new((ptr::null_mut(), ptr::null_mut())))
                .collect::<Vec<_>>();
            let initialized = signals
                .clone()
                .into_iter()
                .map(|_| AtomicBool::new(false))
                .collect::<Vec<_>>();
            SignalMap {
                signals: signals.into_boxed_slice(),
                counters: counters.into_boxed_slice(),
                pipes: pipes.into_boxed_slice(),
                initialized: initialized.into_boxed_slice(),
            }
        }
    };
}
