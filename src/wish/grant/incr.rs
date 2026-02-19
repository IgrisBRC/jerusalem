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

pub fn incr(
    terms: &[Vec<u8>],
    stream: &mut TcpStream,
    temple: &mut Temple,
    tx: Sender<Option<(Value, Option<SystemTime>)>>,
    rx: &Receiver<Option<(Value, Option<SystemTime>)>>,
) -> Result<(), Sin> {
    if terms.len() < 2 {
        stream
            .write_all(b"-ERR Incorrect number of terms for INCR\r\n")
            .map_err(|_| Sin::Disconnected)?;

        return Ok(());
    }

    match temple.get(terms[1].clone(), tx.clone(), rx) {
        Some((Value::String(value), _)) => {
            if let Ok(value) = std::str::from_utf8(&value) {
                if let Ok(value) = value.parse::<i64>() {
                    let incremented_value = value + 1;

                    temple.insert(
                        terms[1].clone(),
                        (
                            Value::String(incremented_value.to_string().into_bytes()),
                            None,
                        ),
                        tx,
                        rx,
                    );

                    stream
                        .write_all(format!(":{}\r\n", incremented_value).as_bytes())
                        .map_err(|_| Sin::Disconnected)?;

                    return Ok(());
                }
            }

            stream
                .write_all(b"-ERR Incorrect use of INCR\r\n")
                .map_err(|_| Sin::Disconnected)?;
        }
        Some(_) => {
            stream
                .write_all(b"-ERR Incorrect use of INCR\r\n")
                .map_err(|_| Sin::Disconnected)?;
        }
        None => {
            temple.insert(
                terms[1].clone(),
                (Value::String(1.to_string().into_bytes()), None),
                tx,
                rx,
            );

            stream.write_all(b":1\r\n").map_err(|_| Sin::Disconnected)?;
        }
    }

    Ok(())
}
