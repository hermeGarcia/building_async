use std::time::{Duration, Instant};

pub enum MyPoll {
    Pending,
    Ready,
}

trait MyFuture {
    fn poll(&mut self) -> MyPoll;
}

struct Sleep {
    now: Option<Instant>,
    wait_for: Duration,
}

impl Sleep {
    fn new(duration: Duration) -> Sleep {
        Self {
            now: None,
            wait_for: duration,
        }
    }
}

impl MyFuture for Sleep {
    fn poll(&mut self) -> MyPoll {
        let now = self.now.get_or_insert_with(Instant::now);
        let elapsed = now.elapsed();

        if elapsed < self.wait_for {
            MyPoll::Pending
        } else {
            MyPoll::Ready
        }
    }
}

fn main() {
    let mut sleep = Sleep::new(Duration::from_secs(1));

    while let MyPoll::Pending = sleep.poll() {}

    println!("Finished");
}
