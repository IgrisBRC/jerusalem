use mio::Token;

use crate::{
    temple::{Temple, Value},
    wish::{
        Command, Response, Sacrilege, Sin,
        grant::{Decree, Gift},
    },
};

use std::{sync::mpsc::Sender, time::SystemTime};

pub fn set(
    terms: Vec<Vec<u8>>,
    temple: &mut Temple,
    tx: Sender<Decree>,
    token: Token,
) -> Result<(), Sin> {
    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    if let (Some(key), Some(value)) = (terms_iter.next(), terms_iter.next()) {
        match terms_iter.next() {
            Some(command) => {
                if let Ok(command) = std::str::from_utf8(&command) {
                    if let ("EX", Some(expiry)) =
                        (command.to_uppercase().as_str(), terms_iter.next())
                    {
                        if let Ok(expiry) = std::str::from_utf8(&expiry) {
                            if let Ok(expiry) = expiry.parse::<u64>() {
                                temple.set(
                                    key,
                                    (
                                        Value::String(value),
                                        Some(
                                            SystemTime::now()
                                                + std::time::Duration::from_secs(expiry),
                                        ),
                                    ),
                                    tx,
                                    token,
                                );

                                return Ok(());
                            }
                        }

                        if tx
                            .send(Decree::Deliver(Gift {
                                token,
                                response: Response::Error(Sacrilege::IncorrectUsage(Command::SET)),
                            }))
                            .is_err()
                        {
                            eprintln!("angel panicked")
                        };

                        return Ok(());
                    } else {
                        if tx
                            .send(Decree::Deliver(Gift {
                                token,
                                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(
                                    Command::SET,
                                )),
                            }))
                            .is_err()
                        {
                            eprintln!("angel panicked")
                        };

                        return Ok(());
                    }
                } else {
                    if tx
                        .send(Decree::Deliver(Gift {
                            token,
                            response: Response::Error(Sacrilege::IncorrectNumberOfArguments(
                                Command::SET,
                            )),
                        }))
                        .is_err()
                    {
                        eprintln!("angel panicked")
                    };

                    return Ok(());
                }
            }
            None => {
                temple.set(key, (Value::String(value), None), tx, token);
                return Ok(());
            }
        }
    } else {
        if tx
            .send(Decree::Deliver(Gift {
                token,
                response: Response::Error(Sacrilege::IncorrectNumberOfArguments(Command::SET)),
            }))
            .is_err()
        {
            eprintln!("angel panicked")
        };

        return Ok(());
    }
}
