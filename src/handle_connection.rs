use std::{
    io::{BufRead, BufReader, BufWriter},
    net::TcpStream,
};

use crate::memory_database::MemoryDatabase;

mod util;
mod handle_get;
mod handle_set;
mod handle_del;
mod handle_exists;
mod handle_incr;

pub fn handle_connection(rstream: TcpStream, db: &mut MemoryDatabase) -> Result<(), String> {
    let mut wstream = BufWriter::new(&rstream);
    let reader = BufReader::new(&rstream);
    let mut reader_lines = reader.lines();

    loop {
        let line = reader_lines
            .next()
            .ok_or("Connection closed by client?")?
            .map_err(|_| "Failed to read line")?;

        let count = if line.starts_with('*') {
            line[1..]
                .trim()
                .parse::<usize>()
                .map_err(|_| "Failed to parse")?
        } else {
            util::write_to_wstream(&mut wstream, b"-ERR Protocol Error\r\n")?;
            continue;
        };

        let mut count_ledger: i32 = count as i32;

        let term = match util::validate_and_get_next_term(&mut reader_lines, &mut count_ledger) {
            Ok(t) => t,
            Err(e) => {
                util::write_to_wstream(&mut wstream, format!("{}\r\n", e).as_bytes())?;
                continue;
            }
        };

        match term.to_uppercase().as_str() {
            "PING" => {
                util::write_to_wstream(&mut wstream, b"+PONG\r\n")?;
            }
            "GET" => {
                handle_get::handle_get(db, &mut reader_lines, count, &mut wstream, &mut count_ledger)?;
            }
            "SET" => {
                handle_set::handle_set(db, &mut reader_lines, count, &mut wstream, &mut count_ledger)?;
            }
            "COMMAND" => {
                util::write_to_wstream(&mut wstream, b"*0\r\n")?;
            }
            "DEL" => {
                handle_del::handle_del(db, &mut reader_lines, count, &mut wstream, &mut count_ledger)?;
            }
            "EXISTS" => {
                handle_exists::handle_exists(db, &mut reader_lines, count, &mut wstream, &mut count_ledger)?;
            }
            "INCR" => {
                handle_incr::handle_incr(db, &mut reader_lines, count, &mut wstream, &mut count_ledger)?;
            }

            _ => {
                let err_msg = format!("-ERR Unknown command {}\r\n", term);
                util::write_to_wstream(&mut wstream, err_msg.as_bytes())?;
            }
        }

        util::cleanup(&mut count_ledger, &mut reader_lines);
    }
}

