use crate::temple::Temple;
use mio::net::TcpStream;
use std::io::Read;

pub enum Phase {
    Idle,
    AwaitingTermCount,
    GraspingMarker,
    AwaitingBulkStringLength,
    AwaitingBulkString(usize),
}

pub struct Virtue {
    backlog: Vec<u8>,
    terms: Vec<Vec<u8>>,
    expected_terms: usize,
    phase: Phase,
}

impl Virtue {
    fn new() -> Self {
        Self {
            backlog: Vec::with_capacity(2048),
            terms: Vec::new(),
            expected_terms: 0,
            phase: Phase::Idle,
        }
    }
}

pub struct Pilgrim {
    pub stream: TcpStream,
    pub virtue: Option<Virtue>,
}

pub enum Sin {
    Utf8Error,
    ParseError,
    Disconnected,
    Blasphemy,
}

mod grant;
mod util;

pub fn wish(pilgrim: &mut Pilgrim, mut temple: Temple) -> Result<(), Sin> {
    let virtue = pilgrim.virtue.get_or_insert_with(Virtue::new);

    let mut buffer = [0; 1024];

    loop {
        match pilgrim.stream.read(&mut buffer) {
            Ok(0) => return Err(Sin::Disconnected),
            Ok(bytes_read) => {
                virtue.backlog.extend_from_slice(&buffer[..bytes_read]);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => return Err(Sin::Disconnected),
        }
    }

    loop {
        if virtue.backlog.is_empty() {
            break;
        }

        match virtue.phase {
            Phase::Idle => {
                if virtue.backlog[0] == b'*' {
                    virtue.phase = Phase::AwaitingTermCount;
                }
                virtue.backlog.drain(..1);
            }
            Phase::AwaitingTermCount => {
                if let Some(index) = util::find_crlf(&virtue.backlog) {
                    virtue.expected_terms = std::str::from_utf8(&virtue.backlog[..index])
                        .map_err(|_| Sin::Utf8Error)?
                        .parse()
                        .map_err(|_| Sin::ParseError)?;

                    if virtue.expected_terms == 0 {
                        return Err(Sin::Blasphemy);
                    }

                    virtue.phase = Phase::GraspingMarker;
                    virtue.backlog.drain(..index + 2);
                } else {
                    break;
                }
            }
            Phase::GraspingMarker => {
                if virtue.backlog[0] == b'$' {
                    virtue.phase = Phase::AwaitingBulkStringLength;
                } else {
                    return Err(Sin::Blasphemy);
                }

                virtue.backlog.drain(..1);
            }
            Phase::AwaitingBulkStringLength => {
                if let Some(index) = util::find_crlf(&virtue.backlog) {
                    let bulk_string_length = std::str::from_utf8(&virtue.backlog[..index])
                        .map_err(|_| Sin::Utf8Error)?
                        .parse()
                        .map_err(|_| Sin::ParseError)?;

                    virtue.phase = Phase::AwaitingBulkString(bulk_string_length);
                    virtue.backlog.drain(..index + 2);
                } else {
                    break;
                }
            }
            Phase::AwaitingBulkString(characters_remaining) => {
                if virtue.backlog.len() >= characters_remaining + 2 {
                    if virtue.backlog[characters_remaining] != b'\r'
                        || virtue.backlog[characters_remaining + 1] != b'\n'
                    {
                        return Err(Sin::Blasphemy);
                    }

                    let term = std::str::from_utf8(&virtue.backlog[..characters_remaining])
                        .map_err(|_| Sin::Utf8Error)?;

                    virtue.terms.push(term.into());

                    virtue.backlog.drain(..characters_remaining + 2);
                    virtue.phase = Phase::GraspingMarker;

                    if virtue.terms.len() == virtue.expected_terms {
                        // println!("Wish received {:?}", virtue.terms);

                        grant::grant(&virtue.terms, &mut pilgrim.stream, &mut temple)?;

                        virtue.terms.clear();
                        virtue.phase = Phase::Idle;
                    }
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}
