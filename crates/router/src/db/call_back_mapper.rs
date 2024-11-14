use error_stack::report;
// use diesel_models::call_back_mapper;
use router_env::{instrument, tracing};

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
        call_back_mapper: storage::CallBackMapperNew,
    ) -> CustomResult<storage::CallBackMapper, errors::StorageError>;

    async fn find_call_back_mapper_by_id(
        &self,
        id: String,
    ) -> CustomResult<storage::CallBackMapper, errors::StorageError>;
}

#[async_trait::async_trait]
impl CallBackMapperInterface for Store {
    #[instrument(skip_all)]
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: storage::CallBackMapperNew,
    ) -> CustomResult<storage::CallBackMapper, errors::StorageError>{
        let conn = connection::pg_connection_write(self).await?;
        call_back_mapper
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_call_back_mapper_by_id(
        &self,
        id: String,
    ) -> CustomResult<storage::CallBackMapper, errors::StorageError>{
        let conn = connection::pg_connection_read(self).await?;
        storage::CallBackMapper::find_by_id(
            &conn,
            id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))

    }

}