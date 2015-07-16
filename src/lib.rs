#![feature(static_condvar)]
#![feature(static_mutex)]

extern crate libc;

use std::sync::{StaticCondvar, CONDVAR_INIT, StaticMutex, MUTEX_INIT};
static CVAR: StaticCondvar = CONDVAR_INIT;
static MUTEX: StaticMutex = MUTEX_INIT;

#[cfg(unix)]
mod platform {
    use libc::c_int;
    use libc::types::os::common::posix01::sighandler_t;
    use libc::consts::os::posix88::SIGINT;
    use libc::funcs::posix01::signal::signal;

    #[repr(C)]
    pub fn handler(_: c_int) {
        super::CVAR.notify_all();
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
        super::CVAR.notify_all();
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
    pub fn set_handler<F: Fn() -> () + 'static + Send>(user_handler: F) -> () {
        unsafe {
            set_os_handler(handler);
        }
        std::thread::spawn(move || {
            loop {
                let _ = CVAR.wait(MUTEX.lock().unwrap());
                user_handler();
            }
        });
    }
}
