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

    let choir = Choir::new(6);

    let temple = Temple::new("IgrisDB".to_string());

    let (tx, rx) = std::sync::mpsc::channel();

    loop {
        while let Ok((token, pilgrim)) = rx.try_recv() {
            pilgrim_map.insert(token, pilgrim);
        }

        // for _ in 0..256 {
        //     match rx.try_recv() {
        //         Ok((token, pilgrim)) => { pilgrim_map.insert(token, pilgrim); }
        //         Err(_) => break,
        //     }
        // }
        
        if poll
            .poll(&mut events, Some(std::time::Duration::from_millis(0)))
            .is_err()
        {
            eprintln!("poll() gone wrong");
        }

        for event in &events {
            let token = event.token();
            match token {
                SERVER => loop {
                    match listener.accept() {
                        Ok((mut stream, _address)) => {
                            // println!("Got a connection from: {}", address);

                            let pilgrim_token = Token(pilgrim_counter);

                            if poll.registry().register(
                                &mut stream,
                                pilgrim_token,
                                Interest::READABLE | Interest::WRITABLE,
                            ).is_err() {
                                eprintln!("register() gone wrong");
                            }

                            pilgrim_counter += 1;

                            let (pilgrim_tx, pilgrim_rx) = std::sync::mpsc::channel();

                            pilgrim_map.insert(
                                pilgrim_token,
                                Pilgrim {
                                    stream,
                                    virtue: None,
                                    tx: pilgrim_tx,
                                    rx: pilgrim_rx,
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
                    if let Some(mut pilgrim) = pilgrim_map.remove(&Token(token_number)) {
                        let sanctum = temple.sanctify();
                        let token_number = token_number;
                        let tx = tx.clone();

                        choir.sing(move || {
                            if let Ok(_) = wish::wish(&mut pilgrim, sanctum) {
                                if tx.send((mio::Token(token_number), pilgrim)).is_err() {
                                    eprintln!("angel panicked");
                                }
                            }
                        });
                    }
                }
            }
        }
    }
}
