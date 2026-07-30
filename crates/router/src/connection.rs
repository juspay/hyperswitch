use common_utils::request_context::RequestContext;
use error_stack::ResultExt;
use storage_impl::{errors as storage_errors, DatabaseStore};

use crate::errors;

pub type PgPooledConn = diesel_models::PgPooledConn;

pub async fn pg_connection_read<T: DatabaseStore + RequestContext>(
    store: &T,
) -> errors::CustomResult<
    storage_impl::database::store::DatabaseConnectionWithContext,
    storage_errors::StorageError,
> {
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

    let connection = pool
        .pg_pool
        .get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;

    Ok(
        storage_impl::database::store::DatabaseConnectionWithContext::new(
            connection,
            store.request_id().map(str::to_owned),
            pool.event_emitter.clone(),
        ),
    )
}

pub async fn pg_accounts_connection_read<T: DatabaseStore + RequestContext>(
    store: &T,
) -> errors::CustomResult<
    storage_impl::database::store::DatabaseConnectionWithContext,
    storage_errors::StorageError,
> {
    // If only OLAP is enabled get replica pool.
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let pool = store.get_accounts_replica_pool();

    // If either one of these are true we need to get master pool.
    //  1. Only OLTP is enabled.
    //  2. Both OLAP and OLTP is enabled.
    //  3. Both OLAP and OLTP is disabled.
    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let pool = store.get_accounts_master_pool();

    let connection = pool
        .pg_pool
        .get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;

    Ok(
        storage_impl::database::store::DatabaseConnectionWithContext::new(
            connection,
            store.request_id().map(str::to_owned),
            pool.event_emitter.clone(),
        ),
    )
}

pub async fn pg_connection_write<T: DatabaseStore + RequestContext>(
    store: &T,
) -> errors::CustomResult<
    storage_impl::database::store::DatabaseConnectionWithContext,
    storage_errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();

    let connection = pool
        .pg_pool
        .get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;

    Ok(
        storage_impl::database::store::DatabaseConnectionWithContext::new(
            connection,
            store.request_id().map(str::to_owned),
            pool.event_emitter.clone(),
        ),
    )
}

pub async fn pg_accounts_connection_write<T: DatabaseStore + RequestContext>(
    store: &T,
) -> errors::CustomResult<
    storage_impl::database::store::DatabaseConnectionWithContext,
    storage_errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_accounts_master_pool();

    let connection = pool
        .pg_pool
        .get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;

    Ok(
        storage_impl::database::store::DatabaseConnectionWithContext::new(
            connection,
            store.request_id().map(str::to_owned),
            pool.event_emitter.clone(),
        ),
    )
}
