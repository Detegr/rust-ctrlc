use crate::error::Error;
use crate::signal::SignalType;
use crate::signalmap::SIGNALS;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use self::unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows::*;

/// Builder for `Channel` allowing to specify more than one signal.
/// # Example
/// ```no_run
/// use ctrlc::{Channel, Signal, SignalType};
///
/// fn main() {
///     let channel = Channel::new_with_multiple()
///         .add_signal(SignalType::Ctrlc)
///         .add_signal(SignalType::Other(
///             #[cfg(unix)] { Signal::SIGTERM },
///             #[cfg(windows)] { Signal::CTRL_BREAK_EVENT },
///         ))
///         .build()
///         .unwrap();
///     println!("Waiting for signal...");
///     channel.recv().unwrap();
///     println!("Got it! Exiting...");
/// }
/// ```
pub struct ChannelBuilder {
    signals: Vec<SignalType>,
}
impl ChannelBuilder {
    /// Build a `Channel` from this channel builder.
    /// # Errors
    /// Possible errors are documented in the documentation of [`Channel`](struct.Channel.html#errors).
    pub fn build(self) -> Result<Channel, Error> {
        Channel::new_from_multiple(&self.signals[..])
    }
    /// Adds a signal to this channel builder.
    pub fn add_signal(mut self, signal: SignalType) -> ChannelBuilder {
        self.signals.push(signal);
        self
    }
}

/// Channel abstraction for signals.
pub struct Channel {
    inner: ChannelType,
    _prevent_sync: *const (),
}

impl Channel {
    /// Creates a new channel. [SignalType](enum.SignalType.html) defines the signal you want the
    /// channel to be used with.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ctrlc::{Channel, SignalType};
    ///
    /// fn main() {
    ///     let channel = Channel::new(SignalType::Ctrlc).unwrap();
    ///     println!("Waiting for Ctrl-C...");
    ///     channel.recv().unwrap();
    ///     println!("Got it! Exiting...");
    /// }
    /// ```
    ///
    /// # Errors
    /// Errors if the signal specified in `SignalType::Other` is not available in the system.
    ///
    /// Errors if there already exists a Channel for the signal.
    ///
    /// On *nix systems an error is returned if the system already has a non-default signal handler for
    /// the registered signal.
    pub fn new(signal: SignalType) -> Result<Channel, Error> {
        Channel::new_from_multiple(&[signal])
    }

    /// Returns a `ChannelBuilder` that can be used to instantiate
    /// a `Channel` with multiple signals.
    pub fn new_with_multiple() -> ChannelBuilder {
        ChannelBuilder { signals: vec![] }
    }

    #[inline]
    fn new_from_multiple(signals: &[SignalType]) -> Result<Channel, Error> {
        for signal in signals {
            let platform_signal: crate::platform::Signal = (*signal).into();
            if !SIGNALS.signals.iter().any(|&s| platform_signal == s) {
                return Err(Error::NoSuchSignal(platform_signal));
            }
        }

        Ok(Channel {
            inner: ChannelType::new(signals.iter().map(|sig| (*sig).into()))?,
            _prevent_sync: ::std::ptr::null(),
        })
    }

    /// Waits for the signal handler to fire while blocking the current thread.
    #[inline]
    pub fn recv(&self) -> Result<SignalType, Error> {
        self.inner.recv()
    }

    /// Tries to receive a signal from the channel without blocking the current thread.
    /// If no signal has been received since last calling `recv` or `try_recv`, returns
    /// an error `Error::ChannelEmpty`.
    #[inline]
    pub fn try_recv(&self) -> Result<SignalType, Error> {
        self.inner.try_recv()
    }
}

unsafe impl Send for Channel {}

// When negative trait bounds are stabilized, this can be used
// instead of _prevent_sync field.
//unsafe impl !Sync for Channel {}
