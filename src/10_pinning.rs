mod common;
mod runtime;

use std::time::Duration;

use common::simple::{MyFuture, MyPoll, Sleep};
use runtime::simple_noobkio::Noobkio;

#[derive(Default)]
pub struct Stack {
    buffer: Option<String>,
    writer: Option<*mut String>,
}

#[derive(Default)]
enum State {
    #[default]
    S0,
    Wait1(Sleep),
    S1,
    Wait2(Sleep),
    S2,
}

#[derive(Default)]
struct Writer {
    stack: Stack,
    state: State,
}
impl Writer {
    pub fn new() -> Writer {
        Default::default()
    }
}

impl MyFuture for Writer {
    fn poll(&mut self) -> MyPoll {
        match &mut self.state {
            State::S0 => {
                self.stack.buffer = Some(String::from("My String says: "));
                self.stack.writer = Some(self.stack.buffer.as_mut().unwrap());
                self.state = State::Wait1(Sleep::new(Duration::from_millis(100)));
                MyPoll::Pending
            }
            State::Wait1(sleep) => match sleep.poll() {
                MyPoll::Pending => MyPoll::Pending,
                MyPoll::Ready(_) => {
                    self.state = State::S1;
                    MyPoll::Pending
                }
            },
            State::S1 => {
                let string = unsafe { &mut *self.stack.writer.take().unwrap() };
                string.push_str("hello");

                self.stack.writer = Some(string);
                self.state = State::Wait2(Sleep::new(Duration::from_millis(100)));
                MyPoll::Pending
            }
            State::Wait2(sleep) => match sleep.poll() {
                MyPoll::Pending => MyPoll::Pending,
                MyPoll::Ready(_) => {
                    self.state = State::S2;
                    MyPoll::Pending
                }
            },
            State::S2 => {
                let string = unsafe { &mut *self.stack.writer.take().unwrap() };
                string.push_str(" world");
                MyPoll::Ready(self.stack.buffer.take().unwrap())
            }
        }
    }
}

fn main() {
    let mut runtime = Noobkio::new();

    let mut handle = runtime.spawn(Box::new(Writer::new()));

    runtime.execute();

    let result = handle.try_join().unwrap();
    println!("{result}");
}
