
use crate::{temple::{Temple, Value}, wish::Sin};
use mio::net::TcpStream;
use std::{io::Write, sync::mpsc::{Receiver, Sender}, time::SystemTime};

pub fn exists(terms: &[Vec<u8>], stream: &mut TcpStream, temple: &mut Temple,
    tx: Sender<Option<(Value, Option<SystemTime>)>>,
    rx: &Receiver<Option<(Value, Option<SystemTime>)>>,
) -> Result<(), Sin> {
    if terms.len() < 2 {
        stream
            .write_all(b"-ERR wrong number of arguments for EXISTS command\r\n")
            .map_err(|_| Sin::Disconnected)?;
        return Ok(());
    }

    let mut count = 0;

    for key in &terms[1..] {
        if temple.get(key.clone(), tx.clone(), &rx).is_some() {
            count += 1;
        }
    }

    let response = format!(":{}\r\n", count);

    stream
        .write_all(response.as_bytes())
        .map_err(|_| Sin::Disconnected)?;

    Ok(())
}
