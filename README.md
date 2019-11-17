# CtrlC
[![Build Status](https://travis-ci.org/Detegr/rust-ctrlc.svg?branch=master)](https://travis-ci.org/Detegr/rust-ctrlc)
[![Build status](https://ci.appveyor.com/api/projects/status/kwg1uu2w2aqn9ta9/branch/master?svg=true)](https://ci.appveyor.com/project/Detegr/rust-ctrlc/branch/master)

A simple easy to use wrapper around Ctrl-C signal.

[Documentation](http://detegr.github.io/doc/ctrlc/)

# Example usage
## Counter example
```rust
extern crate ctrlc;
use ctrlc::{Counter, SignalType};
use std::thread;
use std::time;

fn main() {
    let counter = Counter::new(SignalType::Ctrlc).unwrap();
    println!("Waiting for Ctrl-C...");
    while counter.get() == 0 {
        thread::sleep(time::Duration::from_millis(10));
    }
    println!("Got it! Exiting...");
}
```
## Channel example

```rust
extern crate ctrlc;
use ctrlc::{Channel, SignalType};

fn main() {
    let channel = Channel::new(SignalType::Ctrlc).unwrap();
    println!("Waiting for Ctrl-C...");
    channel.recv().unwrap();
    println!("Got it! Exiting...");
}
```

#### Try the examples yourself
`cargo run --example counter_example`
`cargo run --example channel_example`

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
