

use std::{io::{BufReader, BufWriter, Write}, net::TcpStream};

pub fn write_to_wstream(wstream: &mut BufWriter<&TcpStream>, buf: &[u8]) -> Result<(), String> {
    wstream
        .write_all(buf)
        .map_err(|_| "Failed to write to stream")?;
    wstream.flush().map_err(|_| "Failed to flush stream")?;

    Ok(())
}

pub fn cleanup(count_ledger: &mut i32, reader_lines: &mut std::io::Lines<BufReader<&TcpStream>>) {
    while *count_ledger > 0 {
        let _ = reader_lines.next();
        let _ = reader_lines.next();
        *count_ledger -= 1;
    }
}

pub fn validate_and_get_next_term(
    reader_lines: &mut std::io::Lines<BufReader<&TcpStream>>,
    count_ledger: &mut i32,
) -> Result<String, String> {
    let line = reader_lines
        .next()
        .ok_or("Connnection closed unexpectedly?")?
        .map_err(|_| "Failed to read line")?;

    let term_length = if line.starts_with('$') {
        line[1..].trim().parse::<usize>().map_err(|_| "Parsing error of some kind idk, don't blame me I just used rust's `parse` function, it just returned an error, okay? And no I am not gonna write my own parse function.")?
    } else {
        let _ = reader_lines
            .next()
            .ok_or("Connnection closed unexpectedly?")?
            .map_err(|_| "Failed to read line")?;
        *count_ledger -= 1;
        Err("-ERR Protocol Error: ")?
    };

    let term = reader_lines
        .next()
        .ok_or("Connnection closed unexpectedly?")?
        .map_err(|_| "Failed to read line")?;

    *count_ledger -= 1;

    if term.len() != term_length as usize {
        Err("-ERR Protocol Error: ")?
    }

    Ok(term)
}
