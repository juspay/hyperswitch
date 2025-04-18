use common_utils::errors::CustomResult;
use common_utils::id_type::GlobalTokenId;
use diesel_models as storage;
use error_stack::ResultExt;
use router_env::logger;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, utils::StorageErrorExt},
    types::domain,
};

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[async_trait::async_trait]
pub trait TokenizationInterface {
    async fn insert_tokenization(
        &self,
        tokenization: domain::TokenizationNew,
    ) -> CustomResult<domain::Tokenization, errors::StorageError>;

    async fn find_tokenization_by_id(
        &self,
        id: GlobalTokenId,
    ) -> CustomResult<domain::Tokenization, errors::StorageError>;

    async fn find_tokenization_by_locker_id(
        &self,
        locker_id: &str,
    ) -> CustomResult<domain::Tokenization, errors::StorageError>;
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[async_trait::async_trait]
impl TokenizationInterface for Store {
    async fn insert_tokenization(
        &self,
        tokenization: domain::TokenizationNew,
    ) -> CustomResult<domain::Tokenization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::TokenizationNew::from(tokenization)
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|t| async move { t.try_into() })
            .await
    }

    async fn find_tokenization_by_id(
        &self,
        id: GlobalTokenId,
    ) -> CustomResult<domain::Tokenization, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Tokenization::find_by_id(&conn, id)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|t| async move { t.try_into() })
            .await
    }

    async fn find_tokenization_by_locker_id(
        &self,
        locker_id: &str,
    ) -> CustomResult<domain::Tokenization, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Tokenization::find_by_locker_id(&conn, locker_id)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|t| async move { t.try_into() })
            .await
    }
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[async_trait::async_trait]
impl TokenizationInterface for MockDb {
    async fn insert_tokenization(
        &self,
        _tokenization: domain::TokenizationNew,
    ) -> CustomResult<domain::Tokenization, errors::StorageError> {
        logger::error!("Mock DB does not support tokenization operations");
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_tokenization_by_id(
        &self,
        _id: GlobalTokenId,
    ) -> CustomResult<domain::Tokenization, errors::StorageError> {
        logger::error!("Mock DB does not support tokenization operations");
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_tokenization_by_locker_id(
        &self,
        _locker_id: &str,
    ) -> CustomResult<domain::Tokenization, errors::StorageError> {
        logger::error!("Mock DB does not support tokenization operations");
        Err(errors::StorageError::MockDbError)?
    }
}

impl From<domain::TokenizationNew> for storage::TokenizationNew {
    fn from(value: domain::TokenizationNew) -> Self {
        Self {
            merchant_id: value.merchant_id,
            locker_id: value.locker_id,
            status: value.status.into(),
            version: value.version.into(),
        }
    }
}

impl TryFrom<storage::Tokenization> for domain::Tokenization {
    type Error = error_stack::Report<errors::StorageError>;

    fn try_from(value: storage::Tokenization) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            merchant_id: value.merchant_id,
            locker_id: value.locker_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            status: value.status.into(),
            version: value.version.into(),
        })
    }
} 