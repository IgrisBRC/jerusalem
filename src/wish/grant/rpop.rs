use std::{sync::mpsc::Sender, time::SystemTime};

use mio::Token;

use crate::{
    temple::Temple,
    wish::{
        Command, Response, Sacrilege, Sin,
        grant::{Decree, Gift},
        util::bytes_to_usize,
    },
};

pub fn rpop(terms: Vec<Vec<u8>>, temple: &mut Temple, tx: Sender<Decree>, token: Token) {
    if terms.len() > 3 {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::RPOP)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    if let Some(key) = terms_iter.next() {
        if let Some(count) = terms_iter.next() {
            if let Ok(count) = bytes_to_usize(&count) {
                temple.rpop_m(tx, key, count, token, SystemTime::now());

                return;
            }

            if tx
                .send(Decree::Deliver(Gift {
                    token,
                    response: Response::Error(Sacrilege::IncorrectUsage(Command::RPOP)),
                }))
                .is_err()
            {
                eprintln!("angel panicked");
            };

            return;
        }

        temple.rpop(tx, key, token, SystemTime::now());
    } else if tx
        .send(Decree::Deliver(Gift {
            token,
            response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::RPOP)),
        }))
        .is_err()
    {
        eprintln!("angel panicked");
    }
}
