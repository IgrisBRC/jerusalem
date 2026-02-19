use std::{
    io::Write,
    sync::mpsc::{Receiver, Sender},
    time::SystemTime,
};

use mio::net::TcpStream;

use crate::{
    temple::{Temple, Value},
    wish::Sin,
};

pub fn append(
    terms: &[Vec<u8>],
    stream: &mut TcpStream,
    temple: &mut Temple,
    tx: Sender<Option<(Value, Option<SystemTime>)>>,
    rx: &Receiver<Option<(Value, Option<SystemTime>)>>,
) -> Result<(), Sin> {
    if terms.len() < 3 {
        stream
            .write_all(b"-ERR Incorrect number of terms for APPEND\r\n")
            .map_err(|_| Sin::Disconnected)?;

        return Ok(());
    }

    match temple.get(terms[1].clone(), tx.clone(), rx) {
        Some((Value::String(mut value), expiry)) => {
            value.append(&mut terms[2].clone());
            temple.insert(terms[1].clone(), (Value::String(value), expiry), tx, rx);

            stream
                .write_all(b"+OK\r\n")
                .map_err(|_| Sin::Disconnected)?;
        }
        Some(_) => {
            stream
                .write_all(b"-ERR Incorrect use of APPEND(for now, may implement in the future, also I don't even know if it exists for data structures other than string, like how do I append to a hashmap? That doesn't make sense.)\r\n")
                .map_err(|_| Sin::Disconnected)?;
        }
        None => {
            temple.insert(
                terms[1].clone(),
                (Value::String(terms[2].clone()), None),
                tx,
                rx,
            );

            stream.write_all(b"+OK\r\n").map_err(|_| Sin::Disconnected)?;
        }
    }

    Ok(())
}
