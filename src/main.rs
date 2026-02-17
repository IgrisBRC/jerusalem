use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use mini_redis::choir::Choir;
use mini_redis::temple::Temple;
use mini_redis::wish::{self, Pilgrim};
use mio::net::TcpListener;
use mio::{Events, Interest, Poll, Token};

fn main() {
    let ipv4_addr = Ipv4Addr::new(127, 0, 0, 1);
    let port = 6379;
    let socket_addr_v4 = SocketAddrV4::new(ipv4_addr, port);
    let socket_addr = SocketAddr::V4(socket_addr_v4);

    let mut poll = Poll::new().unwrap();

    let mut listener = TcpListener::bind(socket_addr).unwrap();

    const SERVER: Token = Token(0);

    let mut events = Events::with_capacity(128);

    poll.registry()
        .register(&mut listener, SERVER, Interest::READABLE)
        .unwrap();

    let mut pilgrim_map = HashMap::new();
    let mut pilgrim_counter = 1;

    let choir = Choir::new(5);

    let temple = Temple::new("IgrisDB".to_string());

    loop {
        poll.poll(&mut events, Some(std::time::Duration::from_millis(100)));

        for event in &events {
            let token = event.token();
            match token {
                SERVER => loop {
                    match listener.accept() {
                        Ok((mut stream, address)) => {
                            // println!("Got a connection from: {}", address);

                            let pilgrim_token = Token(pilgrim_counter);

                            poll.registry().register(
                                &mut stream,
                                pilgrim_token,
                                Interest::READABLE | Interest::WRITABLE,
                            );

                            pilgrim_counter += 1;

                            pilgrim_map.insert(
                                pilgrim_token,
                                Pilgrim {
                                    stream,
                                    virtue: None,
                                },
                            );
                        }
                        Err(err) => {
                            if err.kind() == ErrorKind::WouldBlock {
                                break;
                            }
                        }
                    }
                },

                Token(token_number) => {
                    if let Some(pilgrim) = pilgrim_map.get_mut(&Token(token_number)) {
                        if let Err(_) = wish::wish(pilgrim, temple.sanctify()) {
                            pilgrim_map.remove(&Token(token_number));
                        }
                    }
                }
            }
        }
    }
}
