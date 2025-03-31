// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::io;
use std::io::ErrorKind;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HANDLE, WAIT_FAILED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
use windows_sys::Win32::System::Threading::{
    CreateSemaphoreA, ReleaseSemaphore, WaitForSingleObject, INFINITE,
};

use crate::block_outcome::BlockOutcome;

/// Platform specific error type
pub type Error = io::Error;

/// Platform specific signal type
pub type Signal = u32;

const MAX_SEM_COUNT: i32 = 65535;
const TRUE: BOOL = 1;
const FALSE: BOOL = 0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OsHandler {
    semaphore: HANDLE,
}

static mut HANDLER: Option<OsHandler> = None;

unsafe extern "system" fn os_handler(_: u32) -> BOOL {
    if let Some(handler) = HANDLER {
        // Assuming this always succeeds. Can't really handle errors in any meaningful way.
        ReleaseSemaphore(handler.semaphore, 1, ptr::null_mut());
        TRUE
    } else {
        // We have no handler set. Not sure how the hell this function was even called then.
        // But okay, just mark this as not handled (FALSE).
        FALSE
    }
}

/// Register OS signal handler.
///
/// Must be called before calling [`block_ctrl_c()`](fn.block_ctrl_c.html)
/// and should only be called once.
///
/// # Errors
/// Will return an error if a system error occurred.
#[inline]
pub unsafe fn init_os_handler(overwrite: bool) -> Result<(), Error> {
    if is_handler_init() {
        if !overwrite {
            return Err(ErrorKind::AlreadyExists.into())
        } else {
            deinit_os_handler()?;
        }
    }
    assert!(!is_handler_init());

    let semaphore = CreateSemaphoreA(ptr::null_mut(), 0, MAX_SEM_COUNT, ptr::null());
    if semaphore.is_null() {
        return Err(io::Error::last_os_error());
    }

    
    // Remove OUR handlers if those exist
    // It does not make sense to have multiple of same(!) OS handlers added.
    let mut handlers_removed = 0;
    while SetConsoleCtrlHandler(Some(os_handler), FALSE) == TRUE {
        handlers_removed += 1;
    }
    if handlers_removed > 0 {
        // This does not interfere with our ability to add handlers, but it is unexpected for there
        // to be more than one.
        eprintln!(
            "[ctrlc] Somehow {handlers_removed} other OS {} of `ctrlc` was added before. Probably a bug.", 
            if handlers_removed == 1 { "handler" } else { "handlers" }
        );
    }

    // Set our custom handler
    if SetConsoleCtrlHandler(Some(os_handler), TRUE) == FALSE {
        let e = io::Error::last_os_error();
        CloseHandle(semaphore);
        return Err(e);
    }

    HANDLER = Some(OsHandler { semaphore });

    Ok(())
}

/// Unregisters OS signal handler set by [`ctrlc::platform::init_os_handler`].
#[inline]
pub unsafe fn deinit_os_handler() -> Result<(), Error> {
    if let Some(handler) = HANDLER {
        HANDLER = None;
        CloseHandle(handler.semaphore);
        SetConsoleCtrlHandler(Some(os_handler), FALSE); // Remove the handler callbalk
        Ok(())
    } else {
        Err(ErrorKind::NotFound.into())
    }
}

pub unsafe fn is_handler_init() -> bool {
    #[allow(static_mut_refs)]
    return HANDLER.is_some();
}


/// Blocks until a Ctrl-C signal is received.
///
/// Must be called after calling [`init_os_handler()`](fn.init_os_handler.html).
///
/// # Errors
/// Will return an error if a system error occurred.
///
#[inline]
pub unsafe fn block_ctrl_c() -> Result<BlockOutcome, Error> {
    let handler = HANDLER.ok_or::<Error>(ErrorKind::NotFound.into())?;

    match WaitForSingleObject(handler.semaphore, INFINITE) {
        WAIT_OBJECT_0 => Ok(BlockOutcome::Awaited),
        WAIT_FAILED => {
            if Some(handler) != HANDLER {
                Ok(BlockOutcome::HandlerRemoved)
            } else {
                Err(io::Error::last_os_error())
            }
        },
        ret => Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "WaitForSingleObject(), unexpected return value \"{:x}\"",
                ret
            ),
        )),
    }
}
