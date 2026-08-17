//! List-query pagination and sorting types.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::consts::{LIST_DEFAULT_LIMIT, LIST_MAX_LIMIT};

/// A page size, validated against the global list bounds (`1..=LIST_MAX_LIMIT`).
/// Deserialization rejects an out-of-range limit, so the caller gets an error rather
/// than silently receiving fewer (or zero) rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageSize(u64);

impl PageSize {
    /// Caller's value if given, else `default`; clamped into `1..=max_limit`.
    pub fn new(requested: Option<u32>, default: u32, max_limit: u32) -> Self {
        let value = requested.unwrap_or(default).clamp(1, max_limit);
        Self(u64::from(value))
    }

    /// The resolved limit as `i64`, ready for Diesel's `.limit()`.
    pub fn as_i64(self) -> i64 {
        i64::try_from(self.0).unwrap_or(i64::MAX)
    }
}

impl Default for PageSize {
    fn default() -> Self {
        Self(u64::from(LIST_DEFAULT_LIMIT))
    }
}

impl Serialize for PageSize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for PageSize {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Option::<u32>::deserialize(deserializer)? {
            None => Ok(Self::default()),
            Some(value) if value == 0 || value > LIST_MAX_LIMIT => Err(serde::de::Error::custom(
                format!("list limit {value} is invalid, it must be between 1 and {LIST_MAX_LIMIT}"),
            )),
            Some(value) => Ok(Self(u64::from(value))),
        }
    }
}

/// A page offset, capped so a caller can't skip a huge number of rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PageOffset(u64);

impl PageOffset {
    const MAX: u32 = 20_000;

    /// Caller's value if given, else 0; capped at `MAX`.
    pub fn new(requested: Option<u32>) -> Self {
        Self(u64::from(requested.unwrap_or(0).min(Self::MAX)))
    }

    /// The capped offset as `i64`, ready for Diesel's `.offset()`.
    pub fn as_i64(self) -> i64 {
        i64::try_from(self.0).unwrap_or(i64::MAX)
    }
}

impl Serialize for PageOffset {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for PageOffset {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new(Option::<u32>::deserialize(deserializer)?))
    }
}

/// Sort direction. The column is chosen by the caller, not hardcoded here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDirection {
    #[default]
    /// Newest-first (descending).
    Desc,
    /// Oldest-first (ascending).
    Asc,
}
