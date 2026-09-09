mod common;
#[allow(unused)]
mod runtime;

use std::time::{Duration, Instant};

use common::simple::{Sleep, SlowRandomNumber};
use runtime::simple_noobkio::Noobkio;

fn main() {
    let mut runtime = Noobkio::new();

    let mut handle_1 = runtime.spawn(Sleep::new(Duration::from_secs(1)));
    let mut handle_2 = runtime.spawn(Sleep::new(Duration::from_secs(1)));

    let start = Instant::now();
    runtime.execute();
    let elapsed = start.elapsed();

    let result_1 = handle_1.try_join().unwrap();
    let result_2 = handle_2.try_join().unwrap();

    println!("{}s later..", elapsed.as_secs_f32());
    println!("We got {result_1:?} from 1!");
    println!("We got {result_2:?} from 2!");
}
