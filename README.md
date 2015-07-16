# CtrlC
[![Build Status](https://travis-ci.org/Detegr/rust-ctrlc.svg?branch=master)](https://travis-ci.org/Detegr/rust-ctrlc)

A simple easy to use wrapper around Ctrl-C signal.

## Example usage
```rust
extern crate ctrlc;
use ctrlc::CtrlC;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    CtrlC::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    });
	println!("Waiting for Ctrl-C...");
    while running.load(Ordering::SeqCst) {}
    println!("Got it! Exiting...");
}
```

### Try the example yourself
`cargo run --example readme_example`
