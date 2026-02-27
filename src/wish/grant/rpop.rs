
use std::sync::mpsc::Sender;

use mio::Token;

use crate::{
    temple::Temple,
    wish::{
        Command, Response, Sacrilege, Sin,
        grant::{Decree, Gift},
    },
};

pub fn rpop(
    terms: Vec<Vec<u8>>,
    temple: &mut Temple,
    tx: Sender<Decree>,
    token: Token,
) -> Result<(), Sin> {
    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    if let Some(key) = terms_iter.next() {
        if let Some(count) = terms_iter.next() {
            if let Ok(count) = std::str::from_utf8(&count)
                && let Ok(count) = count.parse::<usize>() {
                    temple.rpop_m(tx, key, count, token);

                    return Ok(());
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

            return Ok(());
        }

        temple.rpop(tx, key, token);
    } else if tx
        .send(Decree::Deliver(Gift {
            token,
            response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::RPOP)),
        }))
        .is_err()
    {
        eprintln!("angel panicked");
    }

    Ok(())
}
