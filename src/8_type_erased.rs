mod common;

use std::ptr::NonNull;
use std::sync::mpsc::{self, Receiver, Sender};
use std::task::Poll;
use std::time::{Duration, Instant};

use crate::common::type_erased::{MyFuture, RandomNumber, Sleep};

struct Vtable {
    // Returns true if the future finished.
    poll: fn(NonNull<Header>) -> bool,
    dealloc: fn(NonNull<Header>),
}

#[repr(C)]
struct Header {
    vtable: &'static Vtable,
}

enum Status<F: MyFuture> {
    Running(F),
    Finished,
}

#[repr(C)]
struct InnerFuture<F: MyFuture> {
    header: Header,
    status: Status<F>,
    sender: Sender<F::Output>,
}

impl<F: MyFuture> InnerFuture<F> {
    const VTABLE: Vtable = Vtable {
        poll: poll_erased::<F>,
        dealloc: dealloc_erased::<F>,
    };
}

fn poll_erased<F: MyFuture>(ptr: NonNull<Header>) -> bool {
    let inner = unsafe { &mut *(ptr.as_ptr() as *mut InnerFuture<F>) };
    let Status::Running(future) = &mut inner.status else {
        return true;
    };

    match future.poll() {
        Poll::Pending => false,
        Poll::Ready(out) => {
            inner.status = Status::Finished;
            inner.sender.send(out).unwrap();
            true
        }
    }
}

fn dealloc_erased<F: MyFuture>(ptr: NonNull<Header>) {
    let inner = unsafe { Box::from_raw(ptr.as_ptr() as *mut InnerFuture<F>) };
    drop(inner);
}

struct NoobkioHandle<T> {
    rcv: Option<Receiver<T>>,
}

impl<T> From<Receiver<T>> for NoobkioHandle<T> {
    fn from(value: Receiver<T>) -> Self {
        Self { rcv: Some(value) }
    }
}

impl<T> MyFuture for NoobkioHandle<T> {
    type Output = T;

    fn poll(&mut self) -> Poll<Self::Output> {
        let Some(rcv) = self.rcv.take() else {
            return Poll::Pending;
        };

        match rcv.try_recv() {
            Ok(v) => Poll::Ready(v),
            Err(err) => {
                println!("ERR: {err:?}");
                self.rcv = Some(rcv);
                Poll::Pending
            }
        }
    }
}

pub struct RawFuture {
    ptr: NonNull<Header>,
}

impl RawFuture {
    fn new<F: MyFuture>(future: F) -> (Self, NoobkioHandle<F::Output>) {
        let (snd, rcv) = mpsc::channel();

        let boxed = Box::new(InnerFuture {
            header: Header {
                vtable: &InnerFuture::<F>::VTABLE,
            },
            status: Status::Running(future),
            sender: snd.into(),
        });

        let raw_future = RawFuture {
            ptr: NonNull::from(Box::leak(boxed)).cast(),
        };

        let handle = NoobkioHandle::from(rcv);

        (raw_future, handle)
    }

    fn poll(&self) -> bool {
        unsafe { (self.ptr.as_ref().vtable.poll)(self.ptr) }
    }
}

impl Drop for RawFuture {
    fn drop(&mut self) {
        unsafe { (self.ptr.as_ref().vtable.dealloc)(self.ptr) }
    }
}

pub struct Noobkio {
    futures: Vec<RawFuture>,
}

impl Noobkio {
    fn new() -> Self {
        Noobkio {
            futures: Vec::new(),
        }
    }

    fn spawn<F: MyFuture>(&mut self, future: F) -> NoobkioHandle<F::Output> {
        let (future, handle) = RawFuture::new(future);

        self.futures.push(future);

        handle
    }

    fn execute(&mut self) {
        while !self.futures.is_empty() {
            self.futures.retain_mut(|future| !future.poll());
        }
    }
}

fn main() {
    let mut runtime = Noobkio::new();

    let mut handle_1 = runtime.spawn(Sleep::new(Duration::from_secs(1)));
    let mut handle_2 = runtime.spawn(RandomNumber::from(3));

    let start = Instant::now();
    runtime.execute();
    let elapsed = start.elapsed();

    let Poll::Ready(_) = handle_1.poll() else {
        panic!("WTH?!");
    };

    let Poll::Ready(number) = handle_2.poll() else {
        panic!("WTH?!");
    };

    println!("{}s later..", elapsed.as_secs());
    println!("The number is {number}");
}
