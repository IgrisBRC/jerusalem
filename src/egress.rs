use mio::net::TcpStream;
use std::io::Write;

use crate::wish::{InfoType, Response, Sin, grant::Gift};

pub fn egress(stream: &mut TcpStream, gift: Gift, response: &mut Vec<u8>) -> Result<(), Sin> {
    response.clear();
    let mut itoa_buf = itoa::Buffer::new();

    match gift.response {
        Response::Info(InfoType::Ok) => {
            response.extend_from_slice(b"+OK\r\n");
        }
        Response::Info(InfoType::Pong) => {
            response.extend_from_slice(b"+PONG\r\n");
        }
        Response::BulkString(bulk_string) => match bulk_string {
            Some(value) => {
                response.push(b'$');
                response.extend_from_slice(itoa_buf.format(value.len()).as_bytes());
                response.extend_from_slice(b"\r\n");
                response.extend_from_slice(&value);
                response.extend_from_slice(b"\r\n");
            }
            None => {
                response.extend_from_slice(b"$-1\r\n");
            }
        },
        Response::BulkStringArray(bulk_string_array) => match bulk_string_array {
            Some(bulk_string_array) => {
                response.push(b'*');
                response.extend_from_slice(itoa_buf.format(bulk_string_array.len()).as_bytes());
                response.extend_from_slice(b"\r\n");

                for bulk_string in bulk_string_array {
                    match bulk_string {
                        Some(value) => {
                            response.push(b'$');
                            response.extend_from_slice(itoa_buf.format(value.len()).as_bytes());
                            response.extend_from_slice(b"\r\n");
                            response.extend_from_slice(&value);
                            response.extend_from_slice(b"\r\n");
                        }
                        None => {
                            response.extend_from_slice(b"$-1\r\n");
                        }
                    }
                }
            }
            None => {
                response.extend_from_slice(b"$-1\r\n");
            }
        },
        Response::Amount(amount) => {
            response.push(b':');
            response.extend_from_slice(itoa_buf.format(amount).as_bytes());
            response.extend_from_slice(b"\r\n");
        }
        Response::Number(number) => {
            response.push(b':');
            response.extend_from_slice(itoa_buf.format(number).as_bytes());
            response.extend_from_slice(b"\r\n");
        }
        Response::Length(length) => {
            response.push(b':');
            response.extend_from_slice(itoa_buf.format(length).as_bytes());
            response.extend_from_slice(b"\r\n");
        }
        Response::NumberOfSubscribedChannels(event, number) => {
            response.extend_from_slice(b"*3\r\n$9\r\nsubscribe\r\n$");
            response.extend_from_slice(itoa_buf.format(event.len()).as_bytes());
            response.extend_from_slice(b"\r\n");
            response.extend_from_slice(&event);
            response.extend_from_slice(b"\r\n:");

            response.extend_from_slice(itoa_buf.format(number).as_bytes());
            response.extend_from_slice(b"\r\n");
        }
        Response::Error(_) => {
            response.extend_from_slice(b"-ERR Some error occured, and because I was too impatient to test this I didn't really wanna write out the logic to match my way through to figure out which error has happened here.\r\n");
        }
    }

    stream.write_all(&response).map_err(|_| Sin::Disconnected)?;

    Ok(())
}
