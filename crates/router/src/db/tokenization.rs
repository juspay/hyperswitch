use error_stack::{ResultExt, Report, report};
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
    core::errors::{self, CustomResult},
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
        tokenization: domain_tokenization::Tokenization,
    ) -> CustomResult<domain_tokenization::Tokenization, StorageError>;
}

#[async_trait::async_trait]
impl TokenizationInterface for Store {
    async fn insert_tokenization(
        &self,
        tokenization: domain_tokenization::Tokenization,
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
        tokenization.construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
        
        // // First insert and get the ID
        // let inserted_row = diesel::insert_into(tokenization_dsl::tokenization)
        //     .values(tokenization_db)
        //     .returning(tokenization_dsl::id)
        //     .get_result_async::<uuid::Uuid>(&conn)
        //     .await
        //     .map_err(|error| StorageError::from(error))?;
            
        // // Then fetch the full record
        // tokenization_dsl::tokenization
        //     .filter(tokenization_dsl::id.eq(inserted_row))
        //     .first_async::<Tokenization>(&conn)
        //     .await
        //     .map_err(|error| StorageError::from(error))
        //     .map(TokenizationWrapper)
        //     .map(|wrapper| wrapper.try_into())
        //     .transpose()?
        //     .ok_or(StorageError::ValueNotFound("Tokenization not found after insert".into()).into())
    }
}

#[async_trait::async_trait]
impl TokenizationInterface for MockDb {
    async fn insert_tokenization(
        &self,
        tokenization: domain_tokenization::Tokenization,
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