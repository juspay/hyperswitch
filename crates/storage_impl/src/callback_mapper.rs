use common_utils::errors::CustomResult;
use diesel_models::callback_mapper as storage;
use error_stack::report;
use hyperswitch_domain_models::callback_mapper as domain;
use router_env::{instrument, tracing};
use sample::callback_mapper::CallbackMapperInterface;

use crate::{connection, errors, DataModelExt, DatabaseStore, RouterStore};

impl DataModelExt for domain::CallbackMapper {
    type StorageModel = storage::CallbackMapper;

    fn to_storage_model(self) -> Self::StorageModel {
        storage::CallbackMapper {
            id: self.id,
            type_: self.type_,
            data: self.data,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            id: storage_model.id,
            type_: storage_model.type_,
            data: storage_model.data,
            created_at: storage_model.created_at,
            last_modified_at: storage_model.last_modified_at,
        }
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CallbackMapperInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: domain::CallbackMapper,
    ) -> CustomResult<domain::CallbackMapper, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        call_back_mapper
            .to_storage_model()
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .map(domain::CallbackMapper::from_storage_model)
    }

    #[instrument(skip_all)]
    async fn find_call_back_mapper_by_id(
        &self,
        id: &str,
    ) -> CustomResult<domain::CallbackMapper, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::CallbackMapper::find_by_id(&conn, id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .map(domain::CallbackMapper::from_storage_model)
    }
}
