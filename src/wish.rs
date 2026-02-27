use crate::{temple::Temple, wish::grant::Decree};
use mio::{Token, net::TcpStream};
use std::{io::Read, sync::mpsc::Sender};

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

pub enum Command {
    SET,
    GET,
    EX,
    INCR,
    DECR,
    APPEND,
    STRLEN,
    EXISTS,
    DEL,
    HSET,
    HGET,
    HMGET,
    HDEL,
    HEXISTS,
    HLEN,
    LPUSH,
    LPOP,
    RPUSH,
    RPOP,
    LLEN,
    LRANGE,
    LINDEX,
    LSET,
    LREM
}

pub enum Sacrilege {
    IncorrectNumberOfArguments(Command),
    IncorrectUsage(Command),
    UnknownCommand,
}

pub enum InfoType {
    Ok,
    Pong,
}

pub enum Response {
    Error(Sacrilege),
    Info(InfoType),
    BulkString(Option<Vec<u8>>),
    BulkStringArray(Option<Vec<Option<Vec<u8>>>>),
    Amount(u32),
    Number(i64),
    Length(usize),
}

pub struct Pilgrim {
    pub stream: TcpStream,
    pub virtue: Option<Virtue>,
    pub tx: Sender<Decree>,
}

#[derive(Debug)]
pub enum Sin {
    Utf8Error,
    ParseError,
    Disconnected,
    Blasphemy,
}

pub mod grant;
mod util;

pub fn wish(pilgrim: &mut Pilgrim, mut temple: Temple, token: Token) -> Result<(), Sin> {
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

                    let term = &virtue.backlog[..characters_remaining];

                    virtue.terms.push(term.into());

                    virtue.backlog.drain(..characters_remaining + 2);
                    virtue.phase = Phase::GraspingMarker;

                    if virtue.terms.len() == virtue.expected_terms {
                        let terms_to_grant = std::mem::take(&mut virtue.terms);

                        grant::grant(terms_to_grant, &mut temple, pilgrim.tx.clone(), token)?;

                        virtue.phase = Phase::Idle;
                        virtue.expected_terms = 0;
                    }
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}
