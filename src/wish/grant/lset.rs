use std::sync::mpsc::Sender;

use mio::Token;

use crate::{
    temple::Temple,
    wish::{
        Command, Response, Sacrilege, Sin,
        grant::{Decree, Gift},
        util::bytes_to_i32,
    },
};

pub fn lset(
    terms: Vec<Vec<u8>>,
    temple: &mut Temple,
    tx: Sender<Decree>,
    token: Token,
) -> Result<(), Sin> {
    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    let (Some(key), Some(index), Some(element)) =
        (terms_iter.next(), terms_iter.next(), terms_iter.next())
    else {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::LSET)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        };

        return Ok(());
    };

    let Some(index) = bytes_to_i32(&index) else {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectUsage(Command::LSET)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        };

        return Ok(());
    };

    temple.lset(tx, key, index, element, token);

    Ok(())
}
