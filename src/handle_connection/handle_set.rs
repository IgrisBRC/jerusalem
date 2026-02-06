use std::{
    io::{BufReader, BufWriter, Lines},
    net::TcpStream,
    time::SystemTime,
};

use crate::handle_connection::util;
use crate::memory_database::MemoryDatabase;

pub fn handle_set(
    db: &mut MemoryDatabase,
    reader_lines: &mut Lines<BufReader<&TcpStream>>,
    count: usize,
    wstream: &mut BufWriter<&TcpStream>,
    count_ledger: &mut i32,
) -> Result<(), String> {
    if count == 3 {
        let key = match util::validate_and_get_next_term(reader_lines, count_ledger) {
            Ok(t) => t,
            Err(e) => {
                util::write_to_wstream(wstream, format!("{}\r\n", e).as_bytes())?;
                return Ok(());
            }
        };

        let value = match util::validate_and_get_next_term(reader_lines, count_ledger) {
            Ok(t) => t,
            Err(e) => {
                util::write_to_wstream(wstream, format!("{}\r\n", e).as_bytes())?;
                return Ok(());
            }
        };

        db.insert(&key, (value.into_bytes(), None));

        util::write_to_wstream(wstream, b"+OK\r\n")?;

        return Ok(());
    } else if count == 5 {
        let key = match util::validate_and_get_next_term(reader_lines, count_ledger) {
            Ok(t) => t,
            Err(e) => {
                util::write_to_wstream(wstream, format!("{}\r\n", e).as_bytes())?;
                return Ok(());
            }
        };

        let value = match util::validate_and_get_next_term(reader_lines, count_ledger) {
            Ok(t) => t,
            Err(e) => {
                util::write_to_wstream(wstream, format!("{}\r\n", e).as_bytes())?;
                return Ok(());
            }
        };

        let term = match util::validate_and_get_next_term(reader_lines, count_ledger) {
            Ok(t) => t,
            Err(e) => {
                util::write_to_wstream(wstream, format!("{}\r\n", e).as_bytes())?;
                return Ok(());
            }
        };

        match term.to_uppercase().as_str() {
            "EX" => {
                let expiry = match util::validate_and_get_next_term(reader_lines, count_ledger) {
                    Ok(t) => t,
                    Err(e) => {
                        util::write_to_wstream(wstream, format!("{}\r\n", e).as_bytes())?;
                        return Ok(());
                    }
                };

                let expiry = expiry.parse::<u32>();

                if let Ok(time) = expiry {
                    db.insert(
                        &key,
                        (
                            value.into_bytes(),
                            Some(SystemTime::now() + std::time::Duration::from_secs(time.into())),
                        ),
                    );
                }

                util::write_to_wstream(wstream, b"+OK\r\n")?;

                return Ok(());
            }
            _ => {
                util::write_to_wstream(wstream, b"-ERR Protocol Error\r\n")?;
                return Ok(());
            }
        }
    }

    util::write_to_wstream(wstream, b"-ERR Protocol Error\r\n")?;
    util::cleanup(count_ledger, reader_lines);
    return Ok(());
}
