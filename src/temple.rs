use crate::temple::{
    BroadcastCommand::{Publish, Subscribe, Unsubscribe},
    ClientCommandType::{Broadcast, Database},
    ServerCommand::Save,
};
use crate::temple::{
    CommandType::{Client, Server},
    DatabaseCommand::{
        Append, Decr, Del, Exists, Expire, Get, Hdel, Hexists, Hget, Hgetall, Hlen, Hmget, Hset,
        Incr, Lindex, Llen, Lpop, LpopM, Lpush, Lrange, Lrem, Lset, Mget, Mset, Rpop, RpopM, Rpush,
        Sadd, Set, Sismember, Smembers, Srem, Strlen, Ttl,
    },
};

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::vec::IntoIter;
use std::{collections::HashMap, time::SystemTime};

use mio::Token;
use rkyv::api::low::deserialize;
use rkyv::rancor::Error;

use crate::temple::soul::SaveError;
use crate::wish::grant::{Decree, Gift};
use crate::wish::{InfoType, Response, Sacrilege};

pub struct EventMap(HashMap<Token, HashSet<Vec<u8>>>);
pub struct ClientMap(HashMap<Vec<u8>, HashSet<Token>>);

pub mod soul;

use soul::{ArchivedSoul, Soul, Value};

// pub struct Shrine {
//     file_path: PathBuf,
//     ipv4_address: String,
//     port: u16,
//     io_threads: usize,
//     event_capacity: usize,
// }

// impl Shrine {
//     pub fn new(
//         file_path: PathBuf,
//         ipv4_address: String,
//         port: u16,
//         io_threads: usize,
//         event_capacity: usize,
//     ) -> Self {
//         Shrine {
//             file_path,
//             ipv4_address,
//             port,
//             io_threads,
//             event_capacity,
//         }
//     }
// }

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
    command_type: CommandType,
}

#[derive(Clone)]
pub enum CommandType {
    Server(ServerCommand),
    Client(ClientCommand),
}

#[derive(Clone)]
pub enum ServerCommand {
    Save {
        tx: Sender<Result<(), SaveError>>,
        file_path: PathBuf,
    },
}

#[derive(Clone)]
pub struct ClientCommand {
    tx: Sender<Decree>,
    client_command_type: ClientCommandType,
}

#[derive(Clone)]
pub enum ClientCommandType {
    Database(DatabaseCommand),
    Broadcast(BroadcastCommand),
}

#[derive(Clone)]
pub enum BroadcastCommand {
    Subscribe { events: Vec<Vec<u8>> },
    Publish { event: Vec<u8>, message: Vec<u8> },
    Unsubscribe { terms: Vec<Vec<u8>> },
}

#[derive(Clone)]
pub enum DatabaseCommand {
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
}

#[derive(Clone)]
pub struct Temple {
    file_path: PathBuf,
    tx: Sender<Wish>,
}

impl Default for Temple {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap())
    }
}

impl Temple {
    pub fn new(file_path: PathBuf) -> Self {
        let (tx, rx): (Sender<Wish>, Receiver<Wish>) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut soul: Soul = (|| {
                let Ok(bytes) = std::fs::read("/home/Igris/RustProjects/mini_redis/dump.rdb")
                else {
                    return Soul::new();
                };

                let Ok(archived_soul) = rkyv::access::<ArchivedSoul, Error>(&bytes) else {
                    return Soul::new();
                };

                match deserialize::<_, Error>(archived_soul) {
                    Ok(snapshot) => {
                        println!("Snapshot loaded successfully");
                        snapshot
                    }
                    Err(e) => {
                        println!("Couldn't load snapshot: {}", e);
                        Soul::new()
                    }
                }
            })();

            let mut client_map = ClientMap::new();
            let mut event_map = EventMap::new();
            let mut subscribed_clients = HashSet::new();

            loop {
                match rx.recv() {
                    Ok(wish) => {
                        let token = wish.token;

                        let command_type = wish.command_type;

                        match command_type {
                            Server(server_command) => match server_command {
                                Save { tx, file_path } => {
                                    if tx.send(soul.save(file_path)).is_err() {
                                        eprintln!("angel panicked");
                                    }

                                    break;
                                }
                            },
                            Client(client_command) => {
                                let tx = client_command.tx;

                                match client_command.client_command_type {
                                    Broadcast(broadcast_command) => match broadcast_command {
                                        Subscribe { events } => {
                                            subscribed_clients.insert(token);

                                            let subscribed_channels =
                                                event_map.subscribe(token, events.clone());
                                            client_map.subscribe(token, events.clone());

                                            if tx
                                                .send(Decree::Deliver(Gift {
                                                    token,
                                                    response: Response::SubscribedChannels(
                                                        subscribed_channels,
                                                    ),
                                                }))
                                                .is_err()
                                            {
                                                eprintln!("angel panicked");
                                            }

                                            continue;
                                        }
                                        Unsubscribe { terms } => {
                                            let unsubscribed_events = event_map.unsubscribe(
                                                terms,
                                                token,
                                                &mut subscribed_clients,
                                            );
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
                                        Publish { event, message } => {
                                            let clients = client_map.publish(event.clone());

                                            if tx
                                                .send(Decree::Broadcast(
                                                    token, event, message, clients,
                                                ))
                                                .is_err()
                                            {
                                                eprintln!("angel panicked");
                                            }
                                        }
                                    },
                                    Database(database_command) => {
                                        if subscribed_clients.contains(&token) {
                                            if tx
                                                .send(Decree::Deliver(Gift {
                                                    token,
                                                    response: Response::Error(
                                                        Sacrilege::SubscriberOnlyMode,
                                                    ),
                                                }))
                                                .is_err()
                                            {
                                                eprintln!("angel panicked");
                                            }

                                            continue;
                                        }
                                        match database_command {
                                            Get { key, time } => match soul.get(key, time) {
                                                Ok(bulk_string) => {
                                                    if tx
                                                        .send(Decree::Deliver(Gift {
                                                            token,
                                                            response: Response::BulkString(
                                                                bulk_string,
                                                            ),
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
                                            Set { key, value: val } => {
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
                                            Del { keys, time } => {
                                                if tx
                                                    .send(Decree::Deliver(Gift {
                                                        token,
                                                        response: Response::Amount(
                                                            soul.del(keys, time),
                                                        ),
                                                    }))
                                                    .is_err()
                                                {
                                                    eprintln!("angel panicked");
                                                }
                                            }
                                            Append { key, value, time } => {
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }

                                            Incr { key, time } => match soul.incr(key, time) {
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
                                            Decr { key, time } => match soul.decr(key, time) {
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
                                            Strlen { key, time } => match soul.strlen(key, time) {
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
                                            Exists { keys, time } => {
                                                if tx
                                                    .send(Decree::Deliver(Gift {
                                                        token,
                                                        response: Response::Amount(
                                                            soul.exists(keys, time),
                                                        ),
                                                    }))
                                                    .is_err()
                                                {
                                                    eprintln!("angel panicked");
                                                }
                                            }
                                            Hset {
                                                key,
                                                field_value_pairs,

                                                time,
                                            } => match soul.hset(key, field_value_pairs, time) {
                                                Ok(new_values_added) => {
                                                    if tx
                                                        .send(Decree::Deliver(Gift {
                                                            token,
                                                            response: Response::Amount(
                                                                new_values_added,
                                                            ),
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
                                            Hget { key, field, time } => {
                                                match soul.hget(key, field, time) {
                                                    Ok(bulk_string) => {
                                                        if tx
                                                            .send(Decree::Deliver(Gift {
                                                                token,
                                                                response: Response::BulkString(
                                                                    bulk_string,
                                                                ),
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }
                                            Hmget { key, fields, time } => match soul
                                                .hmget(key, fields, time)
                                            {
                                                Ok(bulk_string_array) => {
                                                    if tx
                                                        .send(Decree::Deliver(Gift {
                                                            token,
                                                            response: Response::BulkStringArray(
                                                                bulk_string_array,
                                                            ),
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
                                            Hdel { key, fields, time } => {
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }
                                            Hexists { key, field, time } => match soul
                                                .hexists(key, field, time)
                                            {
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
                                            Hlen { key, time } => match soul.hlen(key, time) {
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
                                            Lpush {
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
                                            Lpop { key, time } => match soul.lpop(key, time) {
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
                                            LpopM { key, count, time } => {
                                                match soul.lpop_m(key, count, time) {
                                                    Ok(elements) => {
                                                        if tx
                                                            .send(Decree::Deliver(Gift {
                                                                token,
                                                                response: Response::BulkStringArray(
                                                                    elements,
                                                                ),
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }
                                            Rpush {
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
                                            Rpop { key, time } => match soul.rpop(key, time) {
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
                                            RpopM { key, count, time } => {
                                                match soul.rpop_m(key, count, time) {
                                                    Ok(elements) => {
                                                        if tx
                                                            .send(Decree::Deliver(Gift {
                                                                token,
                                                                response: Response::BulkStringArray(
                                                                    elements,
                                                                ),
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }
                                            Llen { key, time } => match soul.llen(key, time) {
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
                                            Lrange {
                                                key,
                                                starting_index,
                                                ending_index,
                                                time,
                                            } => match soul.lrange(
                                                key,
                                                starting_index,
                                                ending_index,
                                                time,
                                            ) {
                                                Ok(bulk_string_array) => {
                                                    if tx
                                                        .send(Decree::Deliver(Gift {
                                                            token,
                                                            response: Response::BulkStringArray(
                                                                bulk_string_array,
                                                            ),
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
                                            Lindex { key, index, time } => {
                                                match soul.lindex(key, index, time) {
                                                    Ok(element) => {
                                                        if tx
                                                            .send(Decree::Deliver(Gift {
                                                                token,
                                                                response: Response::BulkString(
                                                                    element,
                                                                ),
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }
                                            Lset {
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
                                            Lrem {
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
                                            Expire { key, expiry, time } => {
                                                if tx
                                                    .send(Decree::Deliver(Gift {
                                                        token,
                                                        response: Response::Amount(
                                                            soul.expire(key, expiry, time),
                                                        ),
                                                    }))
                                                    .is_err()
                                                {
                                                    eprintln!("angel panicked");
                                                }
                                            }
                                            Ttl { key, time } => {
                                                if tx
                                                    .send(Decree::Deliver(Gift {
                                                        token,
                                                        response: Response::Number(
                                                            soul.ttl(key, time),
                                                        ),
                                                    }))
                                                    .is_err()
                                                {
                                                    eprintln!("angel panicked");
                                                }
                                            }
                                            Mset { terms_iter } => {
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
                                            Mget { terms_iter, time } => {
                                                let bulk_string_array = soul.mget(terms_iter, time);

                                                if tx
                                                    .send(Decree::Deliver(Gift {
                                                        token,
                                                        response: Response::BulkStringArray(
                                                            bulk_string_array,
                                                        ),
                                                    }))
                                                    .is_err()
                                                {
                                                    eprintln!("angel panicked");
                                                }
                                            }
                                            Sadd { key, values, time } => {
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }
                                            Srem { key, values, time } => {
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }
                                            Sismember { key, value, time } => {
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }
                                            Hgetall { key, time } => {
                                                match soul.hgetall(key, time) {
                                                    Ok(bulk_string_array) => {
                                                        if tx
                                                            .send(Decree::Deliver(Gift {
                                                                token,
                                                                response: Response::BulkStringArray(
                                                                    bulk_string_array,
                                                                ),
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }
                                            Smembers { key, time } => {
                                                match soul.smembers(key, time) {
                                                    Ok(bulk_string_array) => {
                                                        if tx
                                                            .send(Decree::Deliver(Gift {
                                                                token,
                                                                response: Response::BulkStringArray(
                                                                    bulk_string_array,
                                                                ),
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
                                                                response: Response::Error(
                                                                    sacrilege,
                                                                ),
                                                            }))
                                                            .is_err()
                                                        {
                                                            eprintln!("angel panicked");
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("GodThread: {}", e);
                        break;
                    }
                }
            }
        });

        Temple { file_path, tx }
    }

    pub fn get(&self, key: Vec<u8>, tx: Sender<Decree>, token: Token, time: u64) {
        if self
            .tx
            .send(Wish {
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Get { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Set { key, value }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Del { keys, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Exists { keys, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Append { key, value, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Incr { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Decr { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Strlen { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Hset {
                        key,
                        field_value_pairs,
                        time,
                    }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Hget { key, field, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Hmget { key, fields, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Hdel { key, fields, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Hexists { key, field, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Hlen { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Lpush {
                        key,
                        elements,
                        time,
                    }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Lpop { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(LpopM { key, count, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Rpush {
                        key,
                        elements,
                        time,
                    }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Rpop { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(RpopM { key, count, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Llen { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Lrange {
                        key,
                        starting_index,
                        ending_index,
                        time,
                    }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Lindex { key, index, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Lset {
                        key,
                        index,
                        element,
                        time,
                    }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Lrem {
                        key,
                        count,
                        element,
                        time,
                    }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Expire { key, expiry, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Ttl { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Mset { terms_iter }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Mget { terms_iter, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Sadd { key, values, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Srem { key, values, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Sismember { key, value, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Hgetall { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Database(Smembers { key, time }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Broadcast(Subscribe { events }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Broadcast(Publish { event, message }),
                }),
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
                token,
                command_type: CommandType::Client(ClientCommand {
                    tx,
                    client_command_type: Broadcast(Unsubscribe { terms }),
                }),
            })
            .is_err()
        {
            eprintln!("angel panicked");
        }
    }

    pub fn save(&mut self, tx: Sender<Result<(), SaveError>>, token: Token) {
        if self
            .tx
            .send(Wish {
                token,
                command_type: CommandType::Server(Save {
                    tx,
                    file_path: self.file_path.to_path_buf(),
                }),
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
