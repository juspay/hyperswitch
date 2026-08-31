use common_utils::errors;
use error_stack::ResultExt;

use crate::{
    database::{pool_metrics, store::DatabaseConnectionWithContext},
    errors::StorageError,
    metrics, DatabaseStore, RequestContext,
};

pub async fn pg_connection_read<'a, T: DatabaseStore + RequestContext>(
    store: &'a T,
) -> errors::CustomResult<DatabaseConnectionWithContext<'a>, StorageError> {
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

    let db_pool_label = {
        #[cfg(all(feature = "olap", not(feature = "oltp")))]
        {
            pool_metrics::DbPool::Replica
        }
        #[cfg(any(
            all(not(feature = "olap"), feature = "oltp"),
            all(feature = "olap", feature = "oltp"),
            all(not(feature = "olap"), not(feature = "oltp"))
        ))]
        {
            pool_metrics::DbPool::Master
        }
    };
    let tenant_id = pool.tenant_id.get_string_repr();

    #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
    let mut connection = metrics::record_db_connection_acquire_duration(
        pool.pg_pool.get(),
        db_pool_label,
        tenant_id,
    )
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

pub async fn pg_connection_write<'a, T: DatabaseStore + RequestContext>(
    store: &'a T,
) -> errors::CustomResult<DatabaseConnectionWithContext<'a>, StorageError> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();
    let tenant_id = pool.tenant_id.get_string_repr();

    #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
    let mut connection = metrics::record_db_connection_acquire_duration(
        pool.pg_pool.get(),
        pool_metrics::DbPool::Master,
        tenant_id,
    )
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
