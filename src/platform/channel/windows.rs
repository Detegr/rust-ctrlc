use byteorder::{ByteOrder, LittleEndian};
use error::Error;
use platform::winapi::um::consoleapi::SetConsoleCtrlHandler;
use platform::winapi::um::handleapi::INVALID_HANDLE_VALUE;
use platform::winapi::um::namedpipeapi::CreatePipe;
use platform::windows::winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
use platform::windows::winapi::shared::minwindef::{LPCVOID, LPVOID};
use platform::windows::winapi::um::fileapi::{ReadFile, WriteFile};
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
    platform_signal: DWORD,
}

impl WindowsChannel {
    pub fn new(signal: SignalType) -> Result<WindowsChannel, Error> {
        let platform_signal = signal.into();
        let sig_index = SIGNALS
            .index_of(&platform_signal)
            .expect("Validity of signal is checked earlier");
        let initialized = &SIGNALS.initialized[sig_index];
        if initialized.compare_and_swap(false, true, Ordering::AcqRel) {
            return Err(Error::MultipleHandlers);
        }
        unsafe {
            if !SIGNALS.has_pipe_handles(&platform_signal) {
                let mut pipe = (INVALID_HANDLE_VALUE, INVALID_HANDLE_VALUE);
                if CreatePipe(&mut pipe.0, &mut pipe.1, ptr::null_mut(), 0) == FALSE {
                    let e = io::Error::last_os_error();
                    return Err(e.into());
                }

                let pipes = SIGNALS.get_pipe_handles_mut(&platform_signal).unwrap();
                pipes.0 = pipe.0;
                pipes.1 = pipe.1;
            }
            if SetConsoleCtrlHandler(Some(os_handler), TRUE) == FALSE {
                return Err(io::Error::last_os_error().into());
            }
        }
        Ok(WindowsChannel { platform_signal })
    }

    pub fn recv(&self) -> Result<SignalType, Error> {
        let pipes = match SIGNALS.get_pipe_handles(&self.platform_signal) {
            None => {
                return Err(Error::NoSuchSignal(self.platform_signal.into()));
            }
            Some(pipes) => pipes,
        };
        let mut buf = [0u8; 4];
        let mut bytes_to_read = 4;
        let mut bytes_read = 0;
        let mut total_bytes = 0;
        loop {
            match unsafe {
                ReadFile(
                    pipes.0,
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
                        if event == self.platform_signal {
                            return Ok(event.into());
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl Drop for WindowsChannel {
    /// Dropping the channel unregisters the signal handler attached to the channel.
    fn drop(&mut self) {
        let sig_index = SIGNALS
            .index_of(&self.platform_signal)
            .expect("Validity of signal is checked earlier");
        let initialized = &SIGNALS.initialized[sig_index];
        if unsafe { SetConsoleCtrlHandler(Some(os_handler), FALSE) } == FALSE {
            unreachable!("Should not fail");
        }
        initialized.compare_and_swap(true, false, Ordering::AcqRel);
    }
}
