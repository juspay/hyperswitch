use error_stack::report;
use hyperswitch_domain_models::callback_mapper as domain;
use router_env::{instrument, tracing};
use storage_impl::DataModelExt;

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait CallbackMapperInterface {
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: domain::CallbackMapper,
    ) -> CustomResult<domain::CallbackMapper, errors::StorageError>;

    async fn find_call_back_mapper_by_id(
        &self,
        id: &str,
    ) -> CustomResult<domain::CallbackMapper, errors::StorageError>;
}

#[async_trait::async_trait]
impl CallbackMapperInterface for Store {
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
