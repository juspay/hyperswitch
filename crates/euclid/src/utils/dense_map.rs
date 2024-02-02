use std::{fmt, iter, marker::PhantomData, ops, slice, vec};

pub trait EntityId {
    fn get_id(&self) -> usize;
    fn with_id(id: usize) -> Self;
}

pub struct DenseMap<K, V> {
    data: Vec<V>,
    _marker: PhantomData<K>,
}

impl<K, V> DenseMap<K, V> {
        /// Creates a new instance of the struct.
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<K, V> Default for DenseMap<K, V> {
        /// This method returns an instance of the current struct with default values by calling the `new` method.
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> DenseMap<K, V>
where
    K: EntityId,
{
        /// Adds an element to the end of the data vector and returns a new instance of K with the current length of the data vector as its id.
    pub fn push(&mut self, elem: V) -> K {
        let curr_len = self.data.len();
        self.data.push(elem);
        K::with_id(curr_len)
    }

    #[inline]
        /// Retrieves the value at the specified index from the data structure, if it exists.
    /// 
    /// # Arguments
    /// 
    /// * `idx` - The index at which to retrieve the value
    /// 
    /// # Returns
    /// 
    /// * `Option<&V>` - Some reference to the value if it exists at the specified index, otherwise None
    pub fn get(&self, idx: K) -> Option<&V> {
        self.data.get(idx.get_id())
    }

    #[inline]
        /// Retrieves a mutable reference to the value associated with the given key from the data structure, if it exists.
    /// 
    /// # Arguments
    /// * `idx` - The key for which the mutable reference to the associated value should be retrieved.
    /// 
    /// # Returns
    /// If a value is associated with the given key, a mutable reference to that value is returned. If no value is associated with the key, `None` is returned.
    pub fn get_mut(&mut self, idx: K) -> Option<&mut V> {
        self.data.get_mut(idx.get_id())
    }

    #[inline]
        /// Checks if the given key exists in the data structure.
    /// 
    /// # Arguments
    /// 
    /// * `key` - The key to be checked for existence in the data structure.
    /// 
    /// # Returns
    /// 
    /// Returns true if the key exists in the data structure, otherwise false.
    pub fn contains_key(&self, key: K) -> bool {
        key.get_id() < self.data.len()
    }

    #[inline]
        /// Returns an iterator over the keys of the data stored in the structure.
    pub fn keys(&self) -> Keys<K> {
        Keys::new(0..self.data.len())
    }

    #[inline]
        /// Converts the data in the current structure into a collection of keys, where each key represents the index of the corresponding element in the data.
    pub fn into_keys(self) -> Keys<K> {
        Keys::new(0..self.data.len())
    }

    #[inline]
        /// Returns an iterator over the values in the data structure.
    pub fn values(&self) -> slice::Iter<'_, V> {
        self.data.iter()
    }

    #[inline]
        /// Returns a mutable iterator over the values of the data stored in the struct.
    pub fn values_mut(&mut self) -> slice::IterMut<'_, V> {
        self.data.iter_mut()
    }

    #[inline]
        /// Consumes the current data structure and returns an iterator over the values it contains.
    pub fn into_values(self) -> vec::IntoIter<V> {
        self.data.into_iter()
    }

    #[inline]
        /// Returns an iterator over the key-value pairs of the HashMap.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter::new(self.data.iter())
    }

    #[inline]
        /// Returns an iterator over mutable references to the key-value pairs in the map.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut::new(self.data.iter_mut())
    }
}

impl<K, V> fmt::Debug for DenseMap<K, V>
where
    K: EntityId + fmt::Debug,
    V: fmt::Debug,
{
        /// Formats the data structure using the given formatter.
    /// 
    /// This method formats the data structure by creating a debug representation of the elements
    /// and key-value pairs contained within it, and then writes this representation to the given
    /// formatter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

pub struct Keys<K> {
    inner: ops::Range<usize>,
    _marker: PhantomData<K>,
}

impl<K> Keys<K> {
        /// Creates a new instance of Self with the specified range.
    fn new(range: ops::Range<usize>) -> Self {
        Self {
            inner: range,
            _marker: PhantomData,
        }
    }
}

impl<K> Iterator for Keys<K>
where
    K: EntityId,
{
    type Item = K;

        /// Takes a mutable reference to self and returns the next item in the iterator wrapped in an Option.
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(K::with_id)
    }
}

pub struct Iter<'a, K, V> {
    inner: iter::Enumerate<slice::Iter<'a, V>>,
    _marker: PhantomData<K>,
}

impl<'a, K, V> Iter<'a, K, V> {
        /// Creates a new instance of Self with the given slice iterator.
    fn new(iter: slice::Iter<'a, V>) -> Self {
        Self {
            inner: iter.enumerate(),
            _marker: PhantomData,
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: EntityId,
{
    type Item = (K, &'a V);

        /// Retrieves the next element from the iterator and maps its key and value using the `with_id`
    /// method of the `K` type.
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(id, val)| (K::with_id(id), val))
    }
}

pub struct IterMut<'a, K, V> {
    inner: iter::Enumerate<slice::IterMut<'a, V>>,
    _marker: PhantomData<K>,
}

impl<'a, K, V> IterMut<'a, K, V> {
        /// Creates a new instance of Self with the given mutable slice iterator.
    fn new(iter: slice::IterMut<'a, V>) -> Self {
        Self {
            inner: iter.enumerate(),
            _marker: PhantomData,
        }
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V>
where
    K: EntityId,
{
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(id, val)| (K::with_id(id), val))
    }
}

pub struct IntoIter<K, V> {
    inner: iter::Enumerate<vec::IntoIter<V>>,
    _marker: PhantomData<K>,
}

impl<K, V> IntoIter<K, V> {
        /// Creates a new instance of Self using the provided iterator. The iterator is consumed and its elements are enumerated, and the resulting iterator is stored within the inner field of the new instance. The _marker field is initialized with PhantomData.
    fn new(iter: vec::IntoIter<V>) -> Self {
        Self {
            inner: iter.enumerate(),
            _marker: PhantomData,
        }
    }
}

impl<K, V> Iterator for IntoIter<K, V>
where
    K: EntityId,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(id, val)| (K::with_id(id), val))
    }
}

impl<K, V> IntoIterator for DenseMap<K, V>
where
    K: EntityId,
{
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

        /// Converts the data into an iterator, consuming the original collection.
    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self.data.into_iter())
    }
}

impl<K, V> FromIterator<V> for DenseMap<K, V>
where
    K: EntityId,
{
        /// Constructs a new instance of `Self` from the elements of the provided iterator.
    /// 
    /// # Arguments
    /// 
    /// * `iter` - An iterator yielding elements of type `V`.
    /// 
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = V>,
    {
        Self {
            data: Vec::from_iter(iter),
            _marker: PhantomData,
        }
    }
}
