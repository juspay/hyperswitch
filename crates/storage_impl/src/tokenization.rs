use error_stack::ResultExt;
use router_env::logger;

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[async_trait::async_trait]
pub trait TokenizationInterface {
    async fn insert_tokenization(
        &self,
        tokenization: storage::TokenizationNew,
    ) -> CustomResult<storage::Tokenization, errors::StorageError>;

    async fn find_tokenization_by_token(
        &self,
        token: &str,
    ) -> CustomResult<storage::Tokenization, errors::StorageError>;

    async fn update_tokenization(
        &self,
        token: &str,
        tokenization: storage::TokenizationUpdate,
    ) -> CustomResult<storage::Tokenization, errors::StorageError>;
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[async_trait::async_trait]
impl TokenizationInterface for Store {
    async fn insert_tokenization(
        &self,
        tokenization: storage::TokenizationNew,
    ) -> CustomResult<storage::Tokenization, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        tokenization
            .insert(&conn)
            .await
            .change_context(errors::StorageError::DatabaseError)
            .attach_printable("Error inserting tokenization")
    }

    async fn find_tokenization_by_token(
        &self,
        token: &str,
    ) -> CustomResult<storage::Tokenization, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::Tokenization::find_by_token(&conn, token)
            .await
            .change_context(errors::StorageError::DatabaseError)
            .attach_printable(format!("Error finding tokenization for token: {}", token))
    }

    async fn update_tokenization(
        &self,
        token: &str,
        tokenization: storage::TokenizationUpdate,
    ) -> CustomResult<storage::Tokenization, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::Tokenization::update_by_token(&conn, token, tokenization)
            .await
            .change_context(errors::StorageError::DatabaseError)
            .attach_printable(format!("Error updating tokenization for token: {}", token))
    }
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[async_trait::async_trait]
impl TokenizationInterface for MockDb {
    async fn insert_tokenization(
        &self,
        _tokenization: storage::TokenizationNew,
    ) -> CustomResult<storage::Tokenization, errors::StorageError> {
        logger::error!("Mock DB does not support tokenization operations");
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_tokenization_by_token(
        &self,
        _token: &str,
    ) -> CustomResult<storage::Tokenization, errors::StorageError> {
        logger::error!("Mock DB does not support tokenization operations");
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_tokenization(
        &self,
        _token: &str,
        _tokenization: storage::TokenizationUpdate,
    ) -> CustomResult<storage::Tokenization, errors::StorageError> {
        logger::error!("Mock DB does not support tokenization operations");
        Err(errors::StorageError::MockDbError)?
    }
} 