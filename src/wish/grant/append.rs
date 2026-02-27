use std::sync::mpsc::Sender;

use mio::Token;

use crate::{
    temple::{Temple, Value},
    wish::{
        Command, Response, Sacrilege, Sin,
        grant::{Decree, Gift},
    },
};

pub fn append(
    terms: Vec<Vec<u8>>,
    temple: &mut Temple,
    tx: Sender<Decree>,
    token: Token,
) -> Result<(), Sin> {
    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    if let (Some(key), Some(value)) = (terms_iter.next(), terms_iter.next()) {
        temple.append(key, Value::String(value), tx, token);
    } else if tx
        .send(Decree::Deliver(Gift {
            token,
            response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::APPEND)),
        }))
        .is_err()
    {
        eprintln!("angel panicked");
    }

    Ok(())
}
