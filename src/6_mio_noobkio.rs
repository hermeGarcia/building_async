mod common;

use std::collections::HashMap;
use std::io::{ErrorKind, Result};

use mio::event::Events;
use mio::net::TcpListener;
use mio::{Interest, Poll, Token};

use crate::common::simple::MyPoll;

pub trait MyFuture {
    fn spawn(self, runtime: &mut Noobkio) -> Result<()>
    where
        Self: Sized + 'static,
    {
        runtime.futures.push(Box::new(self));
        Ok(())
    }

    fn poll(&mut self) -> MyPoll;
}

pub struct NoobkioTcpListener {
    inner: TcpListener,
}

impl NoobkioTcpListener {
    pub fn bind(addr: &str) -> Result<NoobkioTcpListener> {
        let listener = std::net::TcpListener::bind(addr)?;
        let listener = TcpListener::from_std(listener);

        Ok(NoobkioTcpListener { inner: listener })
    }
}

impl MyFuture for NoobkioTcpListener {
    fn spawn(self, runtime: &mut Noobkio) -> Result<()> {
        let token = runtime.next_token();
        let mut listener = self;

        runtime.poll.registry().register(
            &mut listener.inner,
            token,
            Interest::READABLE | Interest::WRITABLE,
        )?;

        let future = Box::new(listener);
        runtime.waiting_io.insert(token, future);

        Ok(())
    }

    fn poll(&mut self) -> MyPoll {
        match self.inner.accept() {
            Err(e) if e.kind() == ErrorKind::WouldBlock => MyPoll::Pending,
            Err(e) => MyPoll::Ready(e.to_string()),
            Ok((_, addr)) => MyPoll::Ready(format!("Connected to {addr:?}")),
        }
    }
}

pub struct Noobkio {
    events: Events,
    poll: Poll,
    token: u64,
    waiting_io: HashMap<Token, Box<dyn MyFuture + 'static>>,
    futures: Vec<Box<dyn MyFuture + 'static>>,
}

impl Noobkio {
    fn new() -> Self {
        Noobkio {
            futures: Vec::new(),
            poll: Poll::new().unwrap(),
            waiting_io: HashMap::default(),
            events: Events::with_capacity(1024),
            token: 0,
        }
    }

    fn spawn(&mut self, future: impl MyFuture + 'static) -> Result<()> {
        future.spawn(self)
    }

    fn execute(&mut self) {
        loop {
            if self.futures.is_empty() && self.waiting_io.is_empty() {
                break;
            }

            self.futures.retain_mut(|future| match future.poll() {
                MyPoll::Pending => true,

                MyPoll::Ready(output) => {
                    println!("{output}");
                    false
                }
            });

            if !self.futures.is_empty() {
                continue;
            }

            if !self.waiting_io.is_empty() {
                self.poll.poll(&mut self.events, None).unwrap();
            }

            for event in self.events.iter() {
                let Some(future) = self.waiting_io.remove(&event.token()) else {
                    continue;
                };

                self.futures.push(future);
            }
        }
    }

    fn next_token(&mut self) -> Token {
        let token = Token(self.token as usize);
        self.token += 1;

        token
    }
}

fn main() -> Result<()> {
    let mut runtime = Noobkio::new();

    runtime
        .spawn(NoobkioTcpListener::bind("127.0.0.1:8080").unwrap())
        .unwrap();
    runtime
        .spawn(NoobkioTcpListener::bind("127.0.0.1:8081").unwrap())
        .unwrap();
    runtime
        .spawn(NoobkioTcpListener::bind("127.0.0.1:8082").unwrap())
        .unwrap();

    runtime.execute();

    Ok(())
}
