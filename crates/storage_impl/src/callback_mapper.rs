use diesel_models::callback_mapper::CallbackMapper as DieselCallbackMapper;
use hyperswitch_domain_models::callback_mapper::{CallbackMapper,CallbackMapperInterface};
use router_env::{instrument, tracing};
use error_stack::report;
use crate::{errors, connection, MockDb, CustomResult, DatabaseStore, RouterStore, kv_router_store::KVRouterStore};

use crate::DataModelExt;

impl DataModelExt for CallbackMapper {
    type StorageModel = DieselCallbackMapper;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselCallbackMapper {
            id: self.id,
            type_: self.callback_mapper_id_type,
            data: self.data,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            id: storage_model.id,
            callback_mapper_id_type: storage_model.type_,
            data: storage_model.data,
            created_at: storage_model.created_at,
            last_modified_at: storage_model.last_modified_at,
        }
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CallbackMapperInterface for KVRouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: CallbackMapper,
    ) -> CustomResult<CallbackMapper, errors::StorageError> {
        self.router_store
            .insert_call_back_mapper(call_back_mapper)
            .await
    }

    #[instrument(skip_all)]
    async fn find_call_back_mapper_by_id(
        &self,
        id: &str,
    ) -> CustomResult<CallbackMapper, errors::StorageError> {
        self.router_store
            .find_call_back_mapper_by_id(id)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CallbackMapperInterface for RouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: CallbackMapper,
    ) -> CustomResult<CallbackMapper, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        call_back_mapper
            .to_storage_model()
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .map(CallbackMapper::from_storage_model)
    }

    #[instrument(skip_all)]
    async fn find_call_back_mapper_by_id(
        &self,
        id: &str,
    ) -> CustomResult<CallbackMapper, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        DieselCallbackMapper::find_by_id(&conn, id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .map(CallbackMapper::from_storage_model)
    }
}

#[async_trait::async_trait]
impl CallbackMapperInterface for MockDb {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_call_back_mapper(
        &self,
        _call_back_mapper: CallbackMapper,
    ) -> CustomResult<CallbackMapper, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn find_call_back_mapper_by_id(
        &self,
        _id: &str,
    ) -> CustomResult<CallbackMapper, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}