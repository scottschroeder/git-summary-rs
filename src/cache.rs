use std::collections::HashMap;
use std::default::Default;
use std::fmt;
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};

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
    K: Hash + Eq,
    V: Clone,
{
    pub fn get<F>(&self, key: &K, func: F) -> V
    where
        K: Clone + fmt::Debug,
        F: Fn(K) -> V,
    {
        let mut created_entry = None;
        let mut writer = None;
        let lookup_entry = {
            let mut map = self.map.lock().unwrap();
            let x = map.entry(key.clone()).or_insert_with(|| {
                let inner = Arc::new(RwLock::new(None));
                created_entry = Some(inner.clone());
                writer = Some(created_entry.as_ref().unwrap().write().unwrap());
                inner
            });
            x.clone()
        };

        match writer {
            Some(mut guard) => {
                // Stable in 1.31
                // guard.replace(value);
                trace!("Performing function call for {:?}", key);
                *guard = Some(func(key.clone()));
                guard.as_ref().unwrap().clone()
            }

            None => {
                trace!("Looking up {:?}", key);
                lookup_entry.read().unwrap().clone().unwrap()
            }
        }
    }
}
