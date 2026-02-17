use mio::net::TcpStream;
use std::io::Write;

use crate::wish::Sin;

pub fn ping(stream: &mut TcpStream) -> Result<(), Sin> {
    stream
        .write_all(b"+PONG\r\n")
        .map_err(|_| Sin::Disconnected)?;

    Ok(())
}
