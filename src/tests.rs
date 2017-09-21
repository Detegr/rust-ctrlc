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
    extern crate kernel32;

    use std::io;
    use std::ptr;
    use self::winapi::winnt::{CHAR, HANDLE};
    use self::winapi::minwindef::DWORD;

    /// Stores a piped stdout handle or a cache that gets flushed when we reattached to the old console.
    enum Output {
        Pipe(HANDLE),
        Cached(Vec<u8>),
    }

    static mut OLD_OUT: *mut Output = 0 as *mut Output;

    impl io::Write for Output {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            match *self {
                Output::Pipe(handle) => unsafe {
                    use self::winapi::winnt::VOID;

                    let mut n = 0u32;
                    if self::kernel32::WriteFile(
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
            use self::winapi::shlobj::INVALID_HANDLE_VALUE;

            unsafe {
                let stdout = self::kernel32::GetStdHandle(winapi::STD_OUTPUT_HANDLE);
                if stdout.is_null() || stdout == INVALID_HANDLE_VALUE {
                    return Err(io::Error::last_os_error());
                }

                let mut out = 0u32;
                match self::kernel32::GetConsoleMode(stdout, &mut out as *mut DWORD) {
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

            if self::kernel32::SetStdHandle(winapi::STD_OUTPUT_HANDLE, stdout) == 0 {
                return Err(io::Error::last_os_error());
            }

            if self::kernel32::SetStdHandle(winapi::STD_ERROR_HANDLE, stdout) == 0 {
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
        use self::winapi::winnt::{GENERIC_READ, GENERIC_WRITE, FILE_SHARE_WRITE};
        use self::winapi::shlobj::INVALID_HANDLE_VALUE;
        use self::winapi::fileapi::OPEN_EXISTING;

        let stdout = self::kernel32::CreateFileA(
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

        if self::kernel32::FreeConsole() == 0 {
            return Err(io::Error::last_os_error());
        }

        if self::kernel32::AllocConsole() == 0 {
            return Err(io::Error::last_os_error());
        }

        // AllocConsole will not always set stdout/stderr to the to the console buffer
        // of the new terminal.

        let stdout = get_stdout()?;
        if self::kernel32::SetStdHandle(winapi::STD_OUTPUT_HANDLE, stdout) == 0 {
            return Err(io::Error::last_os_error());
        }

        if self::kernel32::SetStdHandle(winapi::STD_ERROR_HANDLE, stdout) == 0 {
            return Err(io::Error::last_os_error());
        }

        OLD_OUT = Box::into_raw(Box::new(old_out));

        Ok(())
    }

    /// Reattach to the old console.
    pub unsafe fn cleanup() -> io::Result<()> {
        if self::kernel32::FreeConsole() == 0 {
            return Err(io::Error::last_os_error());
        }

        if self::kernel32::AttachConsole(winapi::wincon::ATTACH_PARENT_PROCESS) == 0 {
            return Err(io::Error::last_os_error());
        }

        Box::from_raw(OLD_OUT).set_as_std()?;

        Ok(())
    }

    /// This will signal the whole process group.
    pub unsafe fn raise_ctrl_c() {
        assert!(self::kernel32::GenerateConsoleCtrlEvent(self::winapi::CTRL_C_EVENT, 0) != 0);
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
    ctrlc::set_handler(move || { tx.send(true).unwrap(); }).unwrap();

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

fn test_counter() {
    use ctrlc::Counter;

    fn test_counter_with(counter: &Counter, raise_function: &'static unsafe fn()) {
        use std::thread;
        use std::time::Duration;

        let ctrlc_thread = thread::spawn(move || for _ in 0..5 {
            thread::sleep(Duration::from_millis(10));
            unsafe {
                raise_function();
            }
        });

        loop {
            let val = counter.get().unwrap();
            if val > 0 {
                if val > 4 {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        ctrlc_thread.join().unwrap();

        let counter_value = counter.get().unwrap();
        unsafe {
            raise_function();
        };
        let new_counter_value = counter.get().unwrap();
        assert!(counter_value + 1 == new_counter_value);
    }

    let counter = Counter::new(ctrlc::SignalType::Ctrlc).unwrap();
    test_counter_with(&counter, &(platform::raise_ctrl_c as unsafe fn()));
    let counter = Counter::new(ctrlc::SignalType::Termination).unwrap();
    test_counter_with(&counter, &(platform::raise_termination as unsafe fn()));
}

fn test_invalid_counter() {
    use std::mem;
    use ctrlc::{Counter, Error, Signal, SignalType};

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
    run_tests!(test_set_handler);

    unsafe {
        platform::cleanup().unwrap();
    }
}
