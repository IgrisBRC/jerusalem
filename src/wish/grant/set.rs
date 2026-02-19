use crate::{
    temple::{Temple, Value},
    wish::Sin,
};
use mio::net::TcpStream;
use std::{
    io::Write,
    sync::mpsc::{Receiver, Sender},
    time::SystemTime,
};

pub fn set(
    terms: &[Vec<u8>],
    stream: &mut TcpStream,
    temple: &mut Temple,
    tx: Sender<Option<(Value, Option<SystemTime>)>>,
    rx: &Receiver<Option<(Value, Option<SystemTime>)>>,
) -> Result<(), Sin> {
    if terms.len() < 3 {
        stream
            .write_all(b"-ERR wrong number of arguments for SET command\r\n")
            .map_err(|_| Sin::Disconnected)?;
        return Ok(());
    }

    let value = terms[2].clone();

    if terms.len() == 5 {
        if let Ok(command) = std::str::from_utf8(&terms[3]) {
            if command.to_uppercase() == "EX" {
                if let Ok(expiry) = std::str::from_utf8(&terms[4]) {
                    if let Ok(expiry) = expiry.parse::<u64>() {
                        temple.insert(
                            terms[1].clone(),
                            (
                                Value::String(value),
                                Some(SystemTime::now() + std::time::Duration::from_secs(expiry)),
                            ),
                            tx,
                            rx,
                        );
                    }
                }
            }
        }
    } else {
        temple.insert(terms[1].clone(), (Value::String(value), None), tx, rx);
    }

    stream
        .write_all(b"+OK\r\n")
        .map_err(|_| Sin::Disconnected)?;

    Ok(())
}
