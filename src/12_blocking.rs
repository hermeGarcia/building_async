use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;

struct Request {
    id: usize,
    reply: oneshot::Sender<Response>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Response {
    id: usize,
}

async fn dispatcher(mut request_rx: mpsc::Receiver<Request>, start: Instant) {
    while let Some(req) = request_rx.recv().await {
        tokio::spawn(async move {
            let delay = if req.id == 5 {
                Duration::from_secs(3)
            } else {
                Duration::from_millis(200)
            };
            sleep(delay).await;
            println!(
                "[{:.2?}] task {} finished processing",
                start.elapsed(),
                req.id
            );
            let _ = req.reply.send(Response { id: req.id });
        });
    }
}

#[tokio::main]
async fn main() {
    let start = Instant::now();
    let (request_tx, request_rx) = mpsc::channel::<Request>(10);
    tokio::spawn(dispatcher(request_rx, start));

    let mut pending = Vec::new();
    for id in 0..10 {
        let (reply_tx, reply_rx) = oneshot::channel();
        request_tx
            .send(Request {
                id,
                reply: reply_tx,
            })
            .await
            .unwrap();
        pending.push(reply_rx);
    }

    for (slot, reply_rx) in pending.into_iter().enumerate() {
        let response = reply_rx.await.unwrap();
        println!(
            "[{:.2?}] received {response:?} while awaiting slot {slot}",
            start.elapsed()
        );
    }
}
