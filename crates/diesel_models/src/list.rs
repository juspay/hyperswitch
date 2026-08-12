//! Shared list-query helpers: `apply_pagination`, `into_boxed_list`, and the
//! `boxed_list_query!` macro.

pub use common_utils::types::list::{PageOffset, PageSize, SortDirection};

use diesel::query_dsl::methods::{BoxedDsl, LimitDsl, OffsetDsl};

/// Apply page size and offset to any boxed list query.
pub fn apply_pagination<Q>(query: Q, page_size: PageSize, offset: PageOffset) -> Q
where
    Q: LimitDsl<Output = Q> + OffsetDsl<Output = Q>,
{
    query.limit(page_size.as_i64()).offset(offset.as_i64())
}

/// The **only** place in the workspace that calls `.into_boxed()`.
///
/// All list queries must go through `boxed_list_query!`, which calls this fn.
/// The `#[allow]` suppresses the workspace-wide `disallowed-methods` lint so
/// that callers of the macro don't need their own `#[allow]`.
#[allow(clippy::disallowed_methods)]
pub fn into_boxed_list<'a, Q>(query: Q) -> <Q as BoxedDsl<'a, diesel::pg::Pg>>::Output
where
    Q: BoxedDsl<'a, diesel::pg::Pg>,
{
    query.internal_into_boxed()
}

/// Build the base of a list query: filter by merchant scope, apply the
/// caller-chosen order expression, and box the query.
///
/// The `order` expression should include a tiebreak column (the table's
/// primary key) for stable pagination, e.g. `(dsl::created_at.desc(), dsl::refund_id.desc())`.
#[macro_export]
macro_rules! boxed_list_query {
    (
        $table:path,
        scope = $scope:expr,
        order = $order:expr
    ) => {{
        $crate::list::into_boxed_list(
            <$table as diesel::associations::HasTable>::table()
                .filter($scope)
                .order($order)
        )
    }};
}
