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
        /// Creates a new instance of the current type, initializing it with a Lazy-initialized RwLock-protected FxHashMap with default values.
    pub const fn new() -> Self {
        Self {
            data: Lazy::new(|| RwLock::new(FxHashMap::default())),
        }
    }

        /// Checks if the given key is present in the cache and returns a boolean value.
    /// 
    /// # Arguments
    /// 
    /// * `key` - A reference to a String representing the key to be searched in the cache.
    /// 
    /// # Returns
    /// 
    /// * `Result<bool, CacheError>` - A Result enum with a boolean value indicating whether the key is present in the cache or not, or a CacheError if the lock on the cache data could not be acquired.
    /// 
    pub fn present(&self, key: &String) -> Result<bool, CacheError> {
        let the_map = self
            .data
            .read()
            .map_err(|_| CacheError::CouldNotAcquireLock)?;
    
        Ok(the_map.get(key).is_some())
    }

        /// Checks if the entry associated with the given key in the cache has expired based on the provided timestamp.
    /// If the entry does not exist in the cache, it is considered expired.
    ///
    /// # Arguments
    ///
    /// * `key` - A reference to a String representing the key of the entry in the cache.
    /// * `timestamp` - An i64 representing the timestamp to compare with the entry's timestamp.
    ///
    /// # Returns
    ///
    /// * If the entry exists and its timestamp is less than or equal to the provided timestamp, returns a Result containing a boolean value indicating whether the entry has expired or not. If the entry does not exist in the cache, returns Ok(false).
    /// * If an error occurs while acquiring the read lock on the cache, returns a CacheError.
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

        /// Retrieves the value associated with the given key from the cache.
    /// If the key is found in the cache, returns a reference-counted pointer to the value.
    /// If the key is not found in the cache, returns a CacheError::EntryNotFound error.
    pub fn retrieve(&self, key: &String) -> Result<Arc<T>, CacheError> {
        let the_map = self
            .data
            .read()
            .map_err(|_| CacheError::CouldNotAcquireLock)?;

        let cache_entry = the_map.get(key).ok_or(CacheError::EntryNotFound)?;

        Ok(Arc::clone(&cache_entry.data))
    }

        /// Saves the provided data with the specified key and timestamp into the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - A String representing the key for the data
    /// * `data` - The data to be saved in the cache
    /// * `timestamp` - An i64 representing the timestamp for the data
    ///
    /// # Returns
    ///
    /// * `Result<(), CacheError>` - A Result indicating success if the data was saved successfully, or a CacheError if an error occurred
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

        /// Clears the cache by removing all key-value pairs.
    ///
    /// # Returns
    /// 
    /// * `Result<(), CacheError>`: A result indicating success if the cache is cleared
    ///   successfully, or a `CacheError` if there was a problem acquiring the lock on the cache data.
    pub fn clear(&self) -> Result<(), CacheError> {
        let mut the_map = self
            .data
            .write()
            .map_err(|_| CacheError::CouldNotAcquireLock)?;

        the_map.clear();
        Ok(())
    }
}
