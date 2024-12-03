use error_stack::report;
use hyperswitch_domain_models::callback_mapper::{self as domain};
use router_env::{instrument, tracing};
use storage_impl::{DataModelExt, MockDb};

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait CallBackMapperInterface {
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: domain::CallBackMapperNew,
    ) -> CustomResult<domain::CallBackMapper, errors::StorageError>;

    async fn find_call_back_mapper_by_id(
        &self,
        id: &str,
    ) -> CustomResult<domain::CallBackMapper, errors::StorageError>;
}

#[async_trait::async_trait]
impl CallBackMapperInterface for Store {
    #[instrument(skip_all)]
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: domain::CallBackMapperNew,
    ) -> CustomResult<domain::CallBackMapper, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let storage_model = call_back_mapper
            .to_storage_model()
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        Ok(domain::CallBackMapper::from_storage_model(storage_model))
    }

    #[instrument(skip_all)]
    async fn find_call_back_mapper_by_id(
        &self,
        id: &str,
    ) -> CustomResult<domain::CallBackMapper, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        Ok(domain::CallBackMapper::from_storage_model(
            storage::CallBackMapper::find_by_id(&conn, id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?,
        ))
    }
}

#[async_trait::async_trait]
impl CallBackMapperInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_call_back_mapper(
        &self,
        _call_back_mapper: domain::CallBackMapperNew,
    ) -> CustomResult<domain::CallBackMapper, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[instrument(skip_all)]
    async fn find_call_back_mapper_by_id(
        &self,
        _id: &str,
    ) -> CustomResult<domain::CallBackMapper, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
