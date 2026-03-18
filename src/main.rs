#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};

use jerusalem::choir::Choir;
use jerusalem::egress;
use jerusalem::temple::Temple;
use jerusalem::temple::soul::SaveError;
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

    let mut server_temple = temple.clone();

    let (ingress_tx, ingress_rx) = std::sync::mpsc::channel::<(Token, Pilgrim)>();
    let (egress_tx, egress_rx) = std::sync::mpsc::channel();

    let shutdown_tx = egress_tx.clone();

    let (pilgrim_tx, pilgrim_rx) = std::sync::mpsc::channel::<Decree>();

    let dummy_tx = pilgrim_tx.clone();

    std::thread::spawn(move || {
        egress::egress(pilgrim_rx, egress_tx);
    });

    if ctrlc::set_handler(move || {
        let (server_tx, server_rx) = std::sync::mpsc::channel::<Result<(), SaveError>>();

        server_temple.save(dummy_tx.clone(), server_tx, SERVER);

        if let Ok(Ok(())) = server_rx.recv() {
            println!("Database snapshot saved successfuly");
        } else {
            println!("Couldn't save database snapshot");
        }

        if shutdown_tx.send(SERVER).is_err() {
            eprintln!("Ctrlc handler failed");
        }
    })
    .is_err()
    {
        eprintln!("Failed to set ctrlc handler");
    };

    loop {
        while let Ok(token) = egress_rx.try_recv() {
            if token == Token(0) {
                std::process::exit(0);
            }

            if let Some(mut pilgrim) = ingress_map.remove(&token)
                && poll.registry().deregister(&mut pilgrim.stream).is_err()
            {
                eprintln!("deregister() failed")
            }
        }

        if poll
            .poll(&mut events, Some(std::time::Duration::from_millis(100)))
            .is_err()
        {
            eprintln!("poll() failed");
        }

        while let Ok((token, mut pilgrim)) = ingress_rx.try_recv() {
            if poll
                .registry()
                .reregister(
                    &mut pilgrim.stream,
                    token,
                    Interest::READABLE | Interest::WRITABLE,
                )
                .is_err()
            {
                // eprintln!("reregister() failed");
            }

            ingress_map.insert(token, pilgrim);
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
