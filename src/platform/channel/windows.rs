use SignalType;
use error::Error;

pub type ChannelType = WindowsChannel;

pub struct WindowsChannel;

impl WindowsChannel {
    pub fn new(signal: SignalType) -> Result<WindowsChannel, Error> {
        Ok(WindowsChannel {})
    }

    pub fn recv(&self) -> Result<SignalType, Error> {
        Ok(SignalType::Ctrlc)
    }
}
