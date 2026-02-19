use std::collections::hash_map::Entry;
use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender};
use std::{collections::HashMap, time::SystemTime};

#[derive(Clone)]
pub enum Value {
    String(Vec<u8>),
    List(VecDeque<Vec<u8>>),
    Hash(HashMap<Vec<u8>, Vec<u8>>),
    Set(HashSet<Vec<u8>>),
}

pub struct Soul(HashMap<Vec<u8>, (Value, Option<SystemTime>)>);

impl Soul {
    pub fn new() -> Self {
        Soul(HashMap::new())
    }

    pub fn get(&mut self, key: Vec<u8>) -> Option<(Value, Option<SystemTime>)> {
        match self.0.entry(key) {
            Entry::Occupied(occupied) => {
                let (data, expiry) = occupied.get();

                if let Some(time) = expiry {
                    if *time < SystemTime::now() {
                        occupied.remove();
                        return None;
                    }
                }

                Some((data.clone(), *expiry))
            }
            Entry::Vacant(_) => None,
        }
    }

    pub fn insert(
        &mut self,
        key: Vec<u8>,
        val: (Value, Option<SystemTime>),
    ) -> Option<(Value, Option<SystemTime>)> {
        self.0.insert(key, val)
    }

    pub fn remove(&mut self, key: Vec<u8>) -> Option<(Value, Option<SystemTime>)> {
        self.0.remove(&key)
    }
}

pub enum Wish {
    Get {
        key: Vec<u8>,
        tx: Sender<Option<(Value, Option<SystemTime>)>>,
    },
    Insert {
        key: Vec<u8>,
        val: (Value, Option<SystemTime>),
        tx: Sender<Option<(Value, Option<SystemTime>)>>,
    },
    Remove {
        key: Vec<u8>,
        tx: Sender<Option<(Value, Option<SystemTime>)>>,
    },
}

#[derive(Clone)]
pub struct Temple {
    name: String,
    tx: Sender<Wish>,
}

impl Temple {
    pub fn new(name: String) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut soul = Soul::new();

            while let Ok(wish) = rx.recv() {
                match wish {
                    Wish::Get { key, tx } => {
                        if tx.send(soul.get(key)).is_err() {
                            eprintln!("angel panicked");
                        }
                    }
                    Wish::Insert { key, val, tx } => {
                        if tx.send(soul.insert(key, val)).is_err() {
                            eprintln!("angel panicked");
                        }
                    }
                    Wish::Remove { key, tx } => {
                        if tx.send(soul.remove(key)).is_err() {
                            eprintln!("angel panicked");
                        }
                    }
                }
            }
        });

        Temple { name, tx }
    }

    pub fn get(
        &self,
        key: Vec<u8>,
        tx: Sender<Option<(Value, Option<SystemTime>)>>,
        rx: &Receiver<Option<(Value, Option<SystemTime>)>>,
    ) -> Option<(Value, Option<SystemTime>)> {
        if self.tx.send(Wish::Get { key, tx }).is_err() {
            eprintln!("angel panicked");
        }

        rx.recv().unwrap_or(None)
    }

    pub fn insert(
        &self,
        key: Vec<u8>,
        value: (Value, Option<SystemTime>),
        tx: Sender<Option<(Value, Option<SystemTime>)>>,
        rx: &Receiver<Option<(Value, Option<SystemTime>)>>,
    ) -> Option<(Value, Option<SystemTime>)> {
        if self.tx.send(Wish::Insert {
            key,
            val: value,
            tx,
        }).is_err() {
            eprintln!("angel panicked");
        }


        rx.recv().unwrap_or(None)
    }

    pub fn remove(
        &self,
        key: Vec<u8>,
        tx: Sender<Option<(Value, Option<SystemTime>)>>,
        rx: &Receiver<Option<(Value, Option<SystemTime>)>>,
    ) -> Option<(Value, Option<SystemTime>)> {
        if self.tx.send(Wish::Remove { key, tx }).is_err() {
            eprintln!("angel panicked");
        }


        rx.recv().unwrap_or(None)
    }

    pub fn sanctify(&self) -> Self {
        self.clone()
    }
}
