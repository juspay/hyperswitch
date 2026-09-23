//! List-query pagination and sorting types.

use serde::{Deserialize, Serialize, Serializer};

use crate::consts::{LIST_DEFAULT_LIMIT, LIST_MAX_LIMIT, LIST_MIN_LIMIT};

/// A page size, validated against the global list bounds (`1..=LIST_MAX_LIMIT`).
/// Deserialization rejects an out-of-range limit, so the caller gets an error rather
/// than silently receiving fewer (or zero) rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "u64")]
pub struct PageSize(u32);

/// Error returned by [`PageSize::new`] / [`PageOffset::new`] when an out-of-range
/// value is supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PaginationError {
    /// `limit` was zero or greater than the allowed maximum.
    #[error("list limit {got} is invalid, it must be between 1 and {max}")]
    InvalidLimit {
        /// The value the caller passed.
        got: u32,
        /// The maximum allowed value.
        max: u32,
    },
    /// `offset` was greater than the allowed maximum.
    #[error("list offset {got} is invalid, it must be at most {max}")]
    InvalidOffset {
        /// The value the caller passed.
        got: u32,
        /// The maximum allowed value.
        max: u32,
    },
}

impl PageSize {
    /// Construct a validated page size, rejecting out-of-range values.
    pub fn new(value: u32) -> Result<Self, PaginationError> {
        if !(LIST_MIN_LIMIT..=LIST_MAX_LIMIT).contains(&value) {
            return Err(PaginationError::InvalidLimit {
                got: value,
                max: LIST_MAX_LIMIT,
            });
        }
        Ok(Self(value))
    }

    /// The resolved limit as `i64`, ready for Diesel's `.limit()`.
    pub fn as_i64(self) -> i64 {
        i64::from(self.0)
    }

    /// The resolved limit as `usize` (for in-memory slicing).
    pub fn as_usize(self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX)
    }

    /// The raw limit as `u32`.
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl Default for PageSize {
    fn default() -> Self {
        Self(LIST_DEFAULT_LIMIT)
    }
}

impl Serialize for PageSize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }
}

// Internal compat conversion: clamps rather than fails. Used by stripe compatibility
// and other non-HTTP paths that hold a resolved `u32`.
impl From<u32> for PageSize {
    fn from(value: u32) -> Self {
        Self(value.clamp(LIST_MIN_LIMIT, LIST_MAX_LIMIT))
    }
}

// Validation hook for `#[serde(try_from = "u64")]`. Rejects out-of-range values from
// the request path; the `u64` intermediary avoids a blanket-impl collision between
// `From<u32>` and `TryFrom<u32>`.
impl TryFrom<u64> for PageSize {
    type Error = PaginationError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let value = u32::try_from(value).map_err(|_| PaginationError::InvalidLimit {
            got: u32::MAX,
            max: LIST_MAX_LIMIT,
        })?;
        Self::new(value)
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "u64")]
pub struct PageOffset(u32);

impl PageOffset {
    const MAX: u32 = 20_000;

    /// Construct a validated page offset, rejecting values above [`Self::MAX`].
    pub fn new(value: u32) -> Result<Self, PaginationError> {
        if value > Self::MAX {
            return Err(PaginationError::InvalidOffset {
                got: value,
                max: Self::MAX,
            });
        }
        Ok(Self(value))
    }

    /// The capped offset as `i64`, ready for Diesel's `.offset()`.
    pub fn as_i64(self) -> i64 {
        i64::from(self.0)
    }

    /// The capped offset as `usize` (for in-memory skipping).
    pub fn as_usize(self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX)
    }
}

impl Serialize for PageOffset {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }
}

// Internal compat conversion: clamps rather than fails.
impl From<u32> for PageOffset {
    fn from(value: u32) -> Self {
        Self(value.min(Self::MAX))
    }
}

// Validation hook for `#[serde(try_from = "u64")]`. Rejects values above MAX from
// the request path; the `u64` intermediary avoids a blanket-impl collision between
// `From<u32>` and `TryFrom<u32>`.
impl TryFrom<u64> for PageOffset {
    type Error = PaginationError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let value = u32::try_from(value).map_err(|_| PaginationError::InvalidOffset {
            got: u32::MAX,
            max: Self::MAX,
        })?;
        Self::new(value)
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
