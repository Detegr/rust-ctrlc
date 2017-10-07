use platform::Signal;

pub trait SignalEvent {
    fn emit(&self, &Signal);
}
