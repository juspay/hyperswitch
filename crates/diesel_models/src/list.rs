//! Shared list-query helpers: pagination, boxing, and the `boxed_list_query!` macro.

pub use common_utils::types::list::{PageOffset, PageSize, SortDirection};
use diesel::query_dsl::methods::{BoxedDsl, LimitDsl, OffsetDsl};

/// Apply page size and offset to a boxed list query.
pub fn apply_pagination<Q>(query: Q, page_size: PageSize, offset: PageOffset) -> Q
where
    Q: LimitDsl<Output = Q> + OffsetDsl<Output = Q>,
{
    query.limit(page_size.as_i64()).offset(offset.as_i64())
}

/// The only sanctioned `.into_boxed()` call; the `#[allow]` here means
/// `boxed_list_query!` callers don't need their own.
#[allow(clippy::disallowed_methods)]
pub fn into_boxed_list<'a, Q>(query: Q) -> <Q as BoxedDsl<'a, diesel::pg::Pg>>::Output
where
    Q: BoxedDsl<'a, diesel::pg::Pg>,
{
    query.internal_into_boxed()
}

/// Base of a list query: merchant `scope` + caller-chosen `order`, boxed.
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
                .order($order),
        )
    }};
}
