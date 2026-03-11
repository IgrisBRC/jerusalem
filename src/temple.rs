use std::collections::hash_map::Entry;
use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender};
use std::time::UNIX_EPOCH;
use std::vec::IntoIter;
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
    EventMap(HashMap<Vec<u8>, HashSet<usize>>),
    ClientMap(HashMap<usize, HashSet<Vec<u8>>>),
}

pub struct Soul(HashMap<Vec<u8>, (Value, Option<u64>)>);

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

    pub fn get(&mut self, key: Vec<u8>, now: u64) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::String(value)) => Ok(Some(value.clone())),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::GET)),
            None => Ok(None),
        }
    }

    pub fn set(&mut self, key: Vec<u8>, val: (Value, Option<u64>)) {
        self.0.insert(key, val);
    }

    pub fn append(
        &mut self,
        key: Vec<u8>,
        mut incoming_value: Vec<u8>,
        now: u64,
    ) -> Result<usize, Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::String(value)) => {
                value.append(&mut incoming_value);
                Ok(value.len())
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::APPEND)),
            None => {
                let incoming_value_len = incoming_value.len();
                self.0.insert(key, (Value::String(incoming_value), None));
                Ok(incoming_value_len)
            }
        }
    }

    pub fn incr(&mut self, key: Vec<u8>, now: u64) -> Result<i64, Sacrilege> {
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

    pub fn decr(&mut self, key: Vec<u8>, now: u64) -> Result<i64, Sacrilege> {
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

    pub fn strlen(&self, key: Vec<u8>, now: u64) -> Result<usize, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::String(value)) => Ok(value.len()),
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::STRLEN)),
            None => Ok(0),
        }
    }

    pub fn del(&mut self, keys: Vec<Vec<u8>>, now: u64) -> u32 {
        let mut number_of_entries_deleted = 0;

        for key in keys {
            if self.remove_valid_value(&key, now).is_some() {
                number_of_entries_deleted += 1;
            }
        }

        number_of_entries_deleted
    }

    pub fn exists(&self, keys: Vec<Vec<u8>>, now: u64) -> u32 {
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
        now: u64,
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
        now: u64,
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
        now: u64,
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

    pub fn hdel(&mut self, key: Vec<u8>, fields: Vec<Vec<u8>>, now: u64) -> Result<u32, Sacrilege> {
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

    pub fn hexists(&mut self, key: Vec<u8>, field: Vec<u8>, now: u64) -> Result<u32, Sacrilege> {
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

    pub fn hlen(&mut self, key: Vec<u8>, now: u64) -> Result<usize, Sacrilege> {
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
        now: u64,
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

    pub fn lpop(&mut self, key: Vec<u8>, now: u64) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), expiry) = occupied.get_mut() {
                    if let Some(expiry) = expiry
                        && *expiry < now
                    {
                        occupied.remove();
                        return Ok(None);
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
        now: u64,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), expiry) = occupied.get_mut() {
                    if let Some(expiry) = expiry
                        && *expiry < now
                    {
                        occupied.remove();
                        return Ok(None);
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
        now: u64,
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

    pub fn rpop(&mut self, key: Vec<u8>, now: u64) -> Result<Option<Vec<u8>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), expiry) = occupied.get_mut() {
                    if let Some(expiry) = expiry
                        && *expiry < now
                    {
                        occupied.remove();
                        return Ok(None);
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
        now: u64,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                if let (Value::List(list), expiry) = occupied.get_mut() {
                    if let Some(expiry) = expiry
                        && *expiry < now
                    {
                        occupied.remove();
                        return Ok(None);
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

    pub fn llen(&self, key: Vec<u8>, now: u64) -> Result<usize, Sacrilege> {
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
        now: u64,
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
        now: u64,
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
        now: u64,
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
        now: u64,
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

    pub fn expire(&mut self, key: Vec<u8>, expiry: u64, now: u64) -> u32 {
        match self.0.entry(key) {
            Entry::Occupied(mut occupied) => {
                let (_, existing_expiry) = occupied.get_mut();

                if let Some(expiry) = existing_expiry
                    && *expiry < now
                {
                    occupied.remove();
                    return 0;
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
                    let expiry =
                        SystemTime::from(UNIX_EPOCH + std::time::Duration::from_secs(*expiry));

                    if expiry < now {
                        occupied.remove();
                        -2
                    } else {
                        let Ok(duration) = SystemTime::from(expiry).duration_since(now) else {
                            occupied.remove();
                            return -2;
                        };

                        duration.as_secs() as i64
                    }
                } else {
                    -1
                }
            }
            Entry::Vacant(_) => -2,
        }
    }

    pub fn mset(&mut self, mut terms_iter: IntoIter<Vec<u8>>) {
        while let (Some(key), Some(value)) = (terms_iter.next(), terms_iter.next()) {
            self.0.insert(key, (Value::String(value), None));
        }
    }

    pub fn mget(&self, terms_iter: IntoIter<Vec<u8>>, now: u64) -> Option<Vec<Option<Vec<u8>>>> {
        let mut result = Vec::with_capacity(terms_iter.len());

        for key in terms_iter {
            match self.get_valid_value(&key, now) {
                Some(Value::String(value)) => {
                    result.push(Some(value.clone()));
                }
                _ => result.push(None),
            }
        }

        Some(result)
    }

    pub fn sadd(
        &mut self,
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
        now: u64,
    ) -> Result<usize, Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::Set(set)) => {
                let mut count = 0;

                for value in values {
                    if set.insert(value) {
                        count += 1;
                    }
                }

                Ok(count)
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::SADD)),
            None => {
                let mut set = HashSet::new();
                let mut count = 0;

                for value in values {
                    if set.insert(value) {
                        count += 1;
                    }
                }

                self.0.insert(key, (Value::Set(set), None));

                Ok(count)
            }
        }
    }

    pub fn srem(
        &mut self,
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
        now: u64,
    ) -> Result<usize, Sacrilege> {
        match self.get_mut_valid_value(&key, now) {
            Some(Value::Set(set)) => {
                let mut count = 0;

                for value in values {
                    if set.remove(&value) {
                        count += 1;
                    }
                }

                if set.is_empty() {
                    self.0.remove(&key);
                }

                Ok(count)
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::SREM)),
            None => Ok(0),
        }
    }

    pub fn sismember(
        &mut self,
        key: Vec<u8>,
        value: Vec<u8>,
        now: u64,
    ) -> Result<usize, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::Set(set)) => {
                if set.contains(&value) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::SREM)),
            None => Ok(0),
        }
    }

    pub fn hgetall(
        &mut self,
        key: Vec<u8>,
        now: u64,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::Hash(map)) => {
                let mut result = Vec::with_capacity(map.len() * 2);

                for (field, value) in map {
                    result.push(Some(field.clone()));
                    result.push(Some(value.clone()));
                }

                Ok(Some(result))
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::HGETALL)),
            None => Ok(None),
        }
    }

    pub fn smembers(
        &mut self,
        key: Vec<u8>,
        now: u64,
    ) -> Result<Option<Vec<Option<Vec<u8>>>>, Sacrilege> {
        match self.get_valid_value(&key, now) {
            Some(Value::Set(set)) => {
                let mut result = Vec::with_capacity(set.len());

                for value in set {
                    result.push(Some(value.clone()));
                }

                Ok(Some(result))
            }
            Some(_) => Err(Sacrilege::IncorrectUsage(Command::SMEMBERS)),
            None => Ok(None),
        }
    }

    fn get_valid_value(&self, key: &Vec<u8>, now: u64) -> Option<&Value> {
        match self.0.get(key) {
            Some((value, Some(expiry))) => {
                if *expiry < now {
                    None
                } else {
                    Some(value)
                }
            }
            Some((value, _)) => Some(value),
            None => None,
        }
    }

    fn get_mut_valid_value(&mut self, key: &Vec<u8>, now: u64) -> Option<&mut Value> {
        match self.0.get_mut(key) {
            Some((value, Some(expiry))) => {
                if *expiry < now {
                    None
                } else {
                    Some(value)
                }
            }
            Some((value, _)) => Some(value),
            None => None,
        }
    }

    pub fn remove_valid_value(&mut self, key: &Vec<u8>, now: u64) -> Option<Value> {
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

    pub fn unsubscribe(&mut self, token: Token, events: &Option<Vec<(Vec<u8>, usize)>>) {
        let Some(events) = events else {
            return;
        };

        for (event, _) in events {
            if let Some(set) = self.0.get_mut(event) {
                set.remove(&token);
                if set.is_empty() {
                    self.0.remove(event);
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

    pub fn subscribe(&mut self, token: Token, events: Vec<Vec<u8>>) -> Vec<(Vec<u8>, usize)> {
        match self.0.get_mut(&token) {
            Some(set) => {
                let mut result = Vec::new();

                let mut count = set.len();

                for event in events {
                    if set.insert(event.clone()) {
                        count += 1;
                        result.push((event, count));
                    }
                }

                result
            }
            None => {
                let mut set = HashSet::new();
                let mut result = Vec::new();
                let mut count = 0;

                for event in events {
                    if set.insert(event.clone()) {
                        count += 1;
                        result.push((event, count));
                    }
                }

                self.0.insert(token, set);

                result
            }
        }
    }

    pub fn unsubscribe(
        &mut self,
        events: Vec<Vec<u8>>,
        token: Token,
        subscribed_clients: &mut HashSet<Token>,
    ) -> Option<Vec<(Vec<u8>, usize)>> {
        match self.0.get_mut(&token) {
            Some(existing_events) => {
                let mut result = Vec::new();
                let mut count = existing_events.len();

                if !events.is_empty() {
                    for event in events {
                        if existing_events.remove(&event) {
                            count -= 1;
                        }

                        result.push((event, count));
                    }

                    if existing_events.is_empty() {
                        self.0.remove(&token);
                        subscribed_clients.remove(&token);
                    }

                    Some(result)
                } else {
                    let unsubscribed_events: Vec<Vec<u8>> =
                        std::mem::take(existing_events).into_iter().collect();
                    let mut count = unsubscribed_events.len();

                    for event in unsubscribed_events {
                        count -= 1;
                        result.push((event, count));
                    }

                    subscribed_clients.remove(&token);
                    self.0.remove(&token);

                    Some(result)
                }
            }
            None => None,
        }
    }
}

pub struct Wish {
    token: Token,
    tx: Sender<Decree>,
    command_type: CommandType,
}

#[derive(Clone)]
pub enum CommandType {
    Get {
        key: Vec<u8>,
        time: u64,
    },
    Set {
        key: Vec<u8>,
        value: (Value, Option<u64>),
    },
    Del {
        keys: Vec<Vec<u8>>,
        time: u64,
    },
    Append {
        key: Vec<u8>,
        value: Vec<u8>,
        time: u64,
    },
    Incr {
        key: Vec<u8>,
        time: u64,
    },
    Decr {
        key: Vec<u8>,
        time: u64,
    },
    Strlen {
        key: Vec<u8>,
        time: u64,
    },
    Exists {
        keys: Vec<Vec<u8>>,
        time: u64,
    },
    Hset {
        key: Vec<u8>,
        field_value_pairs: Vec<(Vec<u8>, Vec<u8>)>,
        time: u64,
    },
    Hget {
        key: Vec<u8>,
        field: Vec<u8>,
        time: u64,
    },
    Hmget {
        key: Vec<u8>,
        fields: Vec<Vec<u8>>,
        time: u64,
    },
    Hdel {
        key: Vec<u8>,
        fields: Vec<Vec<u8>>,
        time: u64,
    },
    Hexists {
        key: Vec<u8>,
        field: Vec<u8>,
        time: u64,
    },
    Hlen {
        key: Vec<u8>,
        time: u64,
    },
    Lpush {
        key: Vec<u8>,
        elements: Vec<Vec<u8>>,
        time: u64,
    },
    Lpop {
        key: Vec<u8>,
        time: u64,
    },
    LpopM {
        key: Vec<u8>,
        count: usize,
        time: u64,
    },
    Rpush {
        key: Vec<u8>,
        elements: Vec<Vec<u8>>,
        time: u64,
    },
    Rpop {
        key: Vec<u8>,
        time: u64,
    },
    RpopM {
        key: Vec<u8>,
        count: usize,
        time: u64,
    },
    Llen {
        key: Vec<u8>,
        time: u64,
    },
    Lrange {
        key: Vec<u8>,
        starting_index: i32,
        ending_index: i32,
        time: u64,
    },
    Lindex {
        key: Vec<u8>,
        index: i32,
        time: u64,
    },
    Lset {
        key: Vec<u8>,
        index: i32,
        element: Vec<u8>,
        time: u64,
    },
    Lrem {
        key: Vec<u8>,
        count: i32,
        element: Vec<u8>,
        time: u64,
    },
    Expire {
        key: Vec<u8>,
        expiry: u64,
        time: u64,
    },
    Ttl {
        key: Vec<u8>,
        time: SystemTime,
    },
    Subscribe {
        events: Vec<Vec<u8>>,
    },
    Publish {
        event: Vec<u8>,
        message: Vec<u8>,
    },
    Mset {
        terms_iter: IntoIter<Vec<u8>>,
    },
    Mget {
        terms_iter: IntoIter<Vec<u8>>,
        time: u64,
    },
    Sadd {
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
        time: u64,
    },
    Srem {
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
        time: u64,
    },
    Sismember {
        key: Vec<u8>,
        value: Vec<u8>,
        time: u64,
    },
    Hgetall {
        key: Vec<u8>,
        time: u64,
    },
    Smembers {
        key: Vec<u8>,
        time: u64,
    },
    Unsubscribe {
        terms: Vec<Vec<u8>>,
    },
}

#[derive(Clone)]
pub struct Temple {
    tx: Sender<Wish>,
}

impl Temple {
    pub fn new() -> Self {
        let (tx, rx): (Sender<Wish>, Receiver<Wish>) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut soul = Soul::new();
            let mut client_map = ClientMap::new();
            let mut event_map = EventMap::new();
            let mut subscribed_clients = HashSet::new();

            loop {
                match rx.recv() {
                    Ok(wish) => {
                        let token = wish.token;
                        let tx = wish.tx;

                        let command_type = wish.command_type;

                        match command_type {
                            CommandType::Subscribe { events } => {
                                subscribed_clients.insert(token);

                                let subscribed_channels =
                                    event_map.subscribe(token, events.clone());
                                client_map.subscribe(token, events.clone());

                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::SubscribedChannels(subscribed_channels),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }

                                continue;
                            }
                            CommandType::Unsubscribe { terms } => {
                                let unsubscribed_events =
                                    event_map.unsubscribe(terms, token, &mut subscribed_clients);
                                client_map.unsubscribe(token, &unsubscribed_events);

                                if tx
                                    .send(Decree::Deliver(Gift {
                                        token,
                                        response: Response::UnsubscribedChannels(
                                            unsubscribed_events,
                                        ),
                                    }))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                };

                                continue;
                            }
                            _ => {}
                        }

                        if subscribed_clients.contains(&token) {
                            if tx
                                .send(Decree::Deliver(Gift {
                                    token,
                                    response: Response::Error(Sacrilege::SubscriberOnlyMode),
                                }))
                                .is_err()
                            {
                                eprintln!("angel panicked");
                            }

                            continue;
                        }

                        match command_type {
                            CommandType::Get { key, time } => match soul.get(key, time) {
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
                            CommandType::Set { key, value: val } => {
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
                            CommandType::Del { keys, time } => {
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
                            CommandType::Append { key, value, time } => {
                                match soul.append(key, value, time) {
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
                                }
                            }

                            CommandType::Incr { key, time } => match soul.incr(key, time) {
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
                            CommandType::Decr { key, time } => match soul.decr(key, time) {
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
                            CommandType::Strlen { key, time } => match soul.strlen(key, time) {
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
                            CommandType::Exists { keys, time } => {
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
                            CommandType::Hset {
                                key,
                                field_value_pairs,

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
                            CommandType::Hget { key, field, time } => {
                                match soul.hget(key, field, time) {
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
                                }
                            }
                            CommandType::Hmget { key, fields, time } => match soul
                                .hmget(key, fields, time)
                            {
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
                            CommandType::Hdel { key, fields, time } => {
                                match soul.hdel(key, fields, time) {
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
                                }
                            }
                            CommandType::Hexists { key, field, time } => {
                                match soul.hexists(key, field, time) {
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
                                }
                            }
                            CommandType::Hlen { key, time } => match soul.hlen(key, time) {
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
                            CommandType::Lpush {
                                key,
                                elements,

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
                            CommandType::Lpop { key, time } => match soul.lpop(key, time) {
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
                            CommandType::LpopM { key, count, time } => {
                                match soul.lpop_m(key, count, time) {
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
                                }
                            }
                            CommandType::Rpush {
                                key,
                                elements,

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
                            CommandType::Rpop { key, time } => match soul.rpop(key, time) {
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
                            CommandType::RpopM { key, count, time } => {
                                match soul.rpop_m(key, count, time) {
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
                                }
                            }
                            CommandType::Llen { key, time } => match soul.llen(key, time) {
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
                            CommandType::Lrange {
                                key,
                                starting_index,
                                ending_index,
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
                            CommandType::Lindex { key, index, time } => {
                                match soul.lindex(key, index, time) {
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
                                }
                            }
                            CommandType::Lset {
                                key,
                                element,
                                index,

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
                            CommandType::Lrem {
                                key,
                                element,
                                count,
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
                            CommandType::Expire { key, expiry, time } => {
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
                            CommandType::Ttl { key, time } => {
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
                            CommandType::Publish { event, message } => {
                                let clients = client_map.publish(event.clone());

                                if tx
                                    .send(Decree::Broadcast(token, event, message, clients))
                                    .is_err()
                                {
                                    eprintln!("angel panicked");
                                }
                            }
                            CommandType::Mset { terms_iter } => {
                                soul.mset(terms_iter);

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
                            CommandType::Mget { terms_iter, time } => {
                                let bulk_string_array = soul.mget(terms_iter, time);

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
                            CommandType::Sadd { key, values, time } => {
                                match soul.sadd(key, values, time) {
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
                                }
                            }
                            CommandType::Srem { key, values, time } => {
                                match soul.srem(key, values, time) {
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
                                }
                            }
                            CommandType::Sismember { key, value, time } => {
                                match soul.sismember(key, value, time) {
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
                                }
                            }
                            CommandType::Hgetall { key, time } => match soul.hgetall(key, time) {
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
                            CommandType::Smembers { key, time } => match soul.smembers(key, time) {
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
                            _ => {}
                        }
                    }

                    Err(e) => {
                        eprintln!("GodThread: {}", e);
                        break;
                    }
                }
            }
        });

        Temple { tx }
    }

    pub fn get(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Get { key, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn set(&self, key: Vec<u8>, value: (Value, Option<u64>), tx: Sender<Decree>, token: Token) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Set { key, value },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn del(&self, keys: Vec<Vec<u8>>, tx: Sender<Decree>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Del { keys, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn exists(&self, keys: Vec<Vec<u8>>, tx: Sender<Decree>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Exists { keys, time },
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
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Append { key, value, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn incr(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Incr { key, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn decr(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Decr { key, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn strlen(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Strlen { key, time },
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
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Hset {
                    key,

                    field_value_pairs,

                    time,
                },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hget(&self, tx: Sender<Decree>, key: Vec<u8>, field: Vec<u8>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Hget { key, field, time },
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
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Hmget { key, fields, time },
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
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Hdel { key, fields, time },
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
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Hexists { key, field, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hlen(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Hlen { key, time },
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
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Lpush {
                    key,
                    elements,
                    time,
                },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lpop(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Lpop { key, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lpop_m(&self, tx: Sender<Decree>, key: Vec<u8>, count: usize, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::LpopM { key, count, time },
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
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Rpush {
                    key,
                    elements,
                    time,
                },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn rpop(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Rpop { key, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn rpop_m(&self, tx: Sender<Decree>, key: Vec<u8>, count: usize, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::RpopM { key, count, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn llen(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Llen { key, time },
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
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Lrange {
                    key,
                    starting_index,
                    ending_index,
                    time,
                },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn lindex(&self, tx: Sender<Decree>, key: Vec<u8>, index: i32, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Lindex { key, index, time },
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
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Lset {
                    key,

                    element,
                    index,

                    time,
                },
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
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Lrem {
                    key,
                    element,
                    count,
                    time,
                },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn expire(&self, tx: Sender<Decree>, key: Vec<u8>, expiry: u64, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Expire { key, expiry, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn ttl(&self, tx: Sender<Decree>, key: Vec<u8>, token: Token, time: SystemTime) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Ttl { key, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn subscribe(&self, tx: Sender<Decree>, events: Vec<Vec<u8>>, token: Token) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Subscribe { events },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn publish(&self, tx: Sender<Decree>, event: Vec<u8>, message: Vec<u8>, token: Token) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Publish { event, message },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn mset(&self, terms_iter: IntoIter<Vec<u8>>, tx: Sender<Decree>, token: Token) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Mset { terms_iter },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn mget(&self, terms_iter: IntoIter<Vec<u8>>, tx: Sender<Decree>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Mget { terms_iter, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn sadd(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
        token: Token,
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Sadd { key, values, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn srem(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
        token: Token,
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Sadd { key, values, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn sismember(
        &self,
        tx: Sender<Decree>,
        key: Vec<u8>,
        value: Vec<u8>,
        token: Token,
        time: u64,
    ) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Sismember { key, value, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn hgetall(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Hgetall { key, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn smembers(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Smembers { key, time },
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn unsubscribe(&self, tx: Sender<Decree>, token: Token, terms: Vec<Vec<u8>>) {
        if self
            .tx
            .send(Wish {
                tx,
                token,
                command_type: CommandType::Unsubscribe { terms },
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
