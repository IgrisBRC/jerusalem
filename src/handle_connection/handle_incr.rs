use std::{
    io::{BufReader, BufWriter, Lines},
    net::TcpStream,
};

use crate::handle_connection::util;
use crate::memory_database::MemoryDatabase;

pub fn handle_incr(
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
        if let Ok(value) = String::from_utf8_lossy(&value_bytes).parse::<i64>() {
            let new_value_because_apparently_using_value_plus_1_is_just_too_many_cpu_cycles =
                value + 1;

            util::write_to_wstream(
                wstream,
                format!(
                    ":{}\r\n",
                    new_value_because_apparently_using_value_plus_1_is_just_too_many_cpu_cycles
                )
                .as_bytes(),
            )?;

            db.insert(
                &key,
                (
                    new_value_because_apparently_using_value_plus_1_is_just_too_many_cpu_cycles
                        .to_string()
                        .trim()
                        .as_bytes()
                        .to_vec(),
                    None,
                ),
            );
        } else {
            util::write_to_wstream(
                wstream,
                b"-ERR invalid use of INCR, value not in number form.\r\n",
            )?;
        }
    } else {
        db.insert(&key, ("1".as_bytes().to_vec(), None));
        util::write_to_wstream(wstream, b":1\r\n")?;
    }

    Ok(())
}
