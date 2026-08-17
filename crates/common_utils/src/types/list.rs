//! List-query pagination and sorting types.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::consts::{LIST_DEFAULT_LIMIT, LIST_MAX_LIMIT, LIST_MIN_LIMIT};

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

    /// The resolved limit as `usize` (for in-memory slicing). The value is bounded to
    /// `LIST_MAX_LIMIT`, so the `try_from` never fails in practice.
    pub fn as_usize(self) -> usize {
        usize::try_from(self.0).unwrap_or_default()
    }

    /// The raw limit as `u64`.
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for PageSize {
    fn default() -> Self {
        Self(u64::from(LIST_DEFAULT_LIMIT))
    }
}

impl From<u32> for PageSize {
    /// Clamp a resolved `u32` into the global bounds. For internal callers (e.g.
    /// compatibility-layer conversions) that already hold a value; the request path
    /// uses `Deserialize`, which *rejects* out-of-range values instead of clamping.
    fn from(value: u32) -> Self {
        Self(u64::from(value.clamp(LIST_MIN_LIMIT, LIST_MAX_LIMIT)))
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
            Some(value) if !(LIST_MIN_LIMIT..=LIST_MAX_LIMIT).contains(&value) => {
                Err(serde::de::Error::custom(format!(
                    "list limit {value} is invalid, it must be between {LIST_MIN_LIMIT} and {LIST_MAX_LIMIT}"
                )))
            }
            Some(value) => Ok(Self(u64::from(value))),
        }
    }
}

// Manual `ToSchema`: the derive would emit `array` for a newtype tuple struct, and we
// need a custom component key + integer bounds matching `Deserialize` validation.
impl<'a> utoipa::ToSchema<'a> for PageSize {
    fn schema() -> (
        &'a str,
        utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
    ) {
        use utoipa::openapi::{KnownFormat, ObjectBuilder, SchemaFormat, SchemaType};
        (
            // Use the full dotted path so the component key matches the `$ref` that
            // utoipa's derive macro generates for fields of this type.
            "common_utils.types.list.PageSize",
            ObjectBuilder::new()
                .schema_type(SchemaType::Integer)
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
                .minimum(Some(f64::from(LIST_MIN_LIMIT)))
                .maximum(Some(f64::from(LIST_MAX_LIMIT)))
                .into(),
        )
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

    /// The capped offset as `usize` (for in-memory skipping). Bounded to `MAX`, so the
    /// `try_from` never fails in practice.
    pub fn as_usize(self) -> usize {
        usize::try_from(self.0).unwrap_or_default()
    }
}

impl From<u32> for PageOffset {
    /// Clamp a resolved `u32` into the offset bound. For internal callers that already
    /// hold a value; the request path uses `Deserialize`, which rejects over-`MAX` values.
    fn from(value: u32) -> Self {
        Self(u64::from(value.min(Self::MAX)))
    }
}

impl Serialize for PageOffset {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for PageOffset {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Option::<u32>::deserialize(deserializer)? {
            None => Ok(Self(0)),
            Some(value) if value > Self::MAX => Err(serde::de::Error::custom(format!(
                "list offset {value} is invalid, it must be at most {}",
                Self::MAX
            ))),
            Some(value) => Ok(Self(u64::from(value))),
        }
    }
}

// Manual `ToSchema` for the same reasons as `PageSize` above.
impl<'a> utoipa::ToSchema<'a> for PageOffset {
    fn schema() -> (
        &'a str,
        utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
    ) {
        use utoipa::openapi::{KnownFormat, ObjectBuilder, SchemaFormat, SchemaType};
        (
            // Use the full dotted path so the component key matches the `$ref` that
            // utoipa's derive macro generates for fields of this type.
            "common_utils.types.list.PageOffset",
            ObjectBuilder::new()
                .schema_type(SchemaType::Integer)
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
                .minimum(Some(0.0))
                .maximum(Some(f64::from(Self::MAX)))
                .into(),
        )
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
