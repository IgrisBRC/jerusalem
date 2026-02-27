pub fn find_crlf(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|w| w == b"\r\n")
}

pub fn bytes_to_i32(bytes: &[u8]) -> Option<i32> {
    if bytes.is_empty() {
        return None;
    }

    let (is_neg, start) = if bytes[0] == b'-' {
        (true, 1)
    } else {
        (false, 0)
    };
    if start == bytes.len() {
        return None;
    }

    let mut result = 0i32;
    for &b in &bytes[start..] {
        if !b.is_ascii_digit() {
            return None;
        }

        result = (result * 10) + (b - b'0') as i32;
    }

    Some(if is_neg { -result } else { result })
}

