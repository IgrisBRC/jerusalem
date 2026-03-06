#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use std::collections::HashMap;
use std::io::{ErrorKind, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};

use jerusalem::choir::Choir;
use jerusalem::egress;
use jerusalem::temple::Temple;
use jerusalem::wish::grant::Decree;
use jerusalem::wish::{self, Pilgrim};

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

    let mut ingress_map: HashMap<Token, Pilgrim> = HashMap::new();

    let mut pilgrim_counter = 1;

    let ingress_choir = Choir::new(3);

    let temple = Temple::new();

    let (ingress_tx, ingress_rx) = std::sync::mpsc::channel();
    let (egress_tx, egress_rx) = std::sync::mpsc::channel();

    let (pilgrim_tx, pilgrim_rx) = std::sync::mpsc::channel::<Decree>();

    std::thread::spawn(move || {
        let mut egress_map: HashMap<Token, mio::net::TcpStream> = HashMap::new();
        let mut buffer = Vec::with_capacity(2100);
        let mut itoa_buf = itoa::Buffer::new();

        loop {
            match pilgrim_rx.recv() {
                Ok(Decree::Welcome(token, stream)) => {
                    egress_map.insert(token, stream);
                }
                Ok(Decree::Deliver(gift)) => {
                    if let Some(stream) = egress_map.get_mut(&gift.token) {
                        let token = gift.token;

                        if egress::egress(stream, gift, &mut buffer).is_err()
                            && egress_tx.send(token).is_err()
                        {
                            eprintln!("angel panicked");
                        };
                    }
                }
                Ok(Decree::Broadcast(token, event, message, clients)) => {
                    let clients_len = clients.len();

                    let mut response = b"*3\r\n$7\r\nmessage\r\n$".to_vec();
                    response.extend_from_slice(itoa_buf.format(event.len()).as_bytes());
                    response.extend_from_slice(b"\r\n");
                    response.extend_from_slice(&event);
                    response.extend_from_slice(b"\r\n$");
                    response.extend_from_slice(itoa_buf.format(message.len()).as_bytes());
                    response.extend_from_slice(b"\r\n");
                    response.extend_from_slice(&message);
                    response.extend_from_slice(b"\r\n");

                    for client in clients {
                        if let Some(stream) = egress_map.get_mut(&client)
                            && stream.write_all(&response).is_err()
                        {
                            eprintln!("writing to stream failed for client");
                        }
                    }

                    if let Some(publisher_stream) = egress_map.get_mut(&token) {
                        let mut response = b":".to_vec();
                        response.extend_from_slice(itoa_buf.format(clients_len).as_bytes());
                        response.extend_from_slice(b"\r\n");

                        if publisher_stream.write_all(&response).is_err() {
                            eprintln!("writing to stream failed for publisher");
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    loop {
        while let Ok((token, pilgrim)) = ingress_rx.try_recv() {
            ingress_map.insert(token, pilgrim);

            if let Some(p) = ingress_map.get_mut(&token)
                && poll
                    .registry()
                    .reregister(
                        &mut p.stream,
                        token,
                        Interest::READABLE | Interest::WRITABLE,
                    )
                    .is_err()
            {
                // eprintln!("reregister() failed");
            }
        }

        while let Ok(token) = egress_rx.try_recv() {
            if let Some(mut pilgrim) = ingress_map.remove(&token)
                && poll.registry().deregister(&mut pilgrim.stream).is_err()
            {
                eprintln!("deregister() failed")
            }
        }

        if poll
            .poll(&mut events, Some(std::time::Duration::from_millis(10)))
            .is_err()
        {
            eprintln!("poll() failed");
        }

        for event in &events {
            let token = event.token();
            match token {
                SERVER => loop {
                    match listener.accept() {
                        Ok((mut stream, _address)) => {
                            let pilgrim_token = Token(pilgrim_counter);

                            if poll
                                .registry()
                                .register(
                                    &mut stream,
                                    pilgrim_token,
                                    Interest::READABLE | Interest::WRITABLE,
                                )
                                .is_err()
                            {
                                eprintln!("register() failed");
                            }

                            let std_stream: TcpStream = stream.into();
                            let std_stream_clone: TcpStream =
                                std_stream.try_clone().expect("Failed to clone socket");

                            let ingress_mio = mio::net::TcpStream::from_std(std_stream);
                            let egress_mio = mio::net::TcpStream::from_std(std_stream_clone);

                            pilgrim_counter += 1;

                            ingress_map.insert(
                                pilgrim_token,
                                Pilgrim {
                                    stream: ingress_mio,
                                    virtue: None,
                                    tx: pilgrim_tx.clone(),
                                },
                            );

                            pilgrim_tx
                                .send(Decree::Welcome(pilgrim_token, egress_mio))
                                .unwrap();
                        }
                        Err(err) => {
                            if err.kind() == ErrorKind::WouldBlock {
                                break;
                            }
                        }
                    }
                },

                Token(token_number) => {
                    if let Some(mut pilgrim) = ingress_map.remove(&Token(token_number)) {
                        let sanctum = temple.sanctify();
                        let tx = ingress_tx.clone();

                        ingress_choir.sing(move || {
                            match wish::wish(&mut pilgrim, sanctum, Token(token_number)) {
                                Ok(_) => {
                                    if tx.send((mio::Token(token_number), pilgrim)).is_err() {
                                        eprintln!("angel panicked");
                                    }
                                }
                                Err(_e) => {
                                    // eprintln!("{:?}", e);
                                }
                            }
                        });
                    }
                }
            }
        }
    }
}
