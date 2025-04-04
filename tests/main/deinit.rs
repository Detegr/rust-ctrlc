// Copyright (c) 2023 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

#[macro_use]
mod harness;
use harness::{platform, run_harness};

use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use std::time::Duration;

fn interrupt_and_wait() {
    unsafe { platform::raise_ctrl_c(); }
    std::thread::sleep(Duration::from_millis(10));
}

fn test_deinit() {
    let total_fires = Arc::new(AtomicUsize::new(0));
    let fires = 0;

    // 1. First handler
    let handle = ctrlc::set_handler_once({
        let total_fires = total_fires.clone();
        move || {
            total_fires.fetch_add(1, Ordering::Relaxed);
            fires + 1
        }
    }).unwrap();
    interrupt_and_wait();

    // First unwrap for thread join, second one to `Option` because handler can be removed without firing.
    let fires = handle.join().unwrap().unwrap(); 
    assert_eq!(fires, 1);
    assert_eq!(total_fires.load(Ordering::Relaxed), 1);

    assert!(ctrlc::remove_all_handlers().is_err()); // This handler should be already removed after firing once.

    // 2. Second handler
    let handle = ctrlc::set_handler_once(|| 42).unwrap();
    interrupt_and_wait();

    assert_eq!(handle.join().unwrap().unwrap(), 42);
    assert_eq!(total_fires.load(Ordering::Relaxed), 1);

    assert!(ctrlc::remove_all_handlers().is_err()); // This handler should be already removed after firing once.

    // 3. Test with a non-once handler
    ctrlc::set_handler({
        let total_fires = total_fires.clone();
        move || {
            total_fires.fetch_add(1, Ordering::Relaxed);
        }
    }).unwrap();
    interrupt_and_wait();
    interrupt_and_wait();
    interrupt_and_wait();
    interrupt_and_wait();
    interrupt_and_wait();

    assert_eq!(total_fires.load(Ordering::Relaxed), 6);
    
    ctrlc::remove_all_handlers().unwrap();

    // 4. First handler again
    let handle = ctrlc::set_handler_once({
        let total_fires = total_fires.clone();
        move || {
            total_fires.fetch_add(1, Ordering::Relaxed);
            fires + 1
        }
    }).unwrap();
    interrupt_and_wait();

    let fires = handle.join().unwrap().unwrap();

    assert_eq!(fires, 2);
    assert_eq!(total_fires.load(Ordering::Relaxed), 7);

    assert!(ctrlc::remove_all_handlers().is_err()); // This handler should be already removed after firing once.
}

fn tests() {
    run_tests!(test_deinit);
}

fn main() {
    run_harness(tests);
}
