mod common;

use std::task::Poll;
use std::time::{Duration, Instant};

use crate::common::simple::Sleep;

impl Future for Sleep {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self;
        let now = this.now.get_or_insert_with(Instant::now);
        let elapsed = now.elapsed();

        if elapsed < this.wait_for {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

#[tokio::main]
async fn main() {
    let now = Instant::now();
    Sleep::new(Duration::from_secs(1)).await;
    let elapsed = now.elapsed().as_secs_f32();

    println!("{elapsed}s later..");
}
