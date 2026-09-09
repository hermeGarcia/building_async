mod common;
mod runtime;

use std::time::Duration;

use crate::runtime::full_noobkio::{Noobkio, Sleep};

async fn say_hello() -> String {
    let mut buffer = String::from("My String says: ");
    let writer = &mut buffer;

    Sleep::new(Duration::from_millis(100)).await;

    writer.push_str("Hello");

    Sleep::new(Duration::from_millis(100)).await;

    writer.push_str("World");

    buffer
}

fn main() {
    let mut runtime = Noobkio::new();
    let handle = runtime.spawn(say_hello());

    runtime.execute();

    let result = handle.rcv.unwrap().try_recv().unwrap();

    println!("We got: {result}");
}
