// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use crate::error::Error as CtrlcError;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Platform specific error type
pub type Error = std::io::Error;

/// Platform specific signal type
#[derive(Debug)]
pub struct Signal {
}

#[derive(Debug, Default)]
pub struct WaitForCtrlC {
}

impl Future
for WaitForCtrlC {
    type Output = Result<(), CtrlcError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

/// Register os signal handler.
///
/// Must be called before calling [`block_ctrl_c()`](fn.block_ctrl_c.html)
/// and should only be called once.
///
/// # Errors
/// Will return an error if a system error occurred.
///
#[allow(dead_code)]
#[inline]
pub fn init_os_handler() -> Result<WaitForCtrlC, Error>
{
    let ret = WaitForCtrlC::default();
    Ok(ret)
}

