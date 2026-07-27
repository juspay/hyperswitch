use common_utils::errors;

use crate::{database::store::DatabaseConnectionWithContext, DatabaseStoreWithContext};

pub async fn pg_connection_read<T: DatabaseStoreWithContext>(
    store: &T,
) -> errors::CustomResult<DatabaseConnectionWithContext, crate::errors::StorageError> {
    // If only OLAP is enabled get replica pool.
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    return store.get_read_connection().await;

    // If either one of these are true we need to get master pool.
    //  1. Only OLTP is enabled.
    //  2. Both OLAP and OLTP is enabled.
    //  3. Both OLAP and OLTP is disabled.
    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    store.get_write_connection().await
}

pub async fn pg_connection_write<T: DatabaseStoreWithContext>(
    store: &T,
) -> errors::CustomResult<DatabaseConnectionWithContext, crate::errors::StorageError> {
    // Since all writes should happen to master DB only choose master DB.
    store.get_write_connection().await
}
