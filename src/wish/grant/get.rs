use std::io::Write;

use mio::net::TcpStream;

use crate::{
    temple::{Temple, Value},
    wish::Sin,
};

pub fn get(terms: &[Vec<u8>], stream: &mut TcpStream, temple: &mut Temple) -> Result<(), Sin> {
    if terms.len() < 2 {
        stream
            .write_all(b"-ERR Incorrect number of terms for GET\r\n")
            .map_err(|_| Sin::Disconnected)?;

        return Ok(())
    }

    let key = std::str::from_utf8(&terms[1]).map_err(|_| Sin::Utf8Error)?;

    match temple.get(key.to_string()) {
        Some((Value::String(value), _)) => {
            let mut response = Vec::with_capacity(value.len() + 16);
            response.extend_from_slice(format!("${}\r\n", value.len()).as_bytes());
            response.extend_from_slice(&value);
            response.extend_from_slice(b"\r\n");
            stream.write_all(&response).map_err(|_| Sin::Disconnected)?;
        }
        Some(_) => {
            stream
                .write_all(b"-ERR Calling GET on wrong data type\r\n")
                .map_err(|_| Sin::Disconnected)?;
        }
        None => {
            stream
                .write_all(b"$-1\r\n")
                .map_err(|_| Sin::Disconnected)?;
        }
    }

    Ok(())
}
