use common_utils::errors;
use error_stack::ResultExt;

use crate::{
    database::store::DatabaseConnectionWithContext, errors::StorageError, DatabaseStore,
    RequestContext,
};

pub async fn pg_connection_read<T: DatabaseStore + RequestContext>(
    store: &T,
) -> errors::CustomResult<DatabaseConnectionWithContext, StorageError> {
    // If only OLAP is enabled get replica pool.
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let pool = store.get_replica_pool();

    // If either one of these are true we need to get master pool.
    //  1. Only OLTP is enabled.
    //  2. Both OLAP and OLTP is enabled.
    //  3. Both OLAP and OLTP is disabled.
    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let pool = store.get_master_pool();

    #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
    let mut connection = pool
        .pg_pool
        .get_owned()
        .await
        .change_context(StorageError::DatabaseConnectionError)?;

    #[cfg(feature = "deja")]
    crate::utils::deja_route_replay_schema(&mut connection, store).await;

    Ok(DatabaseConnectionWithContext::new(
        connection,
        store.request_id().map(str::to_owned),
        pool.event_emitter.clone(),
    ))
}

pub async fn pg_connection_write<T: DatabaseStore + RequestContext>(
    store: &T,
) -> errors::CustomResult<DatabaseConnectionWithContext, StorageError> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();

    #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
    let mut connection = pool
        .pg_pool
        .get_owned()
        .await
        .change_context(StorageError::DatabaseConnectionError)?;

    #[cfg(feature = "deja")]
    crate::utils::deja_route_replay_schema(&mut connection, store).await;

    Ok(DatabaseConnectionWithContext::new(
        connection,
        store.request_id().map(str::to_owned),
        pool.event_emitter.clone(),
    ))
}
