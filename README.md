# CtrlC
[![Build Status](https://travis-ci.org/Detegr/rust-ctrlc.svg?branch=master)](https://travis-ci.org/Detegr/rust-ctrlc)

A simple easy to use wrapper around Ctrl-C signal.

## Dependencies
* [libc](https://crates.io/crates/libc)
* [lazy_static](https://crates.io/crates/lazy_static) (only with stable and beta compilers)

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

#### Try the example yourself
`cargo run --example readme_example`

## Building
If you're using a nightly compiler, I suggest building with `cargo build --features nightly` to avoid the dependency to lazy_static. On stable and beta compilers `cargo build` will do.
