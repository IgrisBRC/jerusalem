use crate::{temple::Temple, wish::Sin};
use mio::net::TcpStream;
use std::io::Write;

mod del;
mod get;
mod ping;
mod set;

pub fn grant(terms: &[Vec<u8>], stream: &mut TcpStream, temple: &mut Temple) -> Result<(), Sin> {
    let command = std::str::from_utf8(&terms[0])
        .map_err(|_| Sin::Disconnected)?
        .to_uppercase();

    match std::str::from_utf8(&terms[0])
        .map_err(|_| Sin::Disconnected)?
        .to_uppercase()
        .as_str()
    {
        "SET" => set::set(terms, stream, temple)?,
        "GET" => get::get(terms, stream, temple)?,
        "PING" => ping::ping(stream)?,
        "DEL" => del::del(terms, stream, temple)?,
        "COMMAND" => stream.write_all(b"+OK\r\n").map_err(|_| Sin::Disconnected)?,
        "CONFIG" => stream.write_all(b"+OK\r\n").map_err(|_| Sin::Disconnected)?,
        _ => stream
            .write_all(b"-ERR unknown command\r\n")
            .map_err(|_| Sin::Disconnected)?,
    }

    Ok(())
}
