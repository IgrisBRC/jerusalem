use std::{sync::mpsc::Sender, time::SystemTime};

use mio::Token;

use crate::{
    temple::{Temple, Value},
    wish::{
        Command, Response, Sacrilege, Sin,
        grant::{Decree, Gift},
    },
};

pub fn unsubscribe(terms: Vec<Vec<u8>>, temple: &mut Temple, tx: Sender<Decree>, token: Token) {
    let mut terms_iter = terms.into_iter();
    terms_iter.next();

    temple.unsubscribe(tx, token, terms_iter.collect());
}
