use common_utils::errors::CustomResult;
use data_models::errors;
use diesel_models::reverse_lookup::{
    ReverseLookup as DieselReverseLookup, ReverseLookupNew as DieselReverseLookupNew,
};
use error_stack::{IntoReport, ResultExt};

use crate::{redis::cache::get_or_populate_redis, DatabaseStore, KVRouterStore, RouterStore};

#[async_trait::async_trait]
pub trait ReverseLookupInterface {
    async fn insert_reverse_lookup(
        &self,
        _new: DieselReverseLookupNew,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError>;
    async fn get_lookup_by_lookup_id(
        &self,
        _id: &str,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError>;
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ReverseLookupInterface for RouterStore<T> {
    async fn insert_reverse_lookup(
        &self,
        new: DieselReverseLookupNew,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        let conn = self
            .get_master_pool()
            .get()
            .await
            .into_report()
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        new.insert(&conn).await.map_err(|er| {
            let new_err = crate::diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
    }

    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        let database_call = || async {
            let conn = crate::utils::pg_connection_read(self).await?;
            DieselReverseLookup::find_by_lookup_id(id, &conn)
                .await
                .map_err(|er| {
                    let new_err = crate::diesel_error_to_data_error(er.current_context());
                    er.change_context(new_err)
                })
        };
        get_or_populate_redis(self, id, database_call).await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ReverseLookupInterface for KVRouterStore<T> {
    async fn insert_reverse_lookup(
        &self,
        new: DieselReverseLookupNew,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        self.router_store.insert_reverse_lookup(new).await
    }

    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        self.router_store.get_lookup_by_lookup_id(id).await
    }
}
