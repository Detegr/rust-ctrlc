use crate::signalevent::SignalEvent;
use error::Error;
use platform;
use platform::winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
use platform::winapi::shared::winerror::WAIT_TIMEOUT;
use platform::winapi::um::consoleapi::SetConsoleCtrlHandler;
use platform::winapi::um::handleapi::CloseHandle;
use platform::winapi::um::synchapi::WaitForMultipleObjects;
use platform::winapi::um::winbase::{CreateSemaphoreA, INFINITE, WAIT_FAILED, WAIT_OBJECT_0};
use platform::winapi::um::winnt::MAXIMUM_WAIT_OBJECTS;
use platform::Signal;
use signalmap::SIGNALS;
use std::io;
use std::ptr;
use std::sync::atomic::Ordering;
use SignalType;

pub type ChannelType = WindowsChannel;

unsafe extern "system" fn os_handler(event: DWORD) -> BOOL {
    let emitter = SIGNALS.get_emitter(&event);
    if let Some(emitter) = emitter {
        emitter.emit(&event);
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
            // TODO
            return Err(Error::MultipleHandlers);
        }
        for platform_signal in signals.iter() {
            let sig_index = SIGNALS
                .index_of(platform_signal)
                .expect("Validity of signal is checked earlier");
            let initialized = &SIGNALS.initialized[sig_index];
            if initialized.compare_and_swap(false, true, Ordering::AcqRel) {
                return Err(Error::MultipleHandlers);
            }
            unsafe {
                if !SIGNALS.has_emitter(platform_signal) {
                    let sem =
                        CreateSemaphoreA(ptr::null_mut(), 0, platform::MAX_SEM_COUNT, ptr::null());
                    if sem.is_null() {
                        let e = io::Error::last_os_error();
                        return Err(e.into());
                    }

                    let mut emitter = SIGNALS.get_emitter_mut(platform_signal).unwrap();
                    *emitter = sem;
                }
                if SetConsoleCtrlHandler(Some(os_handler), TRUE) == FALSE {
                    return Err(io::Error::last_os_error().into());
                }
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
            match SIGNALS.get_emitter(sig) {
                None => {
                    return Err(Error::NoSuchSignal((*sig).into()));
                }
                Some(event) => event_handles.push(event),
            }
        }
        let num_of_handles = event_handles.len() as DWORD; // Only MAXIMUM_WAIT_OBJECTS (64) handles are supported, so this fits to u32
        let wait_time = if wait { INFINITE } else { 0 };
        let i = unsafe {
            WaitForMultipleObjects(num_of_handles, *event_handles.as_ptr(), FALSE, wait_time)
        };
        let some_ready = i < (WAIT_OBJECT_0 + num_of_handles);
        if some_ready {
            SIGNALS
                .get_signal(event_handles[i as usize])
                .map(|sig| (*sig).into())
                .ok_or_else(|| Error::NoSuchSignal(i.into()))
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
            let sig_index = SIGNALS
                .index_of(sig)
                .expect("Validity of signal is checked earlier");
            let initialized = &SIGNALS.initialized[sig_index];
            for emitter in SIGNALS.get_emitter_mut(&sig) {
                unsafe {
                    if CloseHandle(*emitter) == FALSE {
                        unreachable!("Should not fail");
                    }
                }
                *emitter = platform::UNINITIALIZED_SIGNAL_EMITTER;
            }
            if unsafe { SetConsoleCtrlHandler(Some(os_handler), FALSE) } == FALSE {
                unreachable!("Should not fail");
            }
            initialized.compare_and_swap(true, false, Ordering::AcqRel);
        }
    }
}
