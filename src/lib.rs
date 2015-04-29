#![feature(libc)]
#![feature(std_misc)]

extern crate libc;
use libc::types::os::common::posix01::sighandler_t;
use libc::consts::os::posix88::SIGINT;
use libc::funcs::posix01::signal::signal;
use libc::c_int;
use std::sync::{StaticCondvar, CONDVAR_INIT, StaticMutex, MUTEX_INIT};

static CVAR: StaticCondvar = CONDVAR_INIT;
static MUTEX: StaticMutex = MUTEX_INIT;

#[repr(C)]
fn handler(_: c_int) {
    CVAR.notify_all();
}

#[allow(missing_copy_implementations)]
pub struct CtrlC;
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
