use std::sync::mpsc::Sender;

use mio::Token;

use crate::{
    temple::Temple,
    wish::{
        Command, Response, Sacrilege, Sin,
        grant::{Decree, Gift},
    },
};

pub fn llen(
    terms: Vec<Vec<u8>>,
    temple: &mut Temple,
    tx: Sender<Decree>,
    token: Token,
) -> Result<(), Sin> {
    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    if let Some(key) = terms_iter.next() {
        temple.llen(tx, key, token);
    } else {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::LLEN)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        };
    }

    Ok(())
}
