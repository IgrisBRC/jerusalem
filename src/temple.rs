use std::collections::hash_map::Entry;
use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::Sender;
use std::{collections::HashMap, time::SystemTime};

use mio::Token;

use crate::wish::grant::{Decree, Gift};
use crate::wish::{Command, InfoType, Response, Sacrilege};

#[derive(Clone)]
pub enum Value {
    String(Vec<u8>),
    List(VecDeque<Vec<u8>>),
    Hash(HashMap<Vec<u8>, Vec<u8>>),
    Set(HashSet<Vec<u8>>),
}

pub struct Soul(HashMap<Vec<u8>, (Value, Option<SystemTime>)>);

impl Default for Soul {
    fn default() -> Self {
        Self::new()
    }
}

impl Soul {
    pub fn new() -> Self {
        Soul(HashMap::new())
    }

    pub fn get(&mut self, key: Vec<u8>) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(occupied) => {
                let (value, expiry) = occupied.get();

                if let Some(time) = expiry
                    && *time < SystemTime::now() {
                        occupied.remove();
                        return Ok(None);
                    }

                if let Value::String(value) = value {
                    Ok(Some(value.to_vec()))
                } else {
                    Err(Sacrilege::IncorrectUsage(Command::GET))
                }
            }
            Entry::Vacant(_) => Ok(None),
        }
    }

    pub fn set(&mut self, key: Vec<u8>, val: (Value, Option<SystemTime>)) {
        self.0.insert(key, val);
    }

    pub fn append(&mut self, key: Vec<u8>, incoming_value: Value) -> usize {
        let Value::String(mut incoming_value) = incoming_value else {
            return 0;
        };

        let entry = self.0.remove(&key);

        match entry {
            Some((Value::String(mut existing_value), Some(time))) if time >= SystemTime::now() => {
                existing_value.append(&mut incoming_value);

                let length = existing_value.len();

                self.0
                    .insert(key, (Value::String(existing_value), Some(time)));

                length
            }
            Some((Value::String(mut existing_value), None)) => {
                existing_value.append(&mut incoming_value);

                let length = existing_value.len();

                self.0.insert(key, (Value::String(existing_value), None));

                length
            }
            Some((_, _)) => 0,
            None => {
                let length = incoming_value.len();

                self.0.insert(key, (Value::String(incoming_value), None));

                length
            }
        }
    }

    pub fn incr(&mut self, key: Vec<u8>) -> Result<i64, Sacrilege> {
        let entry = self.0.remove(&key);

        match entry {
            Some((Value::String(existing_value), expiry)) => {
                if let Ok(existing_value) = std::str::from_utf8(&existing_value)
                    && let Ok(existing_value) = existing_value.parse::<i64>() {
                        self.0.insert(
                            key,
                            (
                                Value::String((existing_value + 1).to_string().into_bytes()),
                                expiry,
                            ),
                        );

                        return Ok(existing_value + 1);
                    }

                self.0.insert(key, (Value::String(existing_value), expiry));
                Err(Sacrilege::IncorrectUsage(Command::INCR))
            }

            Some((other_value, expiry)) => {
                self.0.insert(key, (other_value, expiry));
                Err(Sacrilege::IncorrectUsage(Command::INCR))
            }

            None => {
                let initial = Value::String(b"1".to_vec());
                self.0.insert(key, (initial, None));

                Ok(1)
            }
        }
    }

    pub fn decr(&mut self, key: Vec<u8>) -> Result<i64, Sacrilege> {
        let entry = self.0.remove(&key);

        match entry {
            Some((Value::String(existing_value), expiry)) => {
                if let Ok(existing_value) = std::str::from_utf8(&existing_value)
                    && let Ok(existing_value) = existing_value.parse::<i64>() {
                        self.0.insert(
                            key,
                            (
                                Value::String((existing_value - 1).to_string().into_bytes()),
                                expiry,
                            ),
                        );

                        return Ok(existing_value - 1);
                    }

                self.0.insert(key, (Value::String(existing_value), expiry));
                Err(Sacrilege::IncorrectUsage(Command::DECR))
            }

            Some((existing_value, expiry)) => {
                self.0.insert(key, (existing_value, expiry));
                Err(Sacrilege::IncorrectUsage(Command::DECR))
            }

            None => {
                let initial = Value::String(b"-1".to_vec());
                self.0.insert(key, (initial, None));

                Ok(-1)
            }
        }
    }

    pub fn strlen(&self, key: Vec<u8>) -> Result<usize, Sacrilege> {
        match self.0.get(&key) {
            Some((Value::String(value), _)) => Ok(value.len()),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::STRLEN)),
            None => Ok(0),
        }
    }

    pub fn del(&mut self, keys: Vec<Vec<u8>>) -> u32 {
        let mut number_of_entries_deleted = 0;

        for key in keys {
            if self.0.remove(&key).is_some() {
                number_of_entries_deleted += 1;
            }
        }

        number_of_entries_deleted
    }

    pub fn exists(&self, keys: Vec<Vec<u8>>) -> u32 {
        let mut number_of_entries_that_exist = 0;

        for key in keys {
            if self.0.contains_key(&key) {
                number_of_entries_that_exist += 1;
            }
        }

        number_of_entries_that_exist
    }

    pub fn hset(
        &mut self,
        key: Vec<u8>,
        field_value_pairs: Vec<(Vec<u8>, Vec<u8>)>,
    ) -> Result<u32, Sacrilege> {
        match self.0.get_mut(&key) {
            Some((Value::Hash(map), _)) => {
                let mut new_values_added = 0;

                for field_value_pair in field_value_pairs {
                    let (field, value) = field_value_pair;

                    if map.insert(field, value).is_none() {
                        new_values_added += 1;
                    }
                }

                Ok(new_values_added)
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::HSET)),
            None => {
                let mut map = HashMap::new();
                let mut new_values_added = 0;

                for field_value_pair in field_value_pairs {
                    let (field, value) = field_value_pair;

                    map.insert(field, value);
                    new_values_added += 1;
                }

                self.0.insert(key, (Value::Hash(map), None));

                Ok(new_values_added)
            }
        }
    }

    pub fn hget(&mut self, key: Vec<u8>, field: Vec<u8>) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.0.get(&key) {
            Some((Value::Hash(map), _)) => Ok(map.get(&field).cloned()),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::HGET)),
            None => Ok(None),
        }
    }

    pub fn hmget(
        &mut self,
        key: Vec<u8>,
        fields: Vec<Vec<u8>>,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        let mut values = Vec::new();

        match self.0.get(&key) {
            Some((Value::Hash(map), _)) => {
                for field in fields {
                    values.push(map.get(&field).cloned());
                }

                Ok(Some(values))
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::HGET)),
            None => Ok(None),
        }
    }

    pub fn hdel(&mut self, key: Vec<u8>, fields: Vec<Vec<u8>>) -> Result<u32, Sacrilege> {
        let mut amount_of_deleted_values = 0;

        match self.0.get_mut(&key) {
            Some((Value::Hash(map), _)) => {
                for field in fields {
                    if map.remove(&field).is_some() {
                        amount_of_deleted_values += 1
                    }
                }

                Ok(amount_of_deleted_values)
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::HDEL)),
            None => Ok(0),
        }
    }

    pub fn hexists(&mut self, key: Vec<u8>, field: Vec<u8>) -> Result<u32, Sacrilege> {
        match self.0.get(&key) {
            Some((Value::Hash(map), _)) => {
                if map.get(&field).is_some() {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::HEXISTS)),
            None => Ok(0),
        }
    }

    pub fn hlen(&mut self, key: Vec<u8>) -> Result<usize, Sacrilege> {
        match self.0.get(&key) {
            Some((Value::Hash(map), _)) => Ok(map.len()),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::HLEN)),
            None => Ok(0),
        }
    }

    pub fn lpush(&mut self, key: Vec<u8>, elements: Vec<Vec<u8>>) -> Result<usize, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), _) = occupied.get_mut() {
                    for element in elements {
                        list.push_front(element);
                    }

                    Ok(list.len())
                } else {
                    Err(Sacrilege::IncorrectUsage(Command::LPUSH))
                }
            }
            Entry::Vacant(vacant) => {
                let mut list = VecDeque::new();

                for element in elements {
                    list.push_front(element);
                }

                let len = list.len();

                vacant.insert((Value::List(list), None));

                Ok(len)
            }
        }
    }

    pub fn lpop(&mut self, key: Vec<u8>) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), _) = occupied.get_mut() {
                    let element = list.pop_front();

                    if list.is_empty() {
                        occupied.remove();
                    }

                    Ok(element)
                } else {
                    Err(Sacrilege::IncorrectUsage(Command::LPOP))
                }
            }
            Entry::Vacant(_) => Ok(None),
        }
    }

    pub fn lpop_m(
        &mut self,
        key: Vec<u8>,
        count: usize,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), _) = occupied.get_mut() {
                    let mut popped = Vec::new();

                    for _ in 0..count {
                        if let Some(element) = list.pop_front() {
                            popped.push(Some(element));
                        } else {
                            break;
                        }
                    }

                    if list.is_empty() {
                        occupied.remove();
                    }

                    Ok(Some(popped))
                } else {
                    Err(Sacrilege::IncorrectUsage(Command::LPOP))
                }
            }
            Entry::Vacant(_) => Ok(None),
        }
    }

    pub fn rpush(&mut self, key: Vec<u8>, elements: Vec<Vec<u8>>) -> Result<usize, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), _) = occupied.get_mut() {
                    for element in elements {
                        list.push_back(element);
                    }

                    Ok(list.len())
                } else {
                    Err(Sacrilege::IncorrectUsage(Command::RPUSH))
                }
            }
            Entry::Vacant(vacant) => {
                let mut list = VecDeque::new();

                for element in elements {
                    list.push_back(element);
                }

                let len = list.len();

                vacant.insert((Value::List(list), None));

                Ok(len)
            }
        }
    }

    pub fn rpop(&mut self, key: Vec<u8>) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), _) = occupied.get_mut() {
                    let element = list.pop_back();

                    if list.is_empty() {
                        occupied.remove();
                    }

                    Ok(element)
                } else {
                    Err(Sacrilege::IncorrectUsage(Command::RPOP))
                }
            }
            Entry::Vacant(_) => Ok(None),
        }
    }

    pub fn rpop_m(
        &mut self,
        key: Vec<u8>,
        count: usize,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), _) = occupied.get_mut() {
                    let mut popped = Vec::new();

                    for _ in 0..count {
                        if let Some(element) = list.pop_back() {
                            popped.push(Some(element));
                        } else {
                            break;
                        }
                    }

                    if list.is_empty() {
                        occupied.remove();
                    }

                    Ok(Some(popped))
                } else {
                    Err(Sacrilege::IncorrectUsage(Command::RPOP))
                }
            }
            Entry::Vacant(_) => Ok(None),
        }
    }

    pub fn llen(&self, key: Vec<u8>) -> Result<usize, Sacrilege> {
        match self.0.get(&key) {
            Some((Value::List(list), _)) => Ok(list.len()),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::LLEN)),
            None => Ok(0),
        }
    }

    pub fn lrange(
        &self,
        key: Vec<u8>,
        mut starting_index: i32,
        mut ending_index: i32,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        match self.0.get(&key) {
            Some((Value::List(list), _)) => {
                let list_len = list.len() as i32;

                if starting_index < 0 {
                    starting_index += list_len;
                }

                if ending_index < 0 {
                    ending_index += list_len;
                }

                if starting_index < 0 {
                    starting_index = 0;
                }

                if ending_index > list_len {
                    ending_index = list_len - 1;
                }

                if ending_index - starting_index >= 0
                    && starting_index < list_len
                    && ending_index < list_len
                {
                    Ok(Some(
                        list.range(starting_index as usize..(ending_index + 1) as usize)
                            .map(|e| Some(e.clone()))
                            .collect(),
                    ))
                } else {
                    Ok(vec![].into())
                }
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::LRANGE)),
            None => Ok(vec![].into()),
        }
    }

    pub fn lindex(&self, key: Vec<u8>, mut index: i32) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.0.get(&key) {
            Some((Value::List(list), _)) => {
                let list_len = list.len() as i32;

                if index < 0 {
                    index += list_len;
                }

                if index < 0 || index >= list_len {
                    return Ok(None);
                }

                Ok(list.get(index as usize).cloned())
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::LINDEX)),
            None => Ok(None),
        }
    }

    pub fn lset(
        &mut self,
        key: Vec<u8>,
        mut index: i32,
        element: Vec<u8>,
    ) -> Result<(), Sacrilege> {
        match self.0.get_mut(&key) {
            Some((Value::List(list), _)) => {
                let list_len = list.len() as i32;

                if index < 0 {
                    index += list_len;
                }

                if index < 0 || index >= list_len {
                    return Err(Sacrilege::IncorrectUsage(Command::LSET));
                }

                list[index as usize] = element;

                Ok(())
            }
            Some(_) => {
                Err(Sacrilege::IncorrectUsage(Command::LSET))
            }
            None => Err(Sacrilege::IncorrectUsage(Command::LSET)),
        }
    }

    pub fn lrem(
        &mut self,
        key: Vec<u8>,
        mut count: i32,
        element: Vec<u8>,
    ) -> Result<usize, Sacrilege> {
        match self.0.get_mut(&key) {
            Some((Value::List(list), _)) => {
                let initial_len = list.len();

                if count < 0 {
                    let mut idx: i32 = list.len() as i32 - 1;

                    while idx >= 0 && count < 0 {
                        if list[idx as usize] == element {
                            list.remove(idx as usize);
                            count += 1;
                        }

                        idx -= 1;
                    }
                } else if count > 0 {
                    let mut list_len = list.len();
                    let mut idx = 0;

                    while idx < list_len && count > 0 {
                        if list[idx] == element {
                            list.remove(idx);
                            count -= 1;
                            list_len -= 1;

                            continue;
                        }

                        idx += 1;
                    }
                } else {
                    list.retain(|existing_element| *existing_element != element);
                }

                Ok(initial_len - list.len())
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::LREM)),
            None => Ok(0),
        }
    }
}

pub enum Wish {
    Get {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
    },
    Set {
        key: Vec<u8>,
        token: Token,
        value: (Value, Option<SystemTime>),
        tx: Sender<Decree>,
    },
    Del {
        keys: Vec<Vec<u8>>,
        token: Token,
        tx: Sender<Decree>,
    },
    Append {
        key: Vec<u8>,
        token: Token,
        value: Value,
        tx: Sender<Decree>,
    },
    Incr {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
    },
    Decr {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
    },
    Strlen {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
    },
    Exists {
        keys: Vec<Vec<u8>>,
        token: Token,
        tx: Sender<Decree>,
    },
    Hset {
        key: Vec<u8>,
        token: Token,
        field_value_pairs: Vec<(Vec<u8>, Vec<u8>)>,
        tx: Sender<Decree>,
    },
    Hget {
        key: Vec<u8>,
        token: Token,
        field: Vec<u8>,
        tx: Sender<Decree>,
    },
    Hmget {
        key: Vec<u8>,
        token: Token,
        fields: Vec<Vec<u8>>,
        tx: Sender<Decree>,
    },
    Hdel {
        key: Vec<u8>,
        token: Token,
        fields: Vec<Vec<u8>>,
        tx: Sender<Decree>,
    },
    Hexists {
        key: Vec<u8>,
        token: Token,
        field: Vec<u8>,
        tx: Sender<Decree>,
    },
    Hlen {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
    },
    Lpush {
        key: Vec<u8>,
        token: Token,
        elements: Vec<Vec<u8>>,
        tx: Sender<Decree>,
    },
    Lpop {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
    },
    LpopM {
        key: Vec<u8>,
        count: usize,
        token: Token,
        tx: Sender<Decree>,
    },
    Rpush {
        key: Vec<u8>,
        token: Token,
        elements: Vec<Vec<u8>>,
        tx: Sender<Decree>,
    },
    Rpop {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
    },
    RpopM {
        key: Vec<u8>,
        count: usize,
        token: Token,
        tx: Sender<Decree>,
    },
    Llen {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
    },
    Lrange {
        key: Vec<u8>,
        starting_index: i32,
        ending_index: i32,
        token: Token,
        tx: Sender<Decree>,
    },
    Lindex {
        key: Vec<u8>,
        index: i32,
        token: Token,
        tx: Sender<Decree>,
    },
    Lset {
        key: Vec<u8>,
        index: i32,
        element: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
    },
    Lrem {
        key: Vec<u8>,
        count: i32,
        element: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
    },
}

#[derive(Clone)]
pub struct Temple<'a> {
    name: &'a str,
    tx: Sender<Wish>,
}

impl<'a> Temple<'a> {
    pub fn new(name: &'a str) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut soul = Soul::new();

            loop {
                match rx.recv() {
                    Ok(wish) => match wish {
                        Wish::Get { key, token, tx } => match soul.get(key) {
                            Ok(bulk_string) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::BulkString(bulk_string),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Set {
                            key,
                            token,
                            value: val,
                            tx,
                        } => {
                            soul.set(key, val);

                            if tx
                                .send(Decree::Deliver(Gift {
                                    token,
                                    response: Response::Info(InfoType::Ok),
                                }))
                                .is_err()
                            {
                                eprintln!("angel panicked");
                            }
                        }
                        Wish::Del { keys, token, tx } => {
                            if tx
                                .send(Decree::Deliver(Gift {
                                    token,
                                    response: Response::Amount(soul.del(keys)),
                                }))
                                .is_err()
                            {
                                eprintln!("angel panicked");
                            }
                        }
                        Wish::Append {
                            key,
                            token,
                            value: val,
                            tx,
                        } => {
                            if tx
                                .send(Decree::Deliver(Gift {
                                    token,
                                    response: Response::Length(soul.append(key, val)),
                                }))
                                .is_err()
                            {
                                eprintln!("angel panicked");
                            }
                        }
                        Wish::Incr { key, token, tx } => match soul.incr(key) {
                            Ok(number) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Number(number),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Decr { key, token, tx } => match soul.decr(key) {
                            Ok(number) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Number(number),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Strlen { key, token, tx } => match soul.strlen(key) {
                            Ok(length) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Length(length),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked")
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked")
                                }
                            }
                        },
                        Wish::Exists { keys, token, tx } => {
                            if tx
                                .send(Decree::Deliver(Gift {
                                    token,
                                    response: Response::Amount(soul.exists(keys)),
                                }))
                                .is_err()
                            {
                                eprintln!("angel panicked");
                            }
                        }
                        Wish::Hset {
                            key,
                            token,
                            field_value_pairs,
                            tx,
                        } => match soul.hset(key, field_value_pairs) {
                            Ok(new_values_added) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Amount(new_values_added),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                };
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                };
                            }
                        },
                        Wish::Hget {
                            key,
                            token,
                            field,
                            tx,
                        } => match soul.hget(key, field) {
                            Ok(bulk_string) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::BulkString(bulk_string),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Hmget {
                            key,
                            token,
                            fields,
                            tx,
                        } => match soul.hmget(key, fields) {
                            Ok(bulk_string_array) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::BulkStringArray(bulk_string_array),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Hdel {
                            key,
                            token,
                            fields,
                            tx,
                        } => match soul.hdel(key, fields) {
                            Ok(amount) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Amount(amount),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Hexists {
                            key,
                            token,
                            field,
                            tx,
                        } => match soul.hexists(key, field) {
                            Ok(amount) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Amount(amount),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Hlen { key, token, tx } => match soul.hlen(key) {
                            Ok(length) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Length(length),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Lpush {
                            key,
                            token,
                            elements,
                            tx,
                        } => match soul.lpush(key, elements) {
                            Ok(length) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Length(length),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Lpop { key, token, tx } => match soul.lpop(key) {
                            Ok(element) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::BulkString(element),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::LpopM {
                            key,
                            count,
                            token,
                            tx,
                        } => match soul.lpop_m(key, count) {
                            Ok(elements) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::BulkStringArray(elements),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Rpush {
                            key,
                            token,
                            elements,
                            tx,
                        } => match soul.rpush(key, elements) {
                            Ok(length) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Length(length),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Rpop { key, token, tx } => match soul.rpop(key) {
                            Ok(element) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::BulkString(element),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::RpopM {
                            key,
                            count,
                            token,
                            tx,
                        } => match soul.rpop_m(key, count) {
                            Ok(elements) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::BulkStringArray(elements),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Llen { key, token, tx } => match soul.llen(key) {
                            Ok(length) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Length(length),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Lrange {
                            key,
                            starting_index,
                            ending_index,
                            token,
                            tx,
                        } => match soul.lrange(key, starting_index, ending_index) {
                            Ok(bulk_string_array) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::BulkStringArray(bulk_string_array),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Lindex {
                            key,
                            token,
                            index,
                            tx,
                        } => match soul.lindex(key, index) {
                            Ok(element) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::BulkString(element),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Lset {
                            key,
                            token,
                            element,
                            index,
                            tx,
                        } => match soul.lset(key, index, element) {
                            Ok(_) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Info(InfoType::Ok),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                        Wish::Lrem {
                            key,
                            token,
                            element,
                            count,
                            tx,
                        } => match soul.lrem(key, count, element) {
                            Ok(amount) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Length(amount),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            Err(sacrilege) => {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::Error(sacrilege),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        },
                    },
                    Err(e) => {
                        eprintln!("GodThread: {}", e);
                        break;
                    }
                }
            }
        });

        Temple { name, tx }
    }

    pub fn get(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token) {
        if self.tx.send(Wish::Get { key, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn set(
        &self,
        key: Vec<u8>,
        value: (Value, Option<SystemTime>),
        tx: Sender<Decree>,
        token: Token,
    ) {
        if self
            .tx
            .send(Wish::Set {
                key,
                token,
                value,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn del(&self, keys: Vec<Vec<u8>>, tx: Sender<Decree>, token: Token) {
        if self.tx.send(Wish::Del { keys, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn exists(&self, keys: Vec<Vec<u8>>, tx: Sender<Decree>, token: Token) {
        if self.tx.send(Wish::Exists { keys, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn append(&self, key: Vec<u8>, value: Value, tx: Sender<Decree>, token: Token) {
        if self
            .tx
            .send(Wish::Append {
                key,
                token,
                value,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn incr(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token) {
        if self.tx.send(Wish::Incr { key, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn decr(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token) {
        if self.tx.send(Wish::Decr { key, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn strlen(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token) {
        if self.tx.send(Wish::Strlen { key, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn hset(
        &self,
        key: Vec<u8>,
        field_value_pairs: Vec<(Vec<u8>, Vec<u8>)>,
        tx: Sender<Decree>,
        token: Token,
    ) {
        if self
            .tx
            .send(Wish::Hset {
                key,
                token,
                field_value_pairs,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hget(&self, tx: Sender<Decree>, key: Vec<u8>, field: Vec<u8>, token: Token) {
        if self
            .tx
            .send(Wish::Hget {
                key,
                token,
                field,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hmget(&self, tx: Sender<Decree>, key: Vec<u8>, fields: Vec<Vec<u8>>, token: Token) {
        if self
            .tx
            .send(Wish::Hmget {
                key,
                token,
                fields,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hdel(&self, tx: Sender<Decree>, key: Vec<u8>, fields: Vec<Vec<u8>>, token: Token) {
        if self
            .tx
            .send(Wish::Hdel {
                key,
                token,
                fields,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hexists(&self, tx: Sender<Decree>, key: Vec<u8>, field: Vec<u8>, token: Token) {
        if self
            .tx
            .send(Wish::Hexists {
                key,
                token,
                field,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hlen(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token) {
        if self.tx.send(Wish::Hlen { key, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn lpush(&self, tx: Sender<Decree>, key: Vec<u8>, elements: Vec<Vec<u8>>, token: Token) {
        if self
            .tx
            .send(Wish::Lpush {
                key,
                token,
                elements,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lpop(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token) {
        if self.tx.send(Wish::Lpop { key, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn lpop_m(&self, tx: Sender<Decree>, key: Vec<u8>, count: usize, token: Token) {
        if self
            .tx
            .send(Wish::LpopM {
                key,
                count,
                token,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn rpush(&self, tx: Sender<Decree>, key: Vec<u8>, elements: Vec<Vec<u8>>, token: Token) {
        if self
            .tx
            .send(Wish::Rpush {
                key,
                token,
                elements,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn rpop(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token) {
        if self.tx.send(Wish::Rpop { key, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn rpop_m(&self, tx: Sender<Decree>, key: Vec<u8>, count: usize, token: Token) {
        if self
            .tx
            .send(Wish::RpopM {
                key,
                count,
                token,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn llen(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token) {
        if self.tx.send(Wish::Llen { key, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn lrange(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        starting_index: i32,
        ending_index: i32,
        token: Token,
    ) {
        if self
            .tx
            .send(Wish::Lrange {
                key,
                starting_index,
                ending_index,
                token,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lindex(&self, tx: Sender<Decree>, key: Vec<u8>, index: i32, token: Token) {
        if self
            .tx
            .send(Wish::Lindex {
                key,
                token,
                index,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lset(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        index: i32,
        element: Vec<u8>,
        token: Token,
    ) {
        if self
            .tx
            .send(Wish::Lset {
                key,
                token,
                element,
                index,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lrem(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        count: i32,
        element: Vec<u8>,
        token: Token,
    ) {
        if self
            .tx
            .send(Wish::Lrem {
                key,
                token,
                element,
                count,
                tx,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn sanctify(&self) -> Self {
        self.clone()
    }
}
