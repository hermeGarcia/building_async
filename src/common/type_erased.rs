use std::task::Poll;
use std::time::{Duration, Instant};

pub trait MyFuture {
    type Output;

    fn poll(&mut self) -> Poll<Self::Output>;
}

pub struct Sleep {
    now: Option<Instant>,
    wait_for: Duration,
}

impl Sleep {
    pub fn new(duration: Duration) -> Sleep {
        Self {
            now: None,
            wait_for: duration,
        }
    }
}

impl MyFuture for Sleep {
    type Output = ();

    fn poll(&mut self) -> Poll<Self::Output> {
        let now = self.now.get_or_insert_with(Instant::now);
        if now.elapsed() < self.wait_for {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

pub struct RandomNumber {
    max: u32,
    rounds: u32,
}

impl From<u32> for RandomNumber {
    fn from(value: u32) -> Self {
        Self {
            max: 0,
            rounds: value,
        }
    }
}

impl MyFuture for RandomNumber {
    type Output = u32;

    fn poll(&mut self) -> Poll<Self::Output> {
        if self.rounds == 0 {
            return Poll::Ready(self.max);
        }

        self.max = std::cmp::max(self.max, rand::random());
        self.rounds -= 1;
        Poll::Pending
    }
}
