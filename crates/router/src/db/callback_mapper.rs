use error_stack::report;
use router_env::{instrument, tracing};
use storage_impl::DataModelExt;

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};
use hyperswitch_domain_models::callback_mapper::{self as DomainCallBackMapper};

#[async_trait::async_trait]
pub trait CallBackMapperInterface {
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: DomainCallBackMapper::CallBackMapperNew,
    ) -> CustomResult<DomainCallBackMapper::CallBackMapper, errors::StorageError>;

    async fn find_call_back_mapper_by_id(
        &self,
        id: String,
    ) -> CustomResult<DomainCallBackMapper::CallBackMapper, errors::StorageError>;
}

#[async_trait::async_trait]
impl CallBackMapperInterface for Store {
    #[instrument(skip_all)]
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: DomainCallBackMapper::CallBackMapperNew, //take domain model as input
    ) -> CustomResult<DomainCallBackMapper::CallBackMapper, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let storage_model = call_back_mapper
            .to_storage_model()
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        // Convert the storage model back to domain model
        Ok(DomainCallBackMapper::CallBackMapper::from_storage_model(storage_model))
    }

    async fn find_call_back_mapper_by_id(
        &self,
        id: String,
    ) -> CustomResult<DomainCallBackMapper::CallBackMapper, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        Ok(DomainCallBackMapper::CallBackMapper::from_storage_model(storage::CallBackMapper::find_by_id(&conn, id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?))
            
    }
}
