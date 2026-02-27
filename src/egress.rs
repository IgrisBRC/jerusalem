use mio::net::TcpStream;
use std::io::Write;

use crate::wish::{InfoType, Response, Sin, grant::Gift};

pub fn egress(stream: &mut TcpStream, gift: Gift) -> Result<(), Sin> {
    let mut response: Vec<u8> = Vec::new();

    match gift.response {
        Response::Info(InfoType::Ok) => {
            response.append(&mut b"+OK\r\n".to_vec());
        }
        Response::Info(InfoType::Pong) => {
            response.append(&mut b"+PONG\r\n".to_vec());
        }
        Response::BulkString(bulk_string) => match bulk_string {
            Some(mut value) => {
                response.append(&mut format!("${}\r\n", value.len()).into_bytes());
                response.append(&mut value);
                response.append(&mut "\r\n".as_bytes().to_vec());
            }
            None => {
                response.append(&mut b"$-1\r\n".to_vec());
            }
        },
        Response::BulkStringArray(bulk_string_array) => match bulk_string_array {
            Some(bulk_string_array) => {
                response.append(&mut format!("*{}\r\n", bulk_string_array.len()).into_bytes());

                for bulk_string in bulk_string_array {
                    match bulk_string {
                        Some(mut value) => {
                            response.append(&mut format!("${}\r\n", value.len()).into_bytes());
                            response.append(&mut value);
                            response.append(&mut "\r\n".as_bytes().to_vec());
                        }
                        None => {
                            response.append(&mut b"$-1\r\n".to_vec());
                        }
                    }
                }
            }
            None => {
                response.append(&mut b"$-1\r\n".to_vec());
            }
        },
        Response::Amount(amount) => {
            response.append(&mut format!(":{}\r\n", amount).into_bytes());
        }
        Response::Number(number) => {
            let mut number_string = number.to_string().into_bytes();

            response.push(b':');
            response.append(&mut number_string);
            response.append(&mut "\r\n".as_bytes().to_vec());
        }
        Response::Length(length) => {
            response.append(&mut format!(":{}\r\n", length).into_bytes());
        }
        Response::Error(_) => {
            response.append(&mut b"-ERR Some error occured, and because I was too impatient to test this I didn't really wanna write out the logic to match my way through to figure out which error has happened here.\r\n".to_vec());
        }
    }

    stream.write_all(&response).map_err(|_| Sin::Disconnected)?;

    Ok(())
}
