use std::time::{Duration, Instant};

pub type MyPoll = std::task::Poll<String>;

pub trait MyFuture {
    fn poll(&mut self) -> MyPoll;
}

pub struct Sleep {
    pub(crate) now: Option<Instant>,
    pub(crate) wait_for: Duration,
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
    fn poll(&mut self) -> MyPoll {
        let now = self.now.get_or_insert_with(Instant::now);
        let elapsed = now.elapsed();

        if elapsed < self.wait_for {
            MyPoll::Pending
        } else {
            MyPoll::Ready(String::with_capacity(0))
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
    fn poll(&mut self) -> MyPoll {
        if self.rounds == 0 {
            return MyPoll::Ready(self.max.to_string());
        }

        self.max = std::cmp::max(self.max, rand::random());
        self.rounds -= 1;
        MyPoll::Pending
    }
}

pub struct SlowRandomNumber {
    max: u32,
    rounds: u32,
    sleep: Sleep,
}

impl From<u32> for SlowRandomNumber {
    fn from(rounds: u32) -> Self {
        Self {
            rounds,
            max: 0,
            sleep: Sleep::new(Duration::from_millis(100)),
        }
    }
}

impl MyFuture for SlowRandomNumber {
    fn poll(&mut self) -> MyPoll {
        if let MyPoll::Pending = self.sleep.poll() {
            return MyPoll::Pending;
        }

        if self.rounds == 0 {
            return MyPoll::Ready(self.max.to_string());
        }

        self.max = std::cmp::max(self.max, rand::random());
        self.rounds -= 1;
        self.sleep = Sleep::new(Duration::from_millis(100));

        MyPoll::Pending
    }
}
