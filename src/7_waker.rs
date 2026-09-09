mod common;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::task::{Wake, Waker};
use std::time::{Duration, Instant};

use common::simple::{MyPoll, Sleep};

pub trait MyFuture {
    fn poll(&mut self, waker: &Waker) -> MyPoll;
}

struct NoobkioWaker {
    id: u64,
    sender: SyncSender<u64>,
}

impl Wake for NoobkioWaker {
    fn wake(self: Arc<Self>) {
        self.sender.send(self.id).expect("executor dropped");
    }
}

struct Noobkio {
    fresh_id: u64,
    future_poll: HashMap<u64, Box<dyn MyFuture + 'static>>,
    sender: SyncSender<u64>,
    ready_queue: Receiver<u64>,
}

impl Noobkio {
    fn new() -> Noobkio {
        let (sender, receiver) = mpsc::sync_channel(100);
        Noobkio {
            sender,
            fresh_id: 0,
            future_poll: HashMap::new(),
            ready_queue: receiver,
        }
    }

    fn next_id(&mut self) -> u64 {
        let fresh = self.fresh_id;
        self.fresh_id += 1;

        fresh
    }

    fn spawn<F: MyFuture + 'static>(&mut self, future: F) {
        let id = self.next_id();
        self.future_poll.insert(id, Box::new(future));
        self.sender.send(id).expect("executor dropped");
    }

    fn execute(&mut self) {
        while let Ok(future_id) = self.ready_queue.recv() {
            let Some(mut future) = self.future_poll.remove(&future_id) else {
                continue;
            };

            let waker = Waker::from(Arc::new(NoobkioWaker {
                id: future_id,
                sender: self.sender.clone(),
            }));

            match future.as_mut().poll(&waker) {
                MyPoll::Pending => {
                    self.future_poll.insert(future_id, future);
                }

                MyPoll::Ready(_out) => {
                    if self.future_poll.is_empty() {
                        break;
                    }
                }
            }
        }
    }
}

impl MyFuture for Sleep {
    fn poll(&mut self, waker: &Waker) -> MyPoll {
        let now = self.now.get_or_insert_with(Instant::now);
        let elapsed = now.elapsed();

        if elapsed < self.wait_for {
            waker.wake_by_ref();
            MyPoll::Pending
        } else {
            MyPoll::Ready(String::with_capacity(0))
        }
    }
}

fn main() {
    let mut runtime = Noobkio::new();
    runtime.spawn(Sleep::new(Duration::from_secs(1)));

    let start = Instant::now();
    runtime.execute();
    let elapsed = start.elapsed();

    println!("{}s later..", elapsed.as_secs_f32());
}
