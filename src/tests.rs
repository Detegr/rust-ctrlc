// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

extern crate ctrlc;

#[cfg(unix)]
mod platform {
    extern crate nix;

    use std::io;

    pub unsafe fn setup() -> io::Result<()> {
        Ok(())
    }

    pub unsafe fn cleanup() -> io::Result<()> {
        Ok(())
    }

    pub unsafe fn raise_ctrl_c() {
        self::nix::sys::signal::raise(self::nix::sys::signal::SIGINT).unwrap();
    }

    pub unsafe fn raise_termination() {
        self::nix::sys::signal::raise(self::nix::sys::signal::SIGTERM).unwrap();
    }

    pub unsafe fn print(fmt: ::std::fmt::Arguments) {
        use self::io::Write;
        let stdout = ::std::io::stdout();
        stdout.lock().write_fmt(fmt).unwrap();
    }
}

#[cfg(windows)]
mod platform {
    extern crate winapi;

    use self::winapi::shared::minwindef::DWORD;
    use self::winapi::shared::ntdef::{CHAR, HANDLE};
    use self::winapi::um::consoleapi::{AllocConsole, GetConsoleMode};
    use self::winapi::um::fileapi::WriteFile;
    use self::winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use self::winapi::um::processenv::{GetStdHandle, SetStdHandle};
    use self::winapi::um::winbase::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};
    use self::winapi::um::wincon::{AttachConsole, FreeConsole, GenerateConsoleCtrlEvent};
    use std::io;
    use std::ptr;

    /// Stores a piped stdout handle or a cache that gets
    /// flushed when we reattached to the old console.
    enum Output {
        Pipe(HANDLE),
        Cached(Vec<u8>),
    }

    static mut OLD_OUT: *mut Output = 0 as *mut Output;

    impl io::Write for Output {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            match *self {
                Output::Pipe(handle) => unsafe {
                    use self::winapi::shared::ntdef::VOID;

                    let mut n = 0u32;
                    if WriteFile(
                        handle,
                        buf.as_ptr() as *const VOID,
                        buf.len() as DWORD,
                        &mut n as *mut DWORD,
                        ptr::null_mut(),
                    ) == 0
                    {
                        Err(io::Error::last_os_error())
                    } else {
                        Ok(n as usize)
                    }
                },
                Output::Cached(ref mut s) => s.write(buf),
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Output {
        /// Stores current piped stdout or creates a new output cache that will
        /// be written to stdout at a later time.
        fn new() -> io::Result<Output> {
            unsafe {
                let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
                if stdout.is_null() || stdout == INVALID_HANDLE_VALUE {
                    return Err(io::Error::last_os_error());
                }

                let mut out = 0u32;
                match GetConsoleMode(stdout, &mut out as *mut DWORD) {
                    0 => Ok(Output::Pipe(stdout)),
                    _ => Ok(Output::Cached(Vec::new())),
                }
            }
        }

        /// Set stdout/stderr and flush cache.
        unsafe fn set_as_std(self) -> io::Result<()> {
            let stdout = match self {
                Output::Pipe(h) => h,
                Output::Cached(_) => get_stdout()?,
            };

            if SetStdHandle(STD_OUTPUT_HANDLE, stdout) == 0 {
                return Err(io::Error::last_os_error());
            }

            if SetStdHandle(STD_ERROR_HANDLE, stdout) == 0 {
                return Err(io::Error::last_os_error());
            }

            match self {
                Output::Pipe(_) => Ok(()),
                Output::Cached(ref s) => {
                    // Write cached output
                    use self::io::Write;
                    let out = io::stdout();
                    out.lock().write_all(&s[..])?;
                    Ok(())
                }
            }
        }
    }

    unsafe fn get_stdout() -> io::Result<HANDLE> {
        use self::winapi::um::fileapi::{CreateFileA, OPEN_EXISTING};
        use self::winapi::um::winnt::{FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE};

        let stdout = CreateFileA(
            "CONOUT$\0".as_ptr() as *const CHAR,
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_WRITE,
            ptr::null_mut(),
            OPEN_EXISTING,
            0,
            ptr::null_mut(),
        );

        if stdout.is_null() || stdout == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(stdout)
        }
    }

    /// Detach from the current console and create a new one,
    /// We do this because GenerateConsoleCtrlEvent() sends ctrl-c events
    /// to all processes on the same console. We want events to be received
    /// only by our process.
    ///
    /// This breaks rust's stdout pre 1.18.0. Rust used to
    /// [cache the std handles](https://github.com/rust-lang/rust/pull/40516)
    ///
    pub unsafe fn setup() -> io::Result<()> {
        let old_out = Output::new()?;

        if FreeConsole() == 0 {
            return Err(io::Error::last_os_error());
        }

        if AllocConsole() == 0 {
            return Err(io::Error::last_os_error());
        }

        // AllocConsole will not always set stdout/stderr to the to the console buffer
        // of the new terminal.

        let stdout = get_stdout()?;
        if SetStdHandle(STD_OUTPUT_HANDLE, stdout) == 0 {
            return Err(io::Error::last_os_error());
        }

        if SetStdHandle(STD_ERROR_HANDLE, stdout) == 0 {
            return Err(io::Error::last_os_error());
        }

        OLD_OUT = Box::into_raw(Box::new(old_out));

        Ok(())
    }

    /// Reattach to the old console.
    pub unsafe fn cleanup() -> io::Result<()> {
        if FreeConsole() == 0 {
            return Err(io::Error::last_os_error());
        }

        if AttachConsole(winapi::um::wincon::ATTACH_PARENT_PROCESS) == 0 {
            return Err(io::Error::last_os_error());
        }

        Box::from_raw(OLD_OUT).set_as_std()?;

        Ok(())
    }

    /// This will signal the whole process group.
    pub unsafe fn raise_ctrl_c() {
        assert!(GenerateConsoleCtrlEvent(winapi::um::wincon::CTRL_C_EVENT, 0) != 0);
    }

    pub unsafe fn raise_termination() {
        assert!(GenerateConsoleCtrlEvent(winapi::um::wincon::CTRL_BREAK_EVENT, 0) != 0);
    }

    /// Print to both consoles, this is not thread safe.
    pub unsafe fn print(fmt: ::std::fmt::Arguments) {
        use self::io::Write;
        {
            let stdout = io::stdout();
            stdout.lock().write_fmt(fmt).unwrap();
        }
        {
            assert!(!OLD_OUT.is_null());
            (*OLD_OUT).write_fmt(fmt).unwrap();
        }
    }
}

fn test_set_handler() {
    let (tx, rx) = ::std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        tx.send(true).unwrap();
    })
    .unwrap();

    unsafe {
        platform::raise_ctrl_c();
    }

    rx.recv_timeout(::std::time::Duration::from_secs(10))
        .unwrap();

    match ctrlc::set_handler(|| {}) {
        Err(ctrlc::Error::MultipleHandlers) => {}
        ret => panic!("{:?}", ret),
    }
}

fn test_set_multiple_handlers() {
    let counter1 = ctrlc::Counter::new(ctrlc::SignalType::Ctrlc);
    let counter2 = ctrlc::Counter::new(ctrlc::SignalType::Ctrlc);
    assert!(counter1.is_ok());
    assert!(counter2.is_err());
    drop(counter1);
    let counter3 = ctrlc::Counter::new(ctrlc::SignalType::Ctrlc);
    assert!(counter3.is_ok());
}

fn test_counter() {
    use ctrlc::Counter;

    fn test_counter_with(counter: Counter, raise_function: unsafe fn()) {
        use std::thread;
        use std::time::Duration;

        let ctrlc_thread = thread::spawn(move || {
            for _ in 0..5 {
                thread::sleep(Duration::from_millis(10));
                unsafe {
                    raise_function();
                }
            }
        });

        loop {
            let val = counter.get().unwrap();
            if val > 4 {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        ctrlc_thread.join().unwrap();

        let counter_value = counter.get().unwrap();
        unsafe {
            raise_function();
        };
        // Wait some time for the signal handler to run
        thread::sleep(Duration::from_millis(100));

        let new_counter_value = counter.get().unwrap();
        assert_eq!(new_counter_value, counter_value + 1);
    }

    let c = Counter::new(ctrlc::SignalType::Ctrlc).unwrap();
    test_counter_with(c, platform::raise_ctrl_c as unsafe fn());

    let c = Counter::new(ctrlc::SignalType::Termination).unwrap();
    test_counter_with(c, platform::raise_termination as unsafe fn());
}

fn test_invalid_counter() {
    use ctrlc::{Counter, Error, Signal, SignalType};
    use std::mem;

    // Create invalid signal
    let invalid_signal: Signal = unsafe { mem::transmute(12345) };

    if let Err(Error::NoSuchSignal(SignalType::Other(sig))) =
        Counter::new(SignalType::Other(invalid_signal))
    {
        assert_eq!(sig, invalid_signal);
    } else {
        assert!(false);
    }
}

fn test_channel() {
    use ctrlc::{Channel, SignalType};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    let flag = Arc::new(AtomicBool::new(false));
    let flag2 = flag.clone();
    let channel = Channel::new(SignalType::Ctrlc).unwrap();
    let termination_channel = Channel::new(SignalType::Termination).unwrap();
    let channel_thread = thread::spawn(move || {
        let sig = channel.recv().expect("Channel should not return error");
        if sig != SignalType::Ctrlc {
            panic!("Invalid signal type received");
        }
        let sig = termination_channel
            .recv()
            .expect("Channel should not return error");
        if sig != SignalType::Termination {
            panic!("Invalid signal type received");
        }
        flag2.store(true, Ordering::Relaxed);
    });
    let raise_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        unsafe {
            platform::raise_ctrl_c();
            platform::raise_termination();
        }
    });

    while !flag.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(1));
    }

    channel_thread.join().unwrap();
    raise_thread.join().unwrap();
}

macro_rules! run_tests {
    ( $($test_fn:ident),* ) => {
        unsafe {
            platform::print(format_args!("\n"));
            $(
                platform::print(format_args!("test tests::{} ... ", stringify!($test_fn)));
                $test_fn();
                platform::print(format_args!("ok\n"));
            )*
            platform::print(format_args!("\n"));
        }
    }
}

fn main() {
    unsafe {
        platform::setup().unwrap();
    }

    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        unsafe {
            platform::cleanup().unwrap();
        }
        (default)(info);
    }));

    run_tests!(test_counter);
    run_tests!(test_invalid_counter);
    run_tests!(test_set_multiple_handlers);
    run_tests!(test_channel);
    run_tests!(test_set_handler);

    unsafe {
        platform::cleanup().unwrap();
    }
}
