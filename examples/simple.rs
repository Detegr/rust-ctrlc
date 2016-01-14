// Copyright (c) 2015 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

// Warning: If you run this example with `cargo run --example simple`
// and hit Ctrl-C, the executable will still be kept running using
// 100% of CPU. This is because cargo seems to kill the process even
// though you have set up Ctrl-C handler of your own.

extern crate ctrlc;
use ctrlc::CtrlC;
fn main() {
    CtrlC::set_handler(|| println!("Hello world!"));
    loop {}
}
