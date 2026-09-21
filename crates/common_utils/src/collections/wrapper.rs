//! `HashMap` and `HashSet` as types of our own, so that `new()` can exist.
//!
//! `std` defines `new`, `with_capacity` and `From<[_; N]>` only for its own
//! default hasher, and an alias cannot add them. A wrapper can, which leaves a
//! call site nothing to change but its import. Everything else is `std`'s,
//! reached through `Deref`.

// The one place that has to name what it wraps.
#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::{
    borrow::Borrow,
    collections::{hash_map, hash_set},
    fmt,
    hash::{BuildHasher, Hash},
    ops::{Deref, DerefMut, Index},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::DefaultHashBuilder;

/// [`std::collections::HashMap`] with [`DefaultHashBuilder`] as its default
/// hasher.
#[repr(transparent)]
pub struct HashMap<K, V, S = DefaultHashBuilder>(std::collections::HashMap<K, V, S>);

/// [`std::collections::HashSet`] with [`DefaultHashBuilder`] as its default
/// hasher.
#[repr(transparent)]
pub struct HashSet<T, S = DefaultHashBuilder>(std::collections::HashSet<T, S>);

impl<K, V> HashMap<K, V, DefaultHashBuilder> {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty map with room for `capacity` entries.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, DefaultHashBuilder::default())
    }
}

impl<K, V, S> HashMap<K, V, S> {
    /// An empty map that hashes with `hasher`.
    pub fn with_hasher(hasher: S) -> Self {
        Self(std::collections::HashMap::with_hasher(hasher))
    }

    /// An empty map with room for `capacity` entries, hashing with `hasher`.
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self(std::collections::HashMap::with_capacity_and_hasher(
            capacity, hasher,
        ))
    }

    /// Whether the map is empty. Inherent so that a path such as
    /// `HashMap::is_empty` resolves, which `Deref` alone does not give.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of entries. Inherent for the same reason as [`Self::is_empty`].
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// The `std` map, for an API that names it.
    pub fn into_inner(self) -> std::collections::HashMap<K, V, S> {
        self.0
    }

    /// The keys, by value.
    pub fn into_keys(self) -> hash_map::IntoKeys<K, V> {
        self.0.into_keys()
    }

    /// The values, by value.
    pub fn into_values(self) -> hash_map::IntoValues<K, V> {
        self.0.into_values()
    }
}

impl<K, V, S> Deref for HashMap<K, V, S> {
    type Target = std::collections::HashMap<K, V, S>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V, S> DerefMut for HashMap<K, V, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, V, S: Default> Default for HashMap<K, V, S> {
    fn default() -> Self {
        Self(std::collections::HashMap::default())
    }
}

impl<K: Clone, V: Clone, S: Clone> Clone for HashMap<K, V, S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for HashMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq for HashMap<K, V, S> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for HashMap<K, V, S> {}

impl<K: Eq + Hash, V, S: BuildHasher + Default> FromIterator<(K, V)> for HashMap<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> Extend<(K, V)> for HashMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}

impl<'a, K: Eq + Hash + Copy, V: Copy, S: BuildHasher> Extend<(&'a K, &'a V)> for HashMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}

impl<K, V, S> IntoIterator for HashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = hash_map::IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = hash_map::Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = hash_map::IterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<K, Q, V, S> Index<&Q> for HashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
    S: BuildHasher,
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.0.index(key)
    }
}

impl<K: Eq + Hash, V, const N: usize> From<[(K, V); N]> for HashMap<K, V, DefaultHashBuilder> {
    fn from(entries: [(K, V); N]) -> Self {
        entries.into_iter().collect()
    }
}

impl<K, V, S> From<std::collections::HashMap<K, V, S>> for HashMap<K, V, S> {
    fn from(map: std::collections::HashMap<K, V, S>) -> Self {
        Self(map)
    }
}

impl<K, V, S> From<HashMap<K, V, S>> for std::collections::HashMap<K, V, S> {
    fn from(map: HashMap<K, V, S>) -> Self {
        map.0
    }
}

impl<K: Serialize, V: Serialize, S> Serialize for HashMap<K, V, S> {
    fn serialize<Z: Serializer>(&self, serializer: Z) -> Result<Z::Ok, Z::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, K, V, S> Deserialize<'de> for HashMap<K, V, S>
where
    K: Deserialize<'de> + Eq + Hash,
    V: Deserialize<'de>,
    S: BuildHasher + Default,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        std::collections::HashMap::deserialize(deserializer).map(Self)
    }
}

impl<T> HashSet<T, DefaultHashBuilder> {
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty set with room for `capacity` values.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, DefaultHashBuilder::default())
    }
}

impl<T, S> HashSet<T, S> {
    /// An empty set that hashes with `hasher`.
    pub fn with_hasher(hasher: S) -> Self {
        Self(std::collections::HashSet::with_hasher(hasher))
    }

    /// An empty set with room for `capacity` values, hashing with `hasher`.
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self(std::collections::HashSet::with_capacity_and_hasher(
            capacity, hasher,
        ))
    }

    /// Whether the set is empty. Inherent so that a path such as
    /// `HashSet::is_empty` resolves, which `Deref` alone does not give.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of values. Inherent for the same reason as [`Self::is_empty`].
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// The `std` set, for an API that names it.
    pub fn into_inner(self) -> std::collections::HashSet<T, S> {
        self.0
    }
}

impl<T, S> Deref for HashSet<T, S> {
    type Target = std::collections::HashSet<T, S>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> DerefMut for HashSet<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T, S: Default> Default for HashSet<T, S> {
    fn default() -> Self {
        Self(std::collections::HashSet::default())
    }
}

impl<T: Clone, S: Clone> Clone for HashSet<T, S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: fmt::Debug, S> fmt::Debug for HashSet<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Eq + Hash, S: BuildHasher> PartialEq for HashSet<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Eq + Hash, S: BuildHasher> Eq for HashSet<T, S> {}

impl<T: Eq + Hash, S: BuildHasher + Default> FromIterator<T> for HashSet<T, S> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T: Eq + Hash, S: BuildHasher> Extend<T> for HashSet<T, S> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}

impl<'a, T: 'a + Eq + Hash + Copy, S: BuildHasher> Extend<&'a T> for HashSet<T, S> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}

impl<T, S> IntoIterator for HashSet<T, S> {
    type Item = T;
    type IntoIter = hash_set::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T, S> IntoIterator for &'a HashSet<T, S> {
    type Item = &'a T;
    type IntoIter = hash_set::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T: Eq + Hash, const N: usize> From<[T; N]> for HashSet<T, DefaultHashBuilder> {
    fn from(values: [T; N]) -> Self {
        values.into_iter().collect()
    }
}

impl<T, S> From<std::collections::HashSet<T, S>> for HashSet<T, S> {
    fn from(set: std::collections::HashSet<T, S>) -> Self {
        Self(set)
    }
}

impl<T, S> From<HashSet<T, S>> for std::collections::HashSet<T, S> {
    fn from(set: HashSet<T, S>) -> Self {
        set.0
    }
}

impl<T: Serialize, S> Serialize for HashSet<T, S> {
    fn serialize<Z: Serializer>(&self, serializer: Z) -> Result<Z::Ok, Z::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T, S> Deserialize<'de> for HashSet<T, S>
where
    T: Deserialize<'de> + Eq + Hash,
    S: BuildHasher + Default,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        std::collections::HashSet::deserialize(deserializer).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_round_trips_through_serde_as_std_does() {
        let ours: HashMap<String, u32> = HashMap::from([("a".to_owned(), 1)]);
        let std_map = std::collections::HashMap::from([("a".to_owned(), 1_u32)]);
        let json = serde_json::to_string(&ours).unwrap();
        assert_eq!(json, serde_json::to_string(&std_map).unwrap());
        assert_eq!(json, r#"{"a":1}"#);
        assert_eq!(
            serde_json::from_str::<HashMap<String, u32>>(&json).unwrap(),
            ours
        );
    }

    #[test]
    fn a_set_round_trips_through_serde_as_std_does() {
        let ours: HashSet<u32> = HashSet::from([7]);
        let json = serde_json::to_string(&ours).unwrap();
        assert_eq!(json, "[7]");
        assert_eq!(serde_json::from_str::<HashSet<u32>>(&json).unwrap(), ours);
    }

    #[test]
    #[allow(clippy::indexing_slicing)]
    fn the_std_constructors_exist_and_the_std_methods_are_reachable() {
        let mut map: HashMap<&str, u32> = HashMap::new();
        map.insert("k", 1);
        *map.entry("k").or_insert(0) += 1;
        assert_eq!(map["k"], 2);
        assert_eq!(map.get("k"), Some(&2));
        let mut set: HashSet<u32> = HashSet::with_capacity(4);
        set.extend([1, 2]);
        let other: HashSet<u32> = [2, 3].into_iter().collect();
        assert_eq!(
            set.intersection(&other).copied().collect::<Vec<_>>(),
            vec![2]
        );
    }
}
