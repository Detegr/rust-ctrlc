//! A simple easy to use wrapper around Ctrl-C signal.

#![cfg_attr(feature="nightly", feature(static_condvar))]
#![cfg_attr(feature="nightly", feature(static_mutex))]

extern crate libc;
#[cfg(feature="stable")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature="nightly")]
mod features {
    use std::sync::{StaticCondvar, CONDVAR_INIT, StaticMutex, MUTEX_INIT};
    pub static CVAR: StaticCondvar = CONDVAR_INIT;
    pub static MUTEX: StaticMutex = MUTEX_INIT;
}
#[cfg(not(feature="nightly"))]
mod features {
    use std::sync::{Condvar, Mutex};
    lazy_static! {
        pub static ref CVAR: Condvar = Condvar::new();
        pub static ref MUTEX: Mutex<bool> = Mutex::new(false);
    }
}
use self::features::*;

#[cfg(unix)]
mod platform {
    use libc::c_int;
    use libc::types::os::common::posix01::sighandler_t;
    use libc::consts::os::posix88::SIGINT;
    use libc::funcs::posix01::signal::signal;

    #[repr(C)]
    pub fn handler(_: c_int) {
        super::features::CVAR.notify_all();
    }
    #[inline]
    pub unsafe fn set_os_handler(handler: fn(c_int)) {
        signal(SIGINT, ::std::mem::transmute::<_, sighandler_t>(handler));
    }
}
#[cfg(windows)]
mod platform {
    use libc::c_int;
    type PHandlerRoutine = unsafe extern fn(CtrlType: c_int) -> bool;

    #[link(name = "kernel32")]
    extern {
        fn SetConsoleCtrlHandler(HandlerRoutine: PHandlerRoutine, Add: bool) -> bool;
    }

    #[repr(C)]
    pub fn handler(_: c_int) -> bool {
        super::features::CVAR.notify_all();
        true
    }
    #[inline]
    pub unsafe fn set_os_handler(handler: fn(c_int) -> bool) {
        SetConsoleCtrlHandler(::std::mem::transmute::<_, PHandlerRoutine>(handler), true);
    }
}
use self::platform::*;

pub struct CtrlC;
impl CtrlC {
    /// Sets up the signal handler for Ctrl-C
    /// # Example
    /// ```
    /// use ctrlc::CtrlC;
    /// CtrlC::set_handler(|| println!("Hello world!"));
    /// ```
    pub fn set_handler<F: Fn() -> () + 'static + Send>(user_handler: F) -> () {
        unsafe {
            set_os_handler(handler);
        }
        ::std::thread::spawn(move || {
            loop {
                let _ = CVAR.wait(MUTEX.lock().unwrap());
                user_handler();
            }
        });
    }
}
