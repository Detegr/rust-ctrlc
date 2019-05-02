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

use platform::{SignalEmitter, UNINITIALIZED_SIGNAL_EMITTER};
use std::cell::UnsafeCell;
use std::sync::atomic::AtomicUsize;

pub struct SignalMap<T> {
    pub signals: Box<[T]>,
    pub counters: Box<[AtomicUsize]>,
    pub emitters: Box<[UnsafeCell<SignalEmitter>]>,
    #[cfg(windows)]
    pub initialized: Box<[AtomicBool]>,
}
unsafe impl<T> Sync for SignalMap<T> {}
impl<T> SignalMap<T>
where
    T: PartialEq,
{
    pub fn get_signal(&self, emitter: &SignalEmitter) -> Option<&T> {
        self.signals
            .iter()
            .zip(self.emitters.iter())
            .find(|&(_, e)| e.get() as *const SignalEmitter == emitter)
            .map(|sigmap| sigmap.0)
    }
    pub fn get_counter(&self, signal: &T) -> Option<&AtomicUsize> {
        self.signals
            .iter()
            .zip(self.counters.iter())
            .find(|&(sig, _)| sig == signal)
            .map(|sigmap| sigmap.1)
    }
    pub fn get_emitter_mut(&self, signal: &T) -> Option<&mut SignalEmitter> {
        self.signals
            .iter()
            .zip(self.emitters.iter())
            .find(|&(sig, _)| sig == signal)
            .map(|sigmap| unsafe { &mut *sigmap.1.get() })
    }
    pub fn get_emitter(&self, signal: &T) -> Option<&SignalEmitter> {
        self.signals
            .iter()
            .zip(self.emitters.iter())
            .find(|&(sig, _)| sig == signal)
            .map(|sigmap| unsafe { &*sigmap.1.get() })
    }
    pub fn has_emitter(&self, signal: &T) -> bool {
        match self.get_emitter(signal) {
            Some(emitter) => *emitter != UNINITIALIZED_SIGNAL_EMITTER,
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
        let emitters = signals
            .clone()
            .into_iter()
            .map(|_| UnsafeCell::new(UNINITIALIZED_SIGNAL_EMITTER))
            .collect::<Vec<_>>();
        #[cfg(unix)]
        {
            SignalMap {
                signals: signals.into_boxed_slice(),
                counters: counters.into_boxed_slice(),
                emitters: emitters.into_boxed_slice(),
            }
        }

        #[cfg(windows)]
        {
            let initialized = signals
                .clone()
                .into_iter()
                .map(|_| AtomicBool::new(false))
                .collect::<Vec<_>>();
            SignalMap {
                signals: signals.into_boxed_slice(),
                counters: counters.into_boxed_slice(),
                emitters: emitters.into_boxed_slice(),
                initialized: initialized.into_boxed_slice(),
            }
        }
    };
}
