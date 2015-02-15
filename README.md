# Rust-CtrlC
Very simple wrapper around `signal` function to set a handler for SIGINT (^C).

## Example usage
```rust
extern crate "rust-ctrlc" as ctrlc;
use ctrlc::CtrlC;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    CtrlC::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    });
    while running.load(Ordering::SeqCst) {}
    println!("Exiting...");
}
```

## TODO:
* Do not allow set_handler to be called multiple times
* Windows support
