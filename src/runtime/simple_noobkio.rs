use std::sync::{Arc, Mutex};

use crate::common::simple::{MyFuture, MyPoll};

pub struct Handle {
    inner: Arc<Mutex<Option<String>>>,
}

impl Handle {
    fn inner_clone(&self) -> Handle {
        Handle {
            inner: self.inner.clone(),
        }
    }

    pub fn try_join(&mut self) -> Option<String> {
        match self.inner.try_lock() {
            Err(_) => None,
            Ok(mut guard) => guard.take(),
        }
    }
}

impl MyFuture for Handle {
    fn poll(&mut self) -> MyPoll {
        match self.try_join() {
            None => MyPoll::Pending,
            Some(out) => MyPoll::Ready(out),
        }
    }
}

pub struct Noobkio {
    futures: Vec<(Box<dyn MyFuture + 'static>, Handle)>,
}

impl Noobkio {
    pub fn new() -> Self {
        Noobkio {
            futures: Vec::new(),
        }
    }

    pub fn spawn(&mut self, mut future: impl MyFuture + 'static) -> Handle {
        if let MyPoll::Ready(result) = future.poll() {
            return Handle {
                inner: Arc::new(Mutex::new(Some(result))),
            };
        }

        let handle = Handle {
            inner: Arc::new(Mutex::new(None)),
        };

        self.futures.push((Box::new(future), handle.inner_clone()));

        handle
    }

    pub fn execute(&mut self) {
        while !self.futures.is_empty() {
            self.futures
                .retain_mut(|(future, handle)| match future.as_mut().poll() {
                    MyPoll::Pending => true,
                    MyPoll::Ready(out) => {
                        let mut guard = handle
                            .inner
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner());

                        *guard = Some(out);
                        false
                    }
                });
        }
    }
}

impl<T: MyFuture> MyFuture for Box<T> {
    fn poll(&mut self) -> MyPoll {
        self.as_mut().poll()
    }
}
