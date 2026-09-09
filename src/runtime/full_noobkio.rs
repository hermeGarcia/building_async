#![allow(unused)]

use std::future::Future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::mpsc::{self, Receiver, Sender};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

struct Vtable {
    // Returns true if the future finished.
    poll: fn(NonNull<Header>, &mut Context<'_>) -> bool,
    dealloc: fn(NonNull<Header>),
}

#[repr(C)]
struct Header {
    vtable: &'static Vtable,
}

enum Status<F: Future> {
    Running(Pin<Box<F>>),
    Finished,
}

#[repr(C)]
struct InnerFuture<F: Future> {
    header: Header,
    status: Status<F>,
    sender: Sender<F::Output>,
}

impl<F: Future> InnerFuture<F> {
    const VTABLE: Vtable = Vtable {
        poll: poll_erased::<F>,
        dealloc: dealloc_erased::<F>,
    };
}

fn poll_erased<F: Future>(ptr: NonNull<Header>, cx: &mut Context<'_>) -> bool {
    let inner = unsafe { &mut *(ptr.as_ptr() as *mut InnerFuture<F>) };
    let Status::Running(future) = &mut inner.status else {
        return true;
    };

    match future.as_mut().poll(cx) {
        Poll::Pending => false,
        Poll::Ready(out) => {
            inner.status = Status::Finished;
            inner.sender.send(out).unwrap();
            true
        }
    }
}

fn dealloc_erased<F: Future>(ptr: NonNull<Header>) {
    let inner = unsafe { Box::from_raw(ptr.as_ptr() as *mut InnerFuture<F>) };
    drop(inner);
}

pub struct NoobkioHandle<T> {
    pub rcv: Option<Receiver<T>>,
}

impl<T> From<Receiver<T>> for NoobkioHandle<T> {
    fn from(value: Receiver<T>) -> Self {
        Self { rcv: Some(value) }
    }
}

impl<T> Future for NoobkioHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let Some(rcv) = this.rcv.take() else {
            return Poll::Pending;
        };

        match rcv.try_recv() {
            Ok(v) => Poll::Ready(v),
            Err(err) => {
                println!("ERR: {err:?}");
                this.rcv = Some(rcv);
                Poll::Pending
            }
        }
    }
}

impl<T> NoobkioHandle<T> {
    pub fn unchecked_rcv(self) -> T {
        self.rcv.unwrap().try_recv().unwrap()
    }
}

pub struct RawFuture {
    ptr: NonNull<Header>,
}

impl RawFuture {
    fn new<F: Future>(future: F) -> (Self, NoobkioHandle<F::Output>) {
        let (snd, rcv) = mpsc::channel();

        let boxed = Box::new(InnerFuture {
            header: Header {
                vtable: &InnerFuture::<F>::VTABLE,
            },
            status: Status::Running(Box::pin(future)),
            sender: snd.into(),
        });

        let raw_future = RawFuture {
            ptr: NonNull::from(Box::leak(boxed)).cast(),
        };

        let handle = NoobkioHandle::from(rcv);

        (raw_future, handle)
    }

    fn poll(&self) -> bool {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        unsafe { (self.ptr.as_ref().vtable.poll)(self.ptr, &mut cx) }
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
    pub fn new() -> Self {
        Noobkio {
            futures: Vec::new(),
        }
    }

    pub fn spawn<F: Future>(&mut self, future: F) -> NoobkioHandle<F::Output> {
        let (future, handle) = RawFuture::new(future);

        self.futures.push(future);

        handle
    }

    pub fn execute(&mut self) {
        while !self.futures.is_empty() {
            self.futures.retain_mut(|future| !future.poll());
        }
    }
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

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

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
