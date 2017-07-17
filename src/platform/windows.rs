// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

extern crate winapi;
extern crate kernel32;

use super::Error;
use self::winapi::{HANDLE, BOOL, DWORD, TRUE, FALSE, c_long};
use std::ptr;
use std::io;

const MAX_SEM_COUNT: c_long = 255;
static mut SEMAPHORE: HANDLE = 0 as HANDLE;

unsafe extern "system" fn os_handler(_: DWORD) -> BOOL {
    // Assuming this always succeeds. Can't really handle errors in any meaningful way.
    kernel32::ReleaseSemaphore(SEMAPHORE, 1, ptr::null_mut());
    TRUE
}

/// Register os signal handler.
///
/// Must be called before calling [`block_ctrl_c()`](fn.block_ctrl_c.html)
/// and should only be called once.
///
/// # Errors
/// Will return an error if a system error occurred.
///
#[inline]
pub unsafe fn init_os_handler() -> Result<(), Error> {
    SEMAPHORE = kernel32::CreateSemaphoreA(ptr::null_mut(), 0, MAX_SEM_COUNT, ptr::null());
    if SEMAPHORE.is_null() {
        return Err(Error::System(io::Error::last_os_error()));
    }

    if kernel32::SetConsoleCtrlHandler(Some(os_handler), TRUE) == FALSE {
        let e = io::Error::last_os_error();
        kernel32::CloseHandle(SEMAPHORE);
        SEMAPHORE = 0 as HANDLE;
        return Err(Error::System(e));
    }

    Ok(())
}

/// Blocks until a Ctrl-C signal is received.
///
/// Must be called after calling [`init_os_handler()`](fn.init_os_handler.html).
///
/// # Errors
/// Will return an error if a system error occurred.
///
#[inline]
pub unsafe fn block_ctrl_c() -> Result<(), Error> {
    use self::winapi::{INFINITE, WAIT_OBJECT_0, WAIT_FAILED};

    match kernel32::WaitForSingleObject(SEMAPHORE, INFINITE) {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_FAILED => Err(Error::System(io::Error::last_os_error())),
        ret => {
            Err(Error::System(io::Error::new(io::ErrorKind::Other,
                                             format!("WaitForSingleObject(), unexpected return value \"{:x}\"",
                                                     ret))))
        }
    }
}
