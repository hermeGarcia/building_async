mod common;

use std::time::{Duration, Instant};

use crate::common::simple::{MyFuture, MyPoll, Sleep};

pub struct Noobkio {
    futures: Vec<Box<dyn MyFuture + 'static>>,
}

impl Noobkio {
    fn new() -> Self {
        Noobkio {
            futures: Vec::new(),
        }
    }

    fn spawn(&mut self, future: impl MyFuture + 'static) {
        self.futures.push(Box::new(future));
    }

    fn execute(&mut self) {
        while !self.futures.is_empty() {
            self.futures
                .retain_mut(|future| future.as_mut().poll() == MyPoll::Pending);
        }
    }
}

fn main() {
    let mut runtime = Noobkio::new();

    runtime.spawn(Sleep::new(Duration::from_secs(1)));

    let start = Instant::now();
    runtime.execute();
    let elapsed = start.elapsed();

    println!("{}s later..", elapsed.as_secs());
}
