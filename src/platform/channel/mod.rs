#[cfg(unix)]
mod unix;
#[cfg(unix)]
use self::unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows::*;

use error::Error;
use signal::SignalType;

/// Channel abstraction for signals
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
    /// extern crate ctrlc;
    /// use std::thread;
    /// use std::time;
    ///
    /// fn main() {
    ///     let channel = ctrlc::Channel::new(ctrlc::SignalType::Ctrlc).unwrap();
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
    #[inline]
    pub fn new(signal: SignalType) -> Result<Channel, Error> {
        Ok(Channel {
            inner: ChannelType::new(signal)?,
            _prevent_sync: ::std::ptr::null(),
        })
    }

    /// Waits for the signal handler to fire while blocking the current thread.
    #[inline]
    pub fn recv(&self) -> Result<SignalType, Error> {
        self.inner.recv()
    }
}

unsafe impl Send for Channel {}

// When negative trait bounds are stabilized, this can be used
// instead of _prevent_sync field.
//unsafe impl !Sync for Channel {}