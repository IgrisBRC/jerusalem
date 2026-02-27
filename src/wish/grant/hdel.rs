use crate::{
    temple::Temple,
    wish::{
        Command, Response, Sacrilege, Sin,
        grant::{Decree, Gift},
    },
};
use mio::Token;
use std::sync::mpsc::Sender;

pub fn hdel(
    terms: Vec<Vec<u8>>,
    temple: &mut Temple,
    tx: Sender<Decree>,
    token: Token,
) -> Result<(), Sin> {
    if terms.len() < 3 {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::HDEL)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        };

        return Ok(());
    }

    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    if let Some(key) = terms_iter.next() {
        temple.hdel(tx, key, terms_iter.collect(), token);
    }

    Ok(())
}
