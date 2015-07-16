extern crate ctrlc;
use ctrlc::CtrlC;
fn main() {
    CtrlC::set_handler(|| println!("Hello world!"));
    loop {}
}
