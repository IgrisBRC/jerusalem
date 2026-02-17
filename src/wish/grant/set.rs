use crate::{
    temple::{Temple, Value},
    wish::Sin,
};
use mio::net::TcpStream;
use std::io::Write;

pub fn set(terms: &[Vec<u8>], stream: &mut TcpStream, temple: &mut Temple) -> Result<(), Sin> {
    if terms.len() < 3 {
        stream
            .write_all(b"-ERR wrong number of arguments for SET command\r\n")
            .map_err(|_| Sin::Disconnected)?;
        return Ok(());
    }

    let key = std::str::from_utf8(&terms[1]).map_err(|_| Sin::Utf8Error)?;

    let val = terms[2].clone();

    temple.insert(key.to_string(), (Value::String(val), None));

    stream
        .write_all(b"+OK\r\n")
        .map_err(|_| Sin::Disconnected)?;

    Ok(())
}
