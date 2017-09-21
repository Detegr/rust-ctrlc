// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

extern crate ctrlc;
use std::thread;
use std::time;

fn main() {
    let counter = ctrlc::Counter::new(ctrlc::SignalType::Ctrlc).unwrap();
    println!("Waiting for Ctrl-C...");
    while counter.get().unwrap() == 0 {
        thread::sleep(time::Duration::from_millis(10));
    }
    println!("Got it! Exiting...");
}
