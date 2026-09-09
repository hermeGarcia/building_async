mod common;
mod runtime;

use std::time::{Duration, Instant};

use common::simple::{MyFuture, MyPoll, Sleep, SlowRandomNumber};
use runtime::simple_noobkio::{Handle, Noobkio};

enum MainState {
    State1(Sleep, SlowRandomNumber),
    State2(SlowRandomNumber),
}

pub struct MainFuture {
    inner: Option<MainState>,
}

impl MainFuture {
    pub fn new(sleep: Sleep, other: SlowRandomNumber) -> MainFuture {
        MainFuture {
            inner: Some(MainState::State1(sleep, other)),
        }
    }
}

impl MyFuture for MainFuture {
    fn poll(&mut self) -> MyPoll {
        let Some(inner) = self.inner.take() else {
            return MyPoll::Pending;
        };

        match inner {
            MainState::State1(mut sleep, other) => {
                let MyPoll::Ready(_) = sleep.poll() else {
                    self.inner = Some(MainState::State1(sleep, other));
                    return MyPoll::Pending;
                };

                self.inner = Some(MainState::State2(other));
                MyPoll::Pending
            }

            MainState::State2(mut other) => {
                let MyPoll::Ready(out) = other.poll() else {
                    self.inner = Some(MainState::State2(other));
                    return MyPoll::Pending;
                };

                let out = format!("{{ value: {out} }}");

                MyPoll::Ready(out)
            }
        }
    }
}

fn main() {
    let mut runtime = Noobkio::new();

    let mut main_handle = runtime.spawn(MainFuture::new(
        Sleep::new(Duration::from_secs(1)),
        SlowRandomNumber::from(10),
    ));

    let start = Instant::now();
    runtime.execute();
    let elapsed = start.elapsed();

    let result = main_handle.try_join().unwrap();
    println!("{}s later..", elapsed.as_secs_f32());
    println!("We got {result}!");
}
