use std::io::Write;
use std::{
    io::{BufReader, BufWriter, Lines},
    net::TcpStream,
};

use crate::handle_connection::util;
use crate::memory_database::MemoryDatabase;

pub fn handle_get(
    db: &mut MemoryDatabase,
    reader_lines: &mut Lines<BufReader<&TcpStream>>,
    count: usize,
    wstream: &mut BufWriter<&TcpStream>,
    count_ledger: &mut i32,
) -> Result<(), String> {
    if count != 2 {
        util::write_to_wstream(wstream, b"-ERR Protocol Error\r\n")?;
        util::cleanup(count_ledger, reader_lines);
        return Ok(());
    }

    let key = match util::validate_and_get_next_term(reader_lines, count_ledger) {
        Ok(t) => t,
        Err(e) => {
            util::write_to_wstream(wstream, format!("{}\r\n", e).as_bytes())?;
            return Ok(());
        }
    };

    if let Some(value_bytes) = db.get(&key) {
        let header = format!("${}\r\n", value_bytes.len());

        wstream
            .write_all(header.as_bytes())
            .map_err(|_| "Failed to write to stream")?;
        wstream
            .write_all(&value_bytes)
            .map_err(|_| "Failed to write to stream")?;
        wstream
            .write_all(b"\r\n")
            .map_err(|_| "Failed to write to stream")?;

        wstream.flush().map_err(|_| "Failed to flush stream")?;
    } else {
        util::write_to_wstream(wstream, b"$-1\r\n")?;
    }

    Ok(())
}
