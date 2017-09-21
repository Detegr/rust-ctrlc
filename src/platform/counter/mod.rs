use error::Error;
use signal::SignalType;
use signalmap::SIGNALS;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use self::unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows::*;

/// Counter abstraction for signals
pub struct Counter {
    signal: ::platform::Signal,
}
impl Counter {
    /// Creates a new counter. [SignalType](enum.SignalType.html) defines the signal you want the
    /// counter to count.
    ///
    /// # Example
    ///
    /// ```no_run
    /// extern crate ctrlc;
    /// use std::thread;
    /// use std::time;
    ///
    /// fn main() {
    ///     let counter = ctrlc::Counter::new(ctrlc::SignalType::Ctrlc).unwrap();
    ///     println!("Waiting for Ctrl-C...");
    ///     while counter.get().unwrap() == 0 {
    ///         thread::sleep(time::Duration::from_millis(10));
    ///     }
    ///     println!("Got it! Exiting...");
    /// }
    /// ```
    ///
    /// # Errors
    /// Errors if the signal specified in `SignalType::Other` is not available in the system.
    ///
    /// On *nix systems an error is returned if the system already has a non-default signal handler for
    /// the registered signal.
    /// On Windows systems an error is returned if there already exists a Counter for the signal.
    /// This is not necessary as per the OS, but is implemented to keep the functionality similar
    /// between the OSes.
    pub fn new(signal: SignalType) -> Result<Counter, Error> {
        let platform_signal = signal.to_platform_signal();

        if !SIGNALS.signals.iter().any(|&s| platform_signal == s) {
            return Err(Error::NoSuchSignal(signal));
        }

        set_handler(platform_signal)?;

        Ok(Counter {
            signal: platform_signal,
        })
    }

    /// Gets the value of the counter using an atomic operation.
    ///
    /// # Note
    /// The value returned may not be the value of the counter anymore.
    /// This function accesses the counter atomically, but loads the value into normal `usize`
    /// variable, so the counter may or may not have changed during the time this function returns
    ///
    /// # Errors
    /// Returns `None` if the signal specified in `SignalType::Other` is not available in the
    /// system.
    pub fn get(&self) -> Option<usize> {
        use std::sync::atomic::Ordering;
        SIGNALS
            .get_counter(&self.signal)
            .and_then(|counter| Some(counter.load(Ordering::Acquire)))
    }
}
impl Drop for Counter {
    /// Dropping the counter unregisters the signal handler attached to the counter.
    fn drop(&mut self) {
        reset_handler(self.signal);
    }
}
