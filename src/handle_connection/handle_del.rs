use std::{
    io::{BufReader, BufWriter, Lines},
    net::TcpStream,
};

use crate::handle_connection::util;
use crate::memory_database::MemoryDatabase;

pub fn handle_del(
    db: &mut MemoryDatabase,
    reader_lines: &mut Lines<BufReader<&TcpStream>>,
    count: usize,
    wstream: &mut BufWriter<&TcpStream>,
    count_ledger: &mut i32,
) -> Result<(), String> {
    if count < 2 {
        util::write_to_wstream(wstream, b"-ERR fuk are you playin at?\r\n")?;
        return Ok(());
    }

    let mut keys_deleted = 0;

    while *count_ledger > 0 {
        let key = match util::validate_and_get_next_term(reader_lines, count_ledger) {
            Ok(t) => t,
            Err(e) => {
                util::write_to_wstream(wstream, format!("{}\r\n", e).as_bytes())?;
                return Ok(());
            }
        };

        if let Some(_) = db.remove(&key) {
            keys_deleted += 1;
        }
    }

    util::write_to_wstream(wstream, format!(":{}\r\n", keys_deleted).as_bytes())?;

    Ok(())
}
