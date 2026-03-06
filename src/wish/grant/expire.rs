use std::{sync::mpsc::Sender, time::SystemTime};

use mio::Token;

use crate::{
    temple::Temple,
    wish::{
        Command, Response, Sacrilege, Sin,
        grant::{Decree, Gift},
        util::bytes_to_u64,
    },
};

pub fn expire(terms: Vec<Vec<u8>>, temple: &mut Temple, tx: Sender<Decree>, token: Token) {
    if terms.len() != 3 {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::EXPIRE)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        }

        return;
    }

    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    let (Some(key), Some(expiry)) = (terms_iter.next(), terms_iter.next()) else {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::EXPIRE)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        }

        return;
    };

    let Ok(expiry) = bytes_to_u64(&expiry) else {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectUsage(Command::EXPIRE)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        }

        return;
    };

    let now = SystemTime::now();

    temple.expire(
        tx,
        key,
        now + std::time::Duration::from_secs(expiry),
        token,
        now,
    );
}
