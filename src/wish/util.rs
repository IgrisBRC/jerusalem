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
        if b < b'0' || b > b'9' {
            return None;
        }

        result = (result * 10) + (b - b'0') as i32;
    }

    Some(if is_neg { -result } else { result })
}

// The one I made
// fn bytes_to_i32(bytes: &[u8]) -> Option<i32> {
//     if bytes.is_empty() {
//         return None;
//     }
//
//     let bytes_len = bytes.len();
//
//     let mut result: i32 = 0;
//     let mut place = 1;
//
//     for idx in (1..bytes_len).rev() {
//         if bytes[idx] < b'0' || bytes[idx] > b'9' {
//             return None;
//         }
//
//         result += place * (bytes[idx] - b'0') as i32;
//         place *= 10;
//     }
//
//     if bytes[0] == b'-' {
//         return Some(-result);
//     } else if bytes[0] >= b'0' && bytes[0] <= b'9' {
//         result += place * (bytes[0] - b'0') as i32;
//         return Some(result);
//     } else {
//         return None;
//     }
// }
