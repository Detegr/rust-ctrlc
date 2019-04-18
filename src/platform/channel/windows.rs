use byteorder::{ByteOrder, LittleEndian};
use error::Error;
use platform::winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
use platform::winapi::shared::minwindef::{LPCVOID, LPVOID};
use platform::winapi::shared::winerror::WAIT_TIMEOUT;
use platform::winapi::um::consoleapi::SetConsoleCtrlHandler;
use platform::winapi::um::fileapi::{ReadFile, WriteFile};
use platform::winapi::um::handleapi::INVALID_HANDLE_VALUE;
use platform::winapi::um::namedpipeapi::CreatePipe;
use platform::winapi::um::synchapi::WaitForMultipleObjects;
use platform::winapi::um::winbase::{INFINITE, WAIT_FAILED, WAIT_OBJECT_0};
use platform::winapi::um::winnt::MAXIMUM_WAIT_OBJECTS;
use platform::Signal;
use signalmap::SIGNALS;
use std::io;
use std::ptr;
use std::sync::atomic::Ordering;
use SignalType;

pub type ChannelType = WindowsChannel;

unsafe extern "system" fn os_handler(event: DWORD) -> BOOL {
    let pipes = SIGNALS.get_pipe_handles(&event);
    if let Some(pipes) = pipes {
        let mut buf = [0u8; 4];
        LittleEndian::write_u32(&mut buf[..], event);
        // Assuming this always succeeds. Can't really handle errors in any meaningful way.
        WriteFile(
            pipes.1,
            buf.as_ptr() as LPCVOID,
            4,
            ptr::null_mut(),
            ptr::null_mut(),
        );
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
                if !SIGNALS.has_pipe_handles(platform_signal) {
                    let mut pipe = (INVALID_HANDLE_VALUE, INVALID_HANDLE_VALUE);
                    if CreatePipe(&mut pipe.0, &mut pipe.1, ptr::null_mut(), 0) == FALSE {
                        let e = io::Error::last_os_error();
                        return Err(e.into());
                    }

                    let pipes = SIGNALS.get_pipe_handles_mut(platform_signal).unwrap();
                    pipes.0 = pipe.0;
                    pipes.1 = pipe.1;
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
        let mut pipe_handles = vec![];
        for sig in self.platform_signals.iter() {
            match SIGNALS.get_pipe_handles(sig) {
                None => {
                    return Err(Error::NoSuchSignal((*sig).into()));
                }
                Some(pipe) => pipe_handles.push(pipe.0),
            }
        }
        let num_of_handles = pipe_handles.len() as DWORD; // Only MAXIMUM_WAIT_OBJECTS (64) handles are supported, so this fits to u32
        let wait_time = if wait { INFINITE } else { 0 };
        let i = unsafe {
            WaitForMultipleObjects(num_of_handles, pipe_handles.as_ptr(), FALSE, wait_time)
        };
        let some_ready = i < (WAIT_OBJECT_0 + num_of_handles);
        if some_ready {
            let pipe_handle = pipe_handles[i as usize];
            let mut buf = [0u8; 4];
            let mut bytes_to_read = 4;
            let mut bytes_read = 0;
            let mut total_bytes = 0;
            loop {
                match unsafe {
                    ReadFile(
                        pipe_handle,
                        buf.as_mut_ptr() as LPVOID,
                        bytes_to_read,
                        &mut bytes_read,
                        ptr::null_mut(),
                    )
                } {
                    TRUE => {
                        total_bytes += bytes_read;
                        if total_bytes < 4 {
                            bytes_to_read -= bytes_read;
                            continue;
                        } else {
                            total_bytes = 0;
                            let event = LittleEndian::read_u32(&buf);
                            for &sig in self.platform_signals.iter() {
                                if event == sig {
                                    return Ok(event.into());
                                }
                            }
                        }
                    }
                    _ => {
                        let e = io::Error::last_os_error();
                        return Err(e.into());
                    }
                }
            }
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
    /// Dropping the channel unregisters the signal handler attached to the channel.
    fn drop(&mut self) {
        for sig in self.platform_signals.iter() {
            let sig_index = SIGNALS
                .index_of(sig)
                .expect("Validity of signal is checked earlier");
            let initialized = &SIGNALS.initialized[sig_index];
            if unsafe { SetConsoleCtrlHandler(Some(os_handler), FALSE) } == FALSE {
                unreachable!("Should not fail");
            }
            initialized.compare_and_swap(true, false, Ordering::AcqRel);
        }
    }
}
