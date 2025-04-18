use error_stack::{ResultExt, Report};
use common_utils::ext_traits::OptionExt;
use common_utils::id_type::{CellId, GlobalTokenId, MerchantId};
use common_utils::errors::CustomResult;
use diesel::{Insertable, RunQueryDsl};
use tokio::time;
use async_bb8_diesel::AsyncRunQueryDsl;

use super::MockDb;
use crate::{
    connection,
    errors::StorageError,
    database::store::Store,
};

use diesel_models::{
    enums::TokenizationFlag as DbTokenizationFlag,
    tokenization::{Tokenization, TokenizationNew},
    schema_v2::tokenization::dsl as tokenization_dsl,
    PgPooledConn,
};
use hyperswitch_domain_models::tokenization as domain_tokenization;

// New type wrapper to avoid orphan rule
#[derive(Debug, Clone)]
pub struct TokenizationWrapper(pub Tokenization);

#[async_trait::async_trait]
pub trait TokenizationInterface {
    async fn insert_tokenization(
        &self,
        tokenization: domain_tokenization::TokenizationNew,
    ) -> CustomResult<domain_tokenization::Tokenization, StorageError>;
}

#[async_trait::async_trait]
impl TokenizationInterface for Store {
    async fn insert_tokenization(
        &self,
        tokenization: domain_tokenization::TokenizationNew,
    ) -> CustomResult<domain_tokenization::Tokenization, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let tokenization_db = TokenizationNew {
            id: tokenization.id,
            merchant_id: tokenization.merchant_id,
            locker_id: tokenization.locker_id,
            created_at: common_utils::date_time::now(),
            updated_at: common_utils::date_time::now(),
            flag: tokenization.flag.to_db_flag(),
            version: tokenization.version,
        };
        
        // First insert and get the ID
        let inserted_row = diesel::insert_into(tokenization_dsl::tokenization)
            .values(tokenization_db)
            .returning(tokenization_dsl::id)
            .get_result_async::<uuid::Uuid>(&conn)
            .await
            .map_err(|error| StorageError::from(error))?;
            
        // Then fetch the full record
        tokenization_dsl::tokenization
            .filter(tokenization_dsl::id.eq(inserted_row))
            .first_async::<Tokenization>(&conn)
            .await
            .map_err(|error| StorageError::from(error))
            .map(TokenizationWrapper)
            .map(|wrapper| wrapper.try_into())
            .transpose()?
            .ok_or(StorageError::ValueNotFound("Tokenization not found after insert".into()).into())
    }
}

#[async_trait::async_trait]
impl TokenizationInterface for MockDb {
    async fn insert_tokenization(
        &self,
        tokenization: domain_tokenization::TokenizationNew,
    ) -> CustomResult<domain_tokenization::Tokenization, StorageError> {
        let mut tokenizations = self.tokenizations.lock().await;
        let tokenization = Tokenization {
            id: GlobalTokenId::generate(&state),
            merchant_id: tokenization.merchant_id,
            locker_id: tokenization.locker_id,
            created_at: common_utils::date_time::now(),
            updated_at: common_utils::date_time::now(),
            flag: tokenization.flag.to_db_flag(),
            version: tokenization.version,
        };
        tokenizations.push(tokenization.clone());
        Ok(TokenizationWrapper(tokenization).try_into()?)
    }
}

// Define a new trait for conversion between TokenizationFlag types
pub trait TokenizationFlagConversion {
    fn to_db_flag(&self) -> DbTokenizationFlag;
    fn from_db_flag(flag: DbTokenizationFlag) -> Self;
}

impl TokenizationFlagConversion for common_utils::tokenization::TokenizationFlag {
    fn to_db_flag(&self) -> DbTokenizationFlag {
        match self {
            common_utils::tokenization::TokenizationFlag::Enabled => DbTokenizationFlag::Enabled,
            common_utils::tokenization::TokenizationFlag::Disabled => DbTokenizationFlag::Disabled,
        }
    }

    fn from_db_flag(flag: DbTokenizationFlag) -> Self {
        match flag {
            DbTokenizationFlag::Enabled => common_utils::tokenization::TokenizationFlag::Enabled,
            DbTokenizationFlag::Disabled => common_utils::tokenization::TokenizationFlag::Disabled,
        }
    }
}

impl TryFrom<TokenizationWrapper> for domain_tokenization::Tokenization {
    type Error = StorageError;

    fn try_from(value: TokenizationWrapper) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.0.id,
            merchant_id: value.0.merchant_id,
            locker_id: value.0.locker_id,
            created_at: value.0.created_at,
            updated_at: value.0.updated_at,
            flag: common_utils::tokenization::TokenizationFlag::from_db_flag(value.0.flag),
            version: value.0.version,
        })
    }
}