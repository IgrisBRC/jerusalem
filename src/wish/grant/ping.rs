use crate::wish::{Command, Sacrilege};
use std::sync::mpsc::Sender;

use mio::Token;

use crate::wish::{
    InfoType, Response, Sin,
    grant::{Decree, Gift},
};

pub fn ping(terms: Vec<Vec<u8>>, tx: Sender<Decree>, token: Token) {
    if terms.len() != 1 {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::PING)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    if tx
        .send(Decree::Deliver(Gift {
            token,
            response: Response::Info(InfoType::Pong),
        }))
        .is_err()
    {
        eprintln!("angel panicked");
    };
}
