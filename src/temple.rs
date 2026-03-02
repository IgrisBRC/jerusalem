use std::collections::hash_map::Entry;
use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::Sender;
use std::{collections::HashMap, time::SystemTime};

use mio::Token;

use crate::wish::grant::{Decree, Gift};
use crate::wish::util::bytes_to_i64;
use crate::wish::{Command, InfoType, Response, Sacrilege};

#[derive(Clone)]
pub enum Value {
    String(Vec<u8>),
    List(VecDeque<Vec<u8>>),
    Hash(HashMap<Vec<u8>, Vec<u8>>),
    Set(HashSet<Vec<u8>>),
    EventMap(HashMap<Vec<u8>, HashSet<Token>>),
    ClientMap(HashMap<Token, HashSet<Vec<u8>>>),
}

pub struct Soul(HashMap<Vec<u8>, (Value, Option<SystemTime>)>);
pub struct EventMap(HashMap<Token, HashSet<Vec<u8>>>);
pub struct ClientMap(HashMap<Vec<u8>, HashSet<Token>>);

impl Default for Soul {
    fn default() -> Self {
        Self::new()
    }
}

impl Soul {
    pub fn new() -> Self {
        Soul(HashMap::new())
    }

    pub fn get(&mut self, key: Vec<u8>, now: SystemTime) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::String(value)) => return Ok(Some(value.clone())),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::GET)),
            None => return Ok(None),
        }
    }

    pub fn set(&mut self, key: Vec<u8>, val: (Value, Option<SystemTime>)) {
        self.0.insert(key, val);
    }

    pub fn append(
        &mut self,
        key: Vec<u8>,
        mut incoming_value: Vec<u8>,
        now: SystemTime,
    ) -> Result<usize, Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::String(value)) => {
                value.append(&mut incoming_value);
                return Ok(value.len());
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::APPEND)),
            None => {
                let incoming_value_len = incoming_value.len();
                self.0.insert(key, (Value::String(incoming_value), None));
                Ok(incoming_value_len)
            }
        }
    }

    pub fn incr(&mut self, key: Vec<u8>, now: SystemTime) -> Result<i64, Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::String(value)) => {
                let mut itoa_buf = itoa::Buffer::new();

                let Ok(mut number) = bytes_to_i64(value) else {
                    return Err(Sacrilege::IncorrectUsage(Command::INCR));
                };

                number += 1;

                value.clear();
                value.extend_from_slice(itoa_buf.format(number).as_bytes());

                Ok(number)
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::INCR)),
            None => {
                self.0.insert(key, (Value::String(b"1".into()), None));
                Ok(1)
            }
        }
    }

    pub fn decr(&mut self, key: Vec<u8>, now: SystemTime) -> Result<i64, Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::String(value)) => {
                let mut itoa_buf = itoa::Buffer::new();

                let Ok(mut number) = bytes_to_i64(value) else {
                    return Err(Sacrilege::IncorrectUsage(Command::DECR));
                };

                number -= 1;

                value.clear();
                value.extend_from_slice(itoa_buf.format(number).as_bytes());

                Ok(number)
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::DECR)),
            None => {
                self.0.insert(key, (Value::String(b"-1".into()), None));
                Ok(-1)
            }
        }
    }

    pub fn strlen(&self, key: Vec<u8>, now: SystemTime) -> Result<usize, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::String(value)) => Ok(value.len()),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::STRLEN)),
            None => Ok(0),
        }
    }

    pub fn del(&mut self, keys: Vec<Vec<u8>>, now: SystemTime) -> u32 {
        let mut number_of_entries_deleted = 0;

        for key in keys {
            if self.remove_valid_value(&key, now).is_some() {
                number_of_entries_deleted += 1;
            }
        }

        number_of_entries_deleted
    }

    pub fn exists(&self, keys: Vec<Vec<u8>>, now: SystemTime) -> u32 {
        let mut number_of_entries_that_exist = 0;

        for key in keys {
            if self.get_valid_value(&key, now).is_some() {
                number_of_entries_that_exist += 1;
            }
        }

        number_of_entries_that_exist
    }

    pub fn hset(
        &mut self,
        key: Vec<u8>,
        field_value_pairs: Vec<(Vec<u8>, Vec<u8>)>,
        now: SystemTime,
    ) -> Result<u32, Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::Hash(map)) => {
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

    pub fn hget(
        &mut self,
        key: Vec<u8>,
        field: Vec<u8>,
        now: SystemTime,
    ) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::Hash(map)) => Ok(map.get(&field).cloned()),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::HGET)),
            None => Ok(None),
        }
    }

    pub fn hmget(
        &mut self,
        key: Vec<u8>,
        fields: Vec<Vec<u8>>,
        now: SystemTime,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        let mut values = Vec::new();

        match self.get_valid_value(&key, now) {
            Some(Value::Hash(map)) => {
                for field in fields {
                    values.push(map.get(&field).cloned());
                }

                Ok(Some(values))
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::HMGET)),
            None => Ok(None),
        }
    }

    pub fn hdel(
        &mut self,
        key: Vec<u8>,
        fields: Vec<Vec<u8>>,
        now: SystemTime,
    ) -> Result<u32, Sacrilege> {
        let mut amount_of_deleted_values = 0;

        match self.get_mut_valid_value(&key, now) {
            Some(Value::Hash(map)) => {
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

    pub fn hexists(
        &mut self,
        key: Vec<u8>,
        field: Vec<u8>,
        now: SystemTime,
    ) -> Result<u32, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::Hash(map)) => {
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

    pub fn hlen(&mut self, key: Vec<u8>, now: SystemTime) -> Result<usize, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::Hash(map)) => Ok(map.len()),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::HLEN)),
            None => Ok(0),
        }
    }

    pub fn lpush(
        &mut self,
        key: Vec<u8>,
        mut elements: Vec<Vec<u8>>,
        now: SystemTime,
    ) -> Result<usize, Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::List(list)) => {
                for element in elements {
                    list.push_front(element);
                }
                Ok(list.len())
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::LPUSH)),
            None => {
                let elements_len = elements.len();
                elements.reverse();

                self.0
                    .insert(key, (Value::List(VecDeque::from(elements)), None));

                Ok(elements_len)
            }
        }
    }

    pub fn lpop(&mut self, key: Vec<u8>, now: SystemTime) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), expiry) = occupied.get_mut() {
                    if let Some(expiry) = expiry {
                        if *expiry < now {
                            occupied.remove();
                            return Ok(None);
                        }
                    }

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
        now: SystemTime,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), expiry) = occupied.get_mut() {
                    if let Some(expiry) = expiry {
                        if *expiry < now {
                            occupied.remove();
                            return Ok(None);
                        }
                    }

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

    pub fn rpush(
        &mut self,
        key: Vec<u8>,
        elements: Vec<Vec<u8>>,
        now: SystemTime,
    ) -> Result<usize, Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::List(list)) => {
                for element in elements {
                    list.push_back(element);
                }
                Ok(list.len())
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::RPUSH)),
            None => {
                let elements_len = elements.len();

                self.0
                    .insert(key, (Value::List(VecDeque::from(elements)), None));

                Ok(elements_len)
            }
        }
    }

    pub fn rpop(&mut self, key: Vec<u8>, now: SystemTime) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), expiry) = occupied.get_mut() {
                    if let Some(expiry) = expiry {
                        if *expiry < now {
                            occupied.remove();
                            return Ok(None);
                        }
                    }

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
        now: SystemTime,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), expiry) = occupied.get_mut() {
                    if let Some(expiry) = expiry {
                        if *expiry < now {
                            occupied.remove();
                            return Ok(None);
                        }
                    }

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

    pub fn llen(&self, key: Vec<u8>, now: SystemTime) -> Result<usize, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::List(list)) => Ok(list.len()),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::LLEN)),
            None => Ok(0),
        }
    }

    pub fn lrange(
        &self,
        key: Vec<u8>,
        mut starting_index: i32,
        mut ending_index: i32,
        now: SystemTime,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::List(list)) => {
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

    pub fn lindex(
        &self,
        key: Vec<u8>,
        mut index: i32,
        now: SystemTime,
    ) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::List(list)) => {
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
        now: SystemTime,
    ) -> Result<(), Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::List(list)) => {
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
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::LSET)),
            None => Err(Sacrilege::IncorrectUsage(Command::LSET)),
        }
    }

    pub fn lrem(
        &mut self,
        key: Vec<u8>,
        mut count: i32,
        element: Vec<u8>,
        now: SystemTime,
    ) -> Result<usize, Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::List(list)) => {
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

    pub fn expire(&mut self, key: Vec<u8>, expiry: SystemTime, now: SystemTime) -> u32 {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                let (_, existing_expiry) = occupied.get_mut();

                if let Some(expiry) = existing_expiry {
                    if *expiry < now {
                        occupied.remove();
                        return 0;
                    }
                }

                *existing_expiry = Some(expiry);
                1
            }
            Entry::Vacant(_) => 0,
        }
    }

    pub fn ttl(&mut self, key: Vec<u8>, now: SystemTime) -> i64 {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                let (_, existing_expiry) = occupied.get_mut();

                if let Some(expiry) = existing_expiry {
                    if *expiry < now {
                        occupied.remove();
                        return -2;
                    } else {
                        let Ok(duration) = (*expiry).duration_since(now) else {
                            occupied.remove();
                            return -2;
                        };

                        duration.as_secs() as i64
                    }
                } else {
                    return -1;
                }
            }
            Entry::Vacant(_) => -2,
        }
    }

    fn get_valid_value(&self, key: &Vec<u8>, now: SystemTime) -> Option<&Value> {
        match self.0.get(key) {
            Some((value, Some(expiry))) => {
                if *expiry < now {
                    return None;
                } else {
                    Some(value)
                }
            }
            Some((value, _)) => Some(value),
            None => None,
        }
    }

    fn get_mut_valid_value(&mut self, key: &Vec<u8>, now: SystemTime) -> Option<&mut Value> {
        match self.0.get_mut(key) {
            Some((value, Some(expiry))) => {
                if *expiry < now {
                    return None;
                } else {
                    Some(value)
                }
            }
            Some((value, _)) => Some(value),
            None => None,
        }
    }

    pub fn remove_valid_value(&mut self, key: &Vec<u8>, now: SystemTime) -> Option<Value> {
        match self.0.remove(key) {
            Some((value, Some(expiry))) => {
                if expiry < now {
                    None
                } else {
                    Some(value)
                }
            }
            Some((value, None)) => Some(value),
            None => None,
        }
    }
}

impl Default for ClientMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientMap {
    pub fn new() -> Self {
        ClientMap(HashMap::new())
    }

    pub fn subscribe(&mut self, token: Token, events: Vec<Vec<u8>>) {
        for event in events {
            match self.0.get_mut(&event) {
                Some(set) => {
                    set.insert(token);
                }
                None => {
                    let mut set = HashSet::new();
                    set.insert(token);

                    self.0.insert(event, set);
                }
            }
        }
    }

    pub fn publish(&self, event: Vec<u8>) -> Vec<Token> {
        match self.0.get(&event) {
            Some(clients) => clients.iter().cloned().collect(),
            None => Vec::new(),
        }
    }
}

impl Default for EventMap {
    fn default() -> Self {
        Self::new()
    }
}

impl EventMap {
    pub fn new() -> Self {
        EventMap(HashMap::new())
    }

    pub fn subscribe(&mut self, token: Token, events: Vec<Vec<u8>>) -> usize {
        match self.0.get_mut(&token) {
            Some(set) => {
                for event in events {
                    set.insert(event);
                }

                set.len()
            }
            None => {
                let mut set = HashSet::new();

                for event in events {
                    set.insert(event);
                }

                let set_len = set.len();

                self.0.insert(token, set);

                set_len
            }
        }
    }
}

pub enum Wish {
    Get {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
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
        time: SystemTime,
    },
    Append {
        key: Vec<u8>,
        token: Token,
        value: Vec<u8>,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Incr {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Decr {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Strlen {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Exists {
        keys: Vec<Vec<u8>>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Hset {
        key: Vec<u8>,
        token: Token,
        field_value_pairs: Vec<(Vec<u8>, Vec<u8>)>,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Hget {
        key: Vec<u8>,
        token: Token,
        field: Vec<u8>,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Hmget {
        key: Vec<u8>,
        token: Token,
        fields: Vec<Vec<u8>>,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Hdel {
        key: Vec<u8>,
        token: Token,
        fields: Vec<Vec<u8>>,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Hexists {
        key: Vec<u8>,
        token: Token,
        field: Vec<u8>,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Hlen {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Lpush {
        key: Vec<u8>,
        token: Token,
        elements: Vec<Vec<u8>>,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Lpop {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    LpopM {
        key: Vec<u8>,
        count: usize,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Rpush {
        key: Vec<u8>,
        token: Token,
        elements: Vec<Vec<u8>>,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Rpop {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    RpopM {
        key: Vec<u8>,
        count: usize,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Llen {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Lrange {
        key: Vec<u8>,
        starting_index: i32,
        ending_index: i32,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Lindex {
        key: Vec<u8>,
        index: i32,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Lset {
        key: Vec<u8>,
        index: i32,
        element: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Lrem {
        key: Vec<u8>,
        count: i32,
        element: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Expire {
        key: Vec<u8>,
        expiry: SystemTime,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Ttl {
        key: Vec<u8>,
        token: Token,
        tx: Sender<Decree>,
        time: SystemTime,
    },
    Subscribe {
        events: Vec<Vec<u8>>,
        token: Token,
        tx: Sender<Decree>,
    },
    Publish {
        event: Vec<u8>,
        message: Vec<u8>,
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
            let mut client_map = ClientMap::new();
            let mut event_map = EventMap::new();

            loop {
                match rx.recv() {
                    Ok(wish) => match wish {
                        Wish::Get {
                            key,
                            token,
                            tx,
                            time,
                        } => match soul.get(key, time) {
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
                        Wish::Del {
                            keys,
                            token,
                            tx,
                            time,
                        } => {
                            if tx
                                .send(Decree::Deliver(Gift {
                                    token,
                                    response: Response::Amount(soul.del(keys, time)),
                                }))
                                .is_err()
                            {
                                eprintln!("angel panicked");
                            }
                        }
                        Wish::Append {
                            key,
                            token,
                            value,
                            tx,
                            time,
                        } => match soul.append(key, value, time) {
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

                        Wish::Incr {
                            key,
                            token,
                            tx,
                            time,
                        } => match soul.incr(key, time) {
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
                        Wish::Decr {
                            key,
                            token,
                            tx,
                            time,
                        } => match soul.decr(key, time) {
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
                        Wish::Strlen {
                            key,
                            token,
                            tx,
                            time,
                        } => match soul.strlen(key, time) {
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
                        Wish::Exists {
                            keys,
                            token,
                            tx,
                            time,
                        } => {
                            if tx
                                .send(Decree::Deliver(Gift {
                                    token,
                                    response: Response::Amount(soul.exists(keys, time)),
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
                            time,
                        } => match soul.hset(key, field_value_pairs, time) {
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
                            time,
                        } => match soul.hget(key, field, time) {
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
                            time,
                        } => match soul.hmget(key, fields, time) {
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
                            time,
                        } => match soul.hdel(key, fields, time) {
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
                            time,
                        } => match soul.hexists(key, field, time) {
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
                        Wish::Hlen {
                            key,
                            token,
                            tx,
                            time,
                        } => match soul.hlen(key, time) {
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
                            time,
                        } => match soul.lpush(key, elements, time) {
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
                        Wish::Lpop {
                            key,
                            token,
                            tx,
                            time,
                        } => match soul.lpop(key, time) {
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
                            time,
                        } => match soul.lpop_m(key, count, time) {
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
                            time,
                        } => match soul.rpush(key, elements, time) {
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
                        Wish::Rpop {
                            key,
                            token,
                            tx,
                            time,
                        } => match soul.rpop(key, time) {
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
                            time,
                        } => match soul.rpop_m(key, count, time) {
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
                        Wish::Llen {
                            key,
                            token,
                            tx,
                            time,
                        } => match soul.llen(key, time) {
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
                            time,
                        } => match soul.lrange(key, starting_index, ending_index, time) {
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
                            time,
                        } => match soul.lindex(key, index, time) {
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
                            time,
                        } => match soul.lset(key, index, element, time) {
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
                            time,
                        } => match soul.lrem(key, count, element, time) {
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
                        Wish::Expire {
                            key,
                            token,
                            expiry,
                            tx,
                            time,
                        } => {
                            if tx
                                .send(Decree::Deliver(Gift {
                                    token,
                                    response: Response::Amount(soul.expire(key, expiry, time)),
                                }))
                                .is_err()
                            {
                                eprintln!("angel panicked");
                            }
                        }
                        Wish::Ttl {
                            key,
                            token,
                            tx,
                            time,
                        } => {
                            if tx
                                .send(Decree::Deliver(Gift {
                                    token,
                                    response: Response::Number(soul.ttl(key, time)),
                                }))
                                .is_err()
                            {
                                eprintln!("angel panicked");
                            }
                        }
                        Wish::Subscribe { events, token, tx } => {
                            let number_of_subscribed_channels =
                                event_map.subscribe(token, events.clone());
                            client_map.subscribe(token, events.clone());

                            for event in events {
                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::NumberOfSubscribedChannels(
                                            event,
                                            number_of_subscribed_channels,
                                        ),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                        }
                        Wish::Publish {
                            event,
                            message,
                            token,
                            tx,
                        } => {
                            let clients = client_map.publish(event.clone());

                            if tx
                                .send(Decree::Broadcast(token, event, message, clients))
                                .is_err()
                            {
                                eprintln!("angel panicked");
                            }
                        }
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

    pub fn get(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Get {
                key,
                token,
                tx,
                time,
            })
            .is_err()
        {
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

    pub fn del(&self, keys: Vec<Vec<u8>>, tx: Sender<Decree>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Del {
                keys,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn exists(&self, keys: Vec<Vec<u8>>, tx: Sender<Decree>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Exists {
                keys,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn append(
        &self,
        key: Vec<u8>,
        value: Vec<u8>,
        tx: Sender<Decree>,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Append {
                key,
                token,
                value,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn incr(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Incr {
                key,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn decr(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Decr {
                key,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn strlen(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Strlen {
                key,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hset(
        &self,
        key: Vec<u8>,
        field_value_pairs: Vec<(Vec<u8>, Vec<u8>)>,
        tx: Sender<Decree>,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Hset {
                key,
                token,
                field_value_pairs,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hget(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        field: Vec<u8>,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Hget {
                key,
                token,
                field,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hmget(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        fields: Vec<Vec<u8>>,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Hmget {
                key,
                token,
                fields,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hdel(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        fields: Vec<Vec<u8>>,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Hdel {
                key,
                token,
                fields,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hexists(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        field: Vec<u8>,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Hexists {
                key,
                token,
                field,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hlen(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Hlen {
                key,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lpush(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        elements: Vec<Vec<u8>>,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Lpush {
                key,
                token,
                elements,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lpop(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Lpop {
                key,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lpop_m(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        count: usize,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::LpopM {
                key,
                count,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn rpush(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        elements: Vec<Vec<u8>>,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Rpush {
                key,
                token,
                elements,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn rpop(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Rpop {
                key,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn rpop_m(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        count: usize,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::RpopM {
                key,
                count,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn llen(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Llen {
                key,
                token,
                tx,
                time,
            })
            .is_err()
        {
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
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Lrange {
                key,
                starting_index,
                ending_index,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lindex(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        index: i32,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Lindex {
                key,
                token,
                index,
                tx,
                time,
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
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Lset {
                key,
                token,
                element,
                index,
                tx,
                time,
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
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Lrem {
                key,
                token,
                element,
                count,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn expire(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        expiry: SystemTime,
        token: Token,
        time: SystemTime,
    ) {
        if self
            .tx
            .send(Wish::Expire {
                key,
                expiry,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn ttl(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish::Ttl {
                key,
                token,
                tx,
                time,
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn subscribe(&self, tx: Sender<Decree>, events: Vec<Vec<u8>>, token: Token) {
        if self.tx.send(Wish::Subscribe { events, token, tx }).is_err() {
            eprintln!("angel panicked");
        }
    }

    pub fn publish(&self, tx: Sender<Decree>, event: Vec<u8>, message: Vec<u8>, token: Token) {
        if self
            .tx
            .send(Wish::Publish {
                event,
                message,
                token,
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
