use crate::{temple::{Temple, Value}, wish::Sin};
use mio::net::TcpStream;
use std::{io::Write, sync::mpsc::{Receiver, Sender}, time::SystemTime};

mod del;
mod get;
mod ping;
mod set;
mod exists;
mod incr;
mod decr;
mod append;

pub fn grant(
    terms: &[Vec<u8>],
    stream: &mut TcpStream,
    temple: &mut Temple,
    tx: Sender<Option<(Value, Option<SystemTime>)>>,
    rx: &Receiver<Option<(Value, Option<SystemTime>)>>,
) -> Result<(), Sin> {
    match std::str::from_utf8(&terms[0])
        .map_err(|_| Sin::Disconnected)?
        .to_uppercase()
        .as_str()
    {
        "SET" => set::set(terms, stream, temple, tx, rx)?,
        "GET" => get::get(terms, stream, temple, tx, rx)?,
        "PING" => ping::ping(stream)?,
        "DEL" => del::del(terms, stream, temple, tx, rx)?,
        "EXISTS" => exists::exists(terms, stream, temple, tx, rx)?,
        "INCR" => incr::incr(terms, stream, temple, tx, rx)?,
        "DECR" => decr::decr(terms, stream, temple, tx, rx)?,
        "APPEND" => append::append(terms, stream, temple, tx, rx)?,
        "COMMAND" => stream
            .write_all(b"+OK\r\n")
            .map_err(|_| Sin::Disconnected)?,
        "CONFIG" => stream
            .write_all(b"+OK\r\n")
            .map_err(|_| Sin::Disconnected)?,
        _ => stream
            .write_all(b"-ERR unknown command\r\n")
            .map_err(|_| Sin::Disconnected)?,
    }

    Ok(())
}
