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

pub fn lrange(
    terms: Vec<Vec<u8>>,
    temple: &mut Temple,
    tx: Sender<Decree>,
    token: Token,
) -> Result<(), Sin> {
    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    let (Some(key), Some(starting_index), Some(ending_index)) =
        (terms_iter.next(), terms_iter.next(), terms_iter.next())
    else {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::LRANGE)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        }

        return Ok(());
    };

    let (Some(starting_index), Some(ending_index)) =
        (bytes_to_i32(&starting_index), bytes_to_i32(&ending_index))
    else {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectUsage(Command::LRANGE)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        }

        return Ok(());
    };

    temple.lrange(tx, key, starting_index, ending_index, token);

    Ok(())
}
