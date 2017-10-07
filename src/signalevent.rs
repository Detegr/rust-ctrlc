use crate::platform::Signal;

pub trait SignalEvent {
    fn emit(&self, sig: &Signal);
}
