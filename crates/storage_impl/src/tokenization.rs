use error_stack::{ResultExt, Report, report};
use common_utils::{
    ext_traits::OptionExt,
    errors::CustomResult, 
    id_type::{CellId, GlobalTokenId, MerchantId}, 
    types::keymanager::KeyManagerState,
};
use diesel::{Insertable, RunQueryDsl};
use tokio::time;
use async_bb8_diesel::AsyncRunQueryDsl;

use super::MockDb;
use crate::{
    connection,
    errors ,
    database::store::Store,
};

use diesel_models::{
    enums::TokenizationFlag as DbTokenizationFlag,
    tokenization::{Tokenization, TokenizationNew},
    schema_v2::tokenization::dsl as tokenization_dsl,
    PgPooledConn,
};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store::MerchantKeyStore,
};
use hyperswitch_domain_models::tokenization;


// New type wrapper to avoid orphan rule
#[derive(Debug, Clone)]
pub struct TokenizationWrapper(pub Tokenization);
#[async_trait::async_trait]
pub trait TokenizationInterface {
    async fn insert_tokenization(
        &self,
        tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>;
}


#[async_trait::async_trait]
impl TokenizationInterface for Store {

    async fn insert_tokenization(
        &self,
        tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

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
    }
}

#[async_trait::async_trait]
impl TokenizationInterface for MockDb {

    async fn insert_tokenization(
        &self,
        tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}