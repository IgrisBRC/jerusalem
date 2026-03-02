use std::{ascii::AsciiExt, string::ParseError};

use crate::wish::Sin;

pub fn find_crlf(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|w| w == b"\r\n")
}

pub fn bytes_to_i32(bytes: &[u8]) -> Result<i32, Sin> {
    if bytes.is_empty() {
        return Err(Sin::ParseError);
    }

    let (is_neg, start) = if bytes[0] == b'-' {
        (true, 1)
    } else {
        (false, 0)
    };
    if start == bytes.len() {
        return Err(Sin::ParseError);
    }

    let mut result = 0i32;
    for &b in &bytes[start..] {
        if !b.is_ascii_digit() {
            return Err(Sin::ParseError);
        }

        result = (result * 10) + (b - b'0') as i32;
    }

    Ok(if is_neg { -result } else { result })
}


pub fn bytes_to_u64(bytes: &[u8]) -> Result<u64, Sin> {
    if bytes.is_empty() {
        return Err(Sin::ParseError);
    }

    let start = 0;

    if start == bytes.len() {
        return Err(Sin::ParseError);
    }

    let mut result = 0u64;

    for &b in &bytes[start..] {
        if !b.is_ascii_digit() {
            return Err(Sin::ParseError);
        }

        result = (result * 10) + (b - b'0') as u64;
    }

    Ok(result)
}


pub fn bytes_to_i64(bytes: &[u8]) -> Result<i64, Sin> {
    if bytes.is_empty() {
        return Err(Sin::ParseError);
    }

    let (is_neg, start) = if bytes[0] == b'-' {
        (true, 1)
    } else {
        (false, 0)
    };
    if start == bytes.len() {
        return Err(Sin::ParseError);
    }

    let mut result = 0i64;
    for &b in &bytes[start..] {
        if !b.is_ascii_digit() {
            return Err(Sin::ParseError);
        }

        result = (result * 10) + (b - b'0') as i64;
    }

    Ok(if is_neg { -result } else { result })
}


pub fn bytes_to_usize(bytes: &[u8]) -> Result<usize, Sin> {
    if bytes.is_empty() {
        return Err(Sin::ParseError);
    }

    let start = 0;

    if start == bytes.len() {
        return Err(Sin::ParseError);
    }

    let mut result = 0usize;

    for &b in &bytes[start..] {
        if !b.is_ascii_digit() {
            return Err(Sin::ParseError);
        }

        result = (result * 10) + (b - b'0') as usize;
    }

    Ok(result)
}
