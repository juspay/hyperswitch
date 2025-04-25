use error_stack::{ResultExt, Report, report};
use common_utils::{
    ext_traits::OptionExt,
    errors::CustomResult, 
    id_type::{CellId, GlobalTokenId, MerchantId}, 
    types::keymanager::KeyManagerState,
};
use diesel::{Insertable, RunQueryDsl, ExpressionMethods};
use tokio::time;
use async_bb8_diesel::AsyncRunQueryDsl;
use crate::{
    core,
    connection,
    errors ,
    services::Store,
};
use storage_impl::MockDb;
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
use hyperswitch_domain_models::tokenization as domain_tokenization;

#[async_trait::async_trait]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub trait TokenizationInterface {
    async fn insert_tokenization(
        &self,
        tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>;

    async fn get_entity_id_vault_id_by_token_id(
        &self,
        token: &common_utils::id_type::GlobalTokenId,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>;
}


#[async_trait::async_trait]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
impl TokenizationInterface for Store {
    
    async fn insert_tokenization(
        &self,
        tokenization: domain_tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState
    ) -> CustomResult<domain_tokenization::Tokenization, errors::StorageError> {
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

    async fn get_entity_id_vault_id_by_token_id(
        &self,
        token: &common_utils::id_type::GlobalTokenId,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState
    ) -> CustomResult<domain_tokenization::Tokenization, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        
        // Use the find_by_id method we just defined
        let tokenization = diesel_models::tokenization::Tokenization::find_by_id(&conn, token)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        let domain_tokenization = tokenization
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)?;
        
        Ok(domain_tokenization)
    }

}

#[async_trait::async_trait]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
impl TokenizationInterface for MockDb {
    
    async fn insert_tokenization(
        &self,
        tokenization: domain_tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState
    ) -> CustomResult<domain_tokenization::Tokenization, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
    async fn get_entity_id_vault_id_by_token_id(
        &self,
        token: &common_utils::id_type::GlobalTokenId,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState
    ) -> CustomResult<domain_tokenization::Tokenization, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
