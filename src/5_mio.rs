use std::io::{self, ErrorKind, Read, Result};

use mio::event::Events;
use mio::net::{TcpListener, TcpStream};
use mio::{Interest, Poll, Token};

const PLACEHOLDER: Token = Token(0);

fn handle_events(events: &Events, stream: &mut TcpStream) -> Result<()> {
    for event in events.iter() {
        let _index: usize = event.token().into();
        let mut data = vec![0u8; 4096];
        loop {
            match stream.read(&mut data) {
                Ok(n) if n == 0 => break,

                Ok(n) => {
                    let txt = String::from_utf8_lossy(&data[..n]);
                    println!("{txt}\n------\n");
                }

                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => break,
                Err(e) => return Err(e),
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let mut poll = Poll::new()?;

    let addr = "127.0.0.1:8080";

    let listener = std::net::TcpListener::bind(addr)?;
    let mut listener = TcpListener::from_std(listener);
    poll.registry()
        .register(&mut listener, PLACEHOLDER, Interest::READABLE)?;

    let mut events = mio::Events::with_capacity(10);

    while events.is_empty() {
        poll.poll(&mut events, None)?;
    }

    let mut stream = loop {
        match listener.accept() {
            Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e),
            Ok((mut stream, _)) => {
                poll.registry()
                    .register(&mut stream, PLACEHOLDER, Interest::READABLE)?;
                break stream;
            }
        }
    };

    poll.registry().deregister(&mut listener)?;
    poll.poll(&mut events, None)?;

    while events.is_empty() {
        poll.poll(&mut events, None)?;
        println!("TIMEOUT (OR SPURIOUS EVENT NOTIFICATION)");
    }

    handle_events(&events, &mut stream)?;
    Ok(())
}
