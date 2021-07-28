use ctrlc;
use ctrlc::{Channel, Signal, SignalType};

fn main() {
    let channel = Channel::new_with_multiple()
        .add_signal(SignalType::Ctrlc)
        .add_signal(SignalType::Other(
            #[cfg(unix)] { Signal::SIGTERM },
            #[cfg(windows)] { Signal::CTRL_BREAK_EVENT },
        ))
        .build()
        .unwrap();
    println!("Waiting for signal...");
    channel.recv().unwrap();
    println!("Got it! Exiting...");
}
