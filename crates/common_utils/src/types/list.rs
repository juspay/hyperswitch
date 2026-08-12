//! List-query pagination and sorting types.

/// A page size, always clamped to `1..=max_limit`.
#[derive(Clone, Copy, Debug)]
pub struct PageSize(i64);

impl PageSize {
    /// Use the caller's value if given, otherwise `default`; then clamp into `1..=max_limit`.
    pub fn new(requested: Option<u32>, default: u32, max_limit: u32) -> Self {
        let value = requested.unwrap_or(default).clamp(1, max_limit);
        Self(i64::from(value))
    }

    /// Returns the resolved page size as `i64`.
    pub fn as_i64(self) -> i64 {
        self.0
    }
}

/// A page offset, capped so a caller can't skip a huge number of rows.
#[derive(Clone, Copy, Debug, Default)]
pub struct PageOffset(i64);

impl PageOffset {
    const MAX: u32 = 20_000;

    /// Use the caller's value if given, otherwise 0; cap at `MAX`.
    pub fn new(requested: Option<u32>) -> Self {
        Self(i64::from(requested.unwrap_or(0).min(Self::MAX)))
    }

    /// Returns the resolved offset as `i64`.
    pub fn as_i64(self) -> i64 {
        self.0
    }
}

/// Sort direction. The column is chosen by the caller, not hardcoded here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDirection {
    /// Descending — newest first.
    #[default]
    Desc,
    /// Ascending — oldest first.
    Asc,
}
