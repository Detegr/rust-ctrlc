// Copyright (c) 2015 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use ctrlc;

#[cfg_attr(feature = "tokio", tokio::main(flavor = "current_thread"))]
#[cfg_attr(feature = "async-std", async_std::main())]
async fn main() {
    #[cfg(feature = "tokio")]
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    #[cfg(feature = "async-std")]
    let (tx, rx) = async_std::channel::bounded(1);

    ctrlc::set_async_handler(async move {
            tx.send(()).await.expect("Could not send signal on channel.");
        })
        .expect("Error setting Ctrl-C handler");

    println!("Waiting for Ctrl-C...");
    rx.recv().await.expect("Could not receive from channel.");
    println!("Got it! Exiting...");
}
