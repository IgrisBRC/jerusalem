
use std::collections::hash_map::Entry;
use std::{collections::HashMap, time::SystemTime};
type GodMap = HashMap<String, (Vec<u8>, Option<SystemTime>)>;

#[derive(Debug)]
pub struct MemoryDatabase {
    _name: String,
    map: GodMap,
}

impl MemoryDatabase {
    pub fn new(db_name: &str) -> Self {
        MemoryDatabase {
            _name: db_name.to_string(),
            map: GodMap::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        match self.map.entry(key.to_string()) {
            Entry::Occupied(occupied) => {
                let (_, expiry) = occupied.get();

                if let Some(time) = expiry {
                    if *time < SystemTime::now() {
                        occupied.remove(); 
                        return None;
                    }
                }

                Some(occupied.get().0.clone())
            }
            Entry::Vacant(_) => None,
        }
    }

    pub fn insert(
        &mut self,
        key: &str,
        val: (Vec<u8>, Option<SystemTime>),
    ) -> Option<(Vec<u8>, Option<SystemTime>)> {
        self.map.insert(key.to_string(), val)
    }

    pub fn remove(
        &mut self,
        key: &str,
    ) -> Option<(Vec<u8>, Option<SystemTime>)>{
        self.map.remove(key)
    }
}

