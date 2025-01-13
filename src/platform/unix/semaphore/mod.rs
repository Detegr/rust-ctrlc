use crate::error::Error as CtrlcError;
use libc::{sem_destroy, sem_init, sem_post, sem_t, sem_wait};
use std::mem::MaybeUninit;

static mut SEM: MaybeUninit<sem_t> = MaybeUninit::uninit();

pub type Error = nix::Error;

#[inline]
pub unsafe fn init_sem() -> Result<(), Error> {
    match sem_init(SEM.as_mut_ptr(), 0, 0) {
        0 => Ok(()),
        _ => Err(Error::last()),
    }
}

#[inline]
pub unsafe fn cleanup_sem() {
    let _ = sem_destroy(SEM.as_mut_ptr());
}

#[inline]
unsafe fn wait_sem() -> Result<(), Error> {
    match sem_wait(SEM.as_mut_ptr()) {
        0 => Ok(()),
        _ => Err(Error::last()),
    }
}

#[inline]
unsafe fn post_sem() -> Result<(), Error> {
    match sem_post(SEM.as_mut_ptr()) {
        0 => Ok(()),
        _ => Err(Error::last()),
    }
}

#[inline]
pub unsafe fn os_handler_sem() {
    // Assuming this always succeeds. Can't really handle errors in any meaningful way.
    let _ = post_sem();
}

#[inline]
pub unsafe fn block_ctrl_c() -> Result<(), CtrlcError> {
    loop {
        match wait_sem() {
            Ok(()) => break,
            Err(nix::errno::Errno::EINTR) => {}
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}
