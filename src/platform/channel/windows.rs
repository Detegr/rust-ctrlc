use crate::error::Error;
use crate::platform;
use crate::signalevent::SignalEvent;
use crate::signalmap::SIGMAP;
use crate::SignalType;
use platform::winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
use platform::winapi::shared::winerror::WAIT_TIMEOUT;
use platform::winapi::um::consoleapi::SetConsoleCtrlHandler;
use platform::winapi::um::handleapi::CloseHandle;
use platform::winapi::um::synchapi::WaitForMultipleObjects;
use platform::winapi::um::winbase::{CreateSemaphoreA, INFINITE, WAIT_FAILED, WAIT_OBJECT_0};
use platform::winapi::um::winnt::MAXIMUM_WAIT_OBJECTS;
use platform::Signal;
use std::convert::TryFrom;
use std::io;
use std::ptr;
use std::sync::atomic::Ordering;

pub type ChannelType = WindowsChannel;

// SAFETY: FFI
unsafe extern "system" fn os_handler(event: DWORD) -> BOOL {
    if let Ok(signal) = Signal::try_from(event) {
        let emitter = SIGMAP.get_emitter(&signal);
        if let Some(emitter) = emitter {
            emitter.emit(&signal);
        }
    }
    TRUE
}

pub struct WindowsChannel {
    platform_signals: Box<[Signal]>,
}

impl WindowsChannel {
    pub fn new(platform_signals: impl Iterator<Item = Signal>) -> Result<WindowsChannel, Error> {
        let signals = platform_signals.collect::<Vec<_>>();
        if signals.len() > (MAXIMUM_WAIT_OBJECTS as usize) {
            return Err(Error::TooManySignals);
        }
        for platform_signal in signals.iter() {
            let sig_index = SIGMAP
                .index_of(platform_signal)
                .expect("Validity of signal is checked earlier");
            let initialized = &SIGMAP.initialized[sig_index];

            // SAFETY: Atomically set initialized[sig_index] = true before acquiring a mutable
            // reference to the signal emitter data within a UnsafeCell
            if initialized
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                return Err(Error::MultipleHandlers);
            }

            // SAFETY: FFI
            let sem = unsafe {
                CreateSemaphoreA(ptr::null_mut(), 0, platform::MAX_SEM_COUNT, ptr::null())
            };
            if sem.is_null() {
                let e = io::Error::last_os_error();
                return Err(e.into());
            }

            let emitter = SIGMAP.get_emitter_mut(platform_signal).unwrap();
            *emitter = sem;

            // SAFETY: FFI
            if unsafe { SetConsoleCtrlHandler(Some(os_handler), TRUE) } == FALSE {
                return Err(io::Error::last_os_error().into());
            }
        }
        Ok(WindowsChannel {
            platform_signals: signals.into_boxed_slice(),
        })
    }

    pub fn recv(&self) -> Result<SignalType, Error> {
        self.recv_inner(true)
    }

    pub fn try_recv(&self) -> Result<SignalType, Error> {
        self.recv_inner(false)
    }

    fn recv_inner(&self, wait: bool) -> Result<SignalType, Error> {
        let mut event_handles = vec![];
        for sig in self.platform_signals.iter() {
            match SIGMAP.get_emitter(sig) {
                None => {
                    return Err(Error::NoSuchSignal((*sig).into()));
                }
                Some(event) => event_handles.push(event),
            }
        }
        let num_of_handles = event_handles.len() as DWORD; // Only MAXIMUM_WAIT_OBJECTS (64) handles are supported, so this fits to u32
        let wait_time = if wait { INFINITE } else { 0 };

        // SAFETY: FFI
        let i = unsafe {
            WaitForMultipleObjects(num_of_handles, *event_handles.as_ptr(), FALSE, wait_time)
        };
        let some_ready = i < (WAIT_OBJECT_0 + num_of_handles);
        if some_ready {
            SIGMAP
                .get_signal(event_handles[i as usize])
                .map(|sig| (*sig).into())
                .ok_or_else(|| Error::NoSuchSignal(Signal::try_from(i).unwrap()))
        } else if i == WAIT_FAILED {
            let e = io::Error::last_os_error();
            return Err(e.into());
        } else {
            assert_eq!(i, WAIT_TIMEOUT);
            Err(Error::ChannelEmpty)
        }
    }
}

impl Drop for WindowsChannel {
    /// Dropping the channel unregisters the signal handlers attached to the channel.
    fn drop(&mut self) {
        for sig in self.platform_signals.iter() {
            let sig_index = SIGMAP
                .index_of(sig)
                .expect("Validity of signal is checked earlier");
            let initialized = &SIGMAP.initialized[sig_index];
            let emitter = SIGMAP
                .get_emitter(&sig)
                .expect("Emitter for the signal must exist");

            // SAFETY: FFI
            unsafe { CloseHandle(*emitter) };

            // SAFETY: We don't want to hold onto the reference within UnsafeCell while
            // setting the `initialized` value back to false.
            drop(emitter);

            // SAFETY: FFI
            if unsafe { SetConsoleCtrlHandler(Some(os_handler), FALSE) } == FALSE {
                unreachable!("Should not fail");
            }
            let _ = initialized.compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire);
        }
    }
}
