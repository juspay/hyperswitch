use std::sync::{Arc, RwLock};

use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;

#[derive(Debug)]
pub struct CacheEntry<T> {
    data: Arc<T>,
    timestamp: i64,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CacheError {
    #[error("Could not acquire the lock for cache entry")]
    CouldNotAcquireLock,
    #[error("Entry not found in cache")]
    EntryNotFound,
}

#[derive(Debug)]
pub struct StaticCache<T> {
    data: Lazy<RwLock<FxHashMap<String, CacheEntry<T>>>>,
}

impl<T> StaticCache<T>
where
    T: Send,
{
    pub const fn new() -> Self {
        Self {
            data: Lazy::new(|| RwLock::new(FxHashMap::default())),
        }
    }

    pub fn present(&self, key: &String) -> Result<bool, CacheError> {
        let the_map = self
            .data
            .read()
            .map_err(|_| CacheError::CouldNotAcquireLock)?;

        Ok(the_map.get(key).is_some())
    }

    pub fn expired(&self, key: &String, timestamp: i64) -> Result<bool, CacheError> {
        let the_map = self
            .data
            .read()
            .map_err(|_| CacheError::CouldNotAcquireLock)?;

        Ok(match the_map.get(key) {
            None => false,
            Some(entry) => timestamp > entry.timestamp,
        })
    }

    pub fn retrieve(&self, key: &String) -> Result<Arc<T>, CacheError> {
        let the_map = self
            .data
            .read()
            .map_err(|_| CacheError::CouldNotAcquireLock)?;

        let cache_entry = the_map.get(key).ok_or(CacheError::EntryNotFound)?;

        Ok(Arc::clone(&cache_entry.data))
    }

    pub fn save(&self, key: String, data: T, timestamp: i64) -> Result<(), CacheError> {
        let mut the_map = self
            .data
            .write()
            .map_err(|_| CacheError::CouldNotAcquireLock)?;

        let entry = CacheEntry {
            data: Arc::new(data),
            timestamp,
        };

        the_map.insert(key, entry);
        Ok(())
    }

    pub fn clear(&self) -> Result<(), CacheError> {
        let mut the_map = self
            .data
            .write()
            .map_err(|_| CacheError::CouldNotAcquireLock)?;

        the_map.clear();
        Ok(())
    }
}
