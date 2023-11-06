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
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<K, V> Default for DenseMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> DenseMap<K, V>
where
    K: EntityId,
{
    pub fn push(&mut self, elem: V) -> K {
        let curr_len = self.data.len();
        self.data.push(elem);
        K::with_id(curr_len)
    }

    #[inline]
    pub fn get(&self, idx: K) -> Option<&V> {
        self.data.get(idx.get_id())
    }

    #[inline]
    pub fn get_mut(&mut self, idx: K) -> Option<&mut V> {
        self.data.get_mut(idx.get_id())
    }

    #[inline]
    pub fn contains_key(&self, key: K) -> bool {
        key.get_id() < self.data.len()
    }

    #[inline]
    pub fn keys(&self) -> Keys<K> {
        Keys::new(0..self.data.len())
    }

    #[inline]
    pub fn into_keys(self) -> Keys<K> {
        Keys::new(0..self.data.len())
    }

    #[inline]
    pub fn values(&self) -> slice::Iter<'_, V> {
        self.data.iter()
    }

    #[inline]
    pub fn values_mut(&mut self) -> slice::IterMut<'_, V> {
        self.data.iter_mut()
    }

    #[inline]
    pub fn into_values(self) -> vec::IntoIter<V> {
        self.data.into_iter()
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter::new(self.data.iter())
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut::new(self.data.iter_mut())
    }
}

impl<K, V> fmt::Debug for DenseMap<K, V>
where
    K: EntityId + fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

pub struct Keys<K> {
    inner: ops::Range<usize>,
    _marker: PhantomData<K>,
}

impl<K> Keys<K> {
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

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(K::with_id)
    }
}

pub struct Iter<'a, K, V> {
    inner: iter::Enumerate<slice::Iter<'a, V>>,
    _marker: PhantomData<K>,
}

impl<'a, K, V> Iter<'a, K, V> {
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

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(id, val)| (K::with_id(id), val))
    }
}

pub struct IterMut<'a, K, V> {
    inner: iter::Enumerate<slice::IterMut<'a, V>>,
    _marker: PhantomData<K>,
}

impl<'a, K, V> IterMut<'a, K, V> {
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

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self.data.into_iter())
    }
}

impl<K, V> FromIterator<V> for DenseMap<K, V>
where
    K: EntityId,
{
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
