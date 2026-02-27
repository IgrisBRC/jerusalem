use std::sync::mpsc::Sender;

use mio::Token;

use crate::{
    temple::Temple,
    wish::{
        grant::{Decree, Gift}, util::bytes_to_i32, Command, Response, Sacrilege, Sin
    },
};

pub fn lindex(
    terms: Vec<Vec<u8>>,
    temple: &mut Temple,
    tx: Sender<Decree>,
    token: Token,
) -> Result<(), Sin> {
    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    if let (Some(key), Some(index)) = (terms_iter.next(), terms_iter.next()) {
        if let Some(index) = bytes_to_i32(&index) {
            temple.lindex(tx, key, index, token);
        } else {
            if tx
                .send(Decree::Deliver(Gift {
                    token,
                    response: Response::Error(Sacrilege::IncorrectUsage(Command::LINDEX)),
                }))
                .is_err()
            {
                eprintln!("angel panicked");
            };
        }
    } else {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::LINDEX)),
            }))
            .is_err()
        {
            eprintln!("angel panicked");
        };
    }

    Ok(())
}
