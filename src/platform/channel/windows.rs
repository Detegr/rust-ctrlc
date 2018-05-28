use error::Error;
use platform::windows::kernel32::CreatePipe;
use platform::windows::winapi::{BOOL, DWORD, FALSE, PHANDLE, TRUE};
use std::ptr;
use SignalType;

pub type ChannelType = WindowsChannel;

static mut PIPE: (PHANDLE, PHANDLE) = (ptr::null_mut(), ptr::null_mut());

pub struct WindowsChannel {
    platform_signal: DWORD,
}

impl WindowsChannel {
    pub fn new(signal: SignalType) -> Result<WindowsChannel, Error> {
        let platform_signal = signal.into();
        unsafe {
            if CreatePipe(PIPE.0, PIPE.1, ptr::null_mut(), 0) == FALSE {
                let e = io::Error::last_os_error();
                return Err(e.into());
            }
        }
        Ok(WindowsChannel { platform_signal })
    }

    pub fn recv(&self) -> Result<SignalType, Error> {
        Ok(SignalType::Ctrlc)
    }
}
