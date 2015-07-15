#![feature(std_misc)]
#![feature(static_condvar)]
#![feature(static_mutex)]

extern crate libc;

#[cfg(unix)]
use libc::types::os::common::posix01::sighandler_t;
#[cfg(unix)]
use libc::consts::os::posix88::SIGINT;
#[cfg(unix)]
use libc::funcs::posix01::signal::signal;

use libc::c_int;

use std::sync::{StaticCondvar, CONDVAR_INIT, StaticMutex, MUTEX_INIT};

static CVAR: StaticCondvar = CONDVAR_INIT;
static MUTEX: StaticMutex = MUTEX_INIT;

#[cfg(unix)]
#[repr(C)]
fn handler(_: c_int) {
    CVAR.notify_all();
}

#[cfg(windows)]
#[repr(C)]
fn handler(_: c_int) -> bool {
    CVAR.notify_all();
    true
}

#[allow(missing_copy_implementations)]
pub struct CtrlC;

#[cfg(unix)]
impl CtrlC {
    pub fn set_handler<F: Fn() -> () + 'static + Send>(user_handler: F) -> () {
        unsafe {
            signal(SIGINT, std::mem::transmute::<_, sighandler_t>(handler));
        }
        std::thread::spawn(move || {
            loop {
                let _ = CVAR.wait(MUTEX.lock().unwrap());
                user_handler();
            }
        });
    }
}

#[cfg(windows)]
type PHandlerRoutine = unsafe extern fn(CtrlType: c_int) -> bool;

#[cfg(windows)]
#[link(name = "kernel32")]
extern {
	fn SetConsoleCtrlHandler(HandlerRoutine: PHandlerRoutine, Add: bool) -> bool;
}

#[cfg(windows)]
impl CtrlC {
    pub fn set_handler<F: Fn() -> () + 'static + Send>(user_handler: F) -> () {
        unsafe {
            SetConsoleCtrlHandler(std::mem::transmute::<_, PHandlerRoutine>(handler), true);
        }
        std::thread::spawn(move || {
            loop {
                let _ = CVAR.wait(MUTEX.lock().unwrap());
                user_handler();
            }
        });
    }
}
