use std::{sync::mpsc::Sender, time::SystemTime};

use mio::Token;

use crate::{
    temple::Temple,
    wish::{
        Command, Response, Sacrilege, Sin,
        grant::{Decree, Gift},
        util::bytes_to_i32,
    },
};

pub fn lindex(terms: Vec<Vec<u8>>, temple: &mut Temple, tx: Sender<Decree>, token: Token) {
    if terms.len() != 3 {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::LINDEX)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    if let (Some(key), Some(index)) = (terms_iter.next(), terms_iter.next()) {
        if let Ok(index) = bytes_to_i32(&index) {
            temple.lindex(tx, key, index, token, SystemTime::now());
        } else if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectUsage(Command::LINDEX)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        }
    } else if tx
        .send(Decree::Deliver(Gift {
            token,
            response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::LINDEX)),
        }))
        .is_err()
    {
        eprintln!("angel panicked");
    }
}
