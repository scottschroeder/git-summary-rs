use std::{
    collections::{hash_map::Entry, HashMap},
    fmt,
    hash::Hash,
    sync::{Arc, Mutex, RwLock},
};

#[derive(Debug)]
pub struct Cache<K: Hash + Eq, V> {
    map: Arc<Mutex<HashMap<K, Arc<RwLock<Option<V>>>>>>,
}

impl<K, V> Clone for Cache<K, V>
where
    K: Hash + Eq,
{
    fn clone(&self) -> Cache<K, V> {
        Cache {
            map: self.map.clone(),
        }
    }
}

impl<K, V> Default for Cache<K, V>
where
    K: Hash + Eq,
{
    fn default() -> Cache<K, V> {
        Cache {
            map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<K, V> Cache<K, V>
where
    K: Hash + Eq + Clone + fmt::Debug,
    V: Clone,
{
    pub fn get_or_insert_with<F>(&self, key: &K, func: &F) -> V
    where
        F: Fn(K) -> V,
    {
        let entry;
        let mut writer = None;
        {
            let mut map = self.map.lock().unwrap();
            match map.entry(key.clone()) {
                Entry::Occupied(oe) => entry = oe.get().clone(),
                Entry::Vacant(ve) => {
                    entry = ve.insert(Arc::new(RwLock::new(None))).clone();
                    writer = Some(entry.as_ref().write().unwrap());
                }
            }
        }

        match writer {
            // We are the first one, and we have the writer
            // We need to perform the function call and store the result
            Some(mut guard) => {
                // Stable in 1.31
                // guard.replace(value);
                trace!("Performing function call for {:?}", key);
                *guard = Some(func(key.clone()));
                guard.as_ref().unwrap().clone()
            }

            // We just wait for a read lock and return the value
            None => {
                trace!("Looking up {:?}", key);
                entry.read().unwrap().clone().unwrap()
            }
        }
    }
}
