use std::sync::mpsc::Sender;

use mio::Token;

use crate::wish::{grant::{Decree, Gift}, InfoType, Response, Sin};

pub fn ping(tx: Sender<Decree>, token: Token) -> Result<(), Sin> {
    if tx.send(Decree::Deliver(Gift {
        token,
        response: Response::Info(InfoType::Pong),
    })).is_err() {
        eprintln!("angel panicked");
    };

    Ok(())
}
