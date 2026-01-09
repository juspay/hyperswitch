#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use common_utils::errors::CustomResult;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use diesel_models::tokenization as tokenization_diesel;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use error_stack::{report, ResultExt};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store::MerchantKeyStore,
};

use super::MockDb;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use crate::{connection, errors};
use crate::{kv_router_store::KVRouterStore, DatabaseStore, RouterStore};

#[cfg(not(all(feature = "v2", feature = "tokenization_v2")))]
pub trait TokenizationInterface {}

#[async_trait::async_trait]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub trait TokenizationInterface {
    async fn insert_tokenization(
        &self,
        tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>;

    async fn get_entity_id_vault_id_by_token_id(
        &self,
        token: &common_utils::id_type::GlobalTokenId,
        merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>;

    async fn update_tokenization_record(
        &self,
        tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        tokenization_update: hyperswitch_domain_models::tokenization::TokenizationUpdate,
        merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>;
}

#[async_trait::async_trait]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
impl<T: DatabaseStore> TokenizationInterface for RouterStore<T> {
    async fn insert_tokenization(
        &self,
        tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        let conn = connection::pg_connection_write(self).await?;

        tokenization
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
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
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        let conn = connection::pg_connection_read(self).await?;

        let tokenization = tokenization_diesel::Tokenization::find_by_id(&conn, token)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        let domain = tokenization
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)?;

        Ok(domain)
    }

    async fn update_tokenization_record(
        &self,
        tokenization_record: hyperswitch_domain_models::tokenization::Tokenization,
        tokenization_update: hyperswitch_domain_models::tokenization::TokenizationUpdate,
        merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        let conn = connection::pg_connection_write(self).await?;

        let tokenization_record = Conversion::convert(tokenization_record)
            .await
            .change_context(errors::StorageError::DecryptionError)?;
        self.call_database(
            merchant_key_store,
            tokenization_record.update_with_id(
                &conn,
                tokenization_diesel::TokenizationUpdateInternal::from(tokenization_update),
            ),
        )
        .await
    }
}

#[async_trait::async_trait]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
impl<T: DatabaseStore> TokenizationInterface for KVRouterStore<T> {
    async fn insert_tokenization(
        &self,
        tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        self.router_store
            .insert_tokenization(tokenization, merchant_key_store)
            .await
    }

    async fn get_entity_id_vault_id_by_token_id(
        &self,
        token: &common_utils::id_type::GlobalTokenId,
        merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        self.router_store
            .get_entity_id_vault_id_by_token_id(token, merchant_key_store)
            .await
    }

    async fn update_tokenization_record(
        &self,
        tokenization_record: hyperswitch_domain_models::tokenization::Tokenization,
        tokenization_update: hyperswitch_domain_models::tokenization::TokenizationUpdate,
        merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        self.router_store
            .update_tokenization_record(
                tokenization_record,
                tokenization_update,
                merchant_key_store,
            )
            .await
    }
}

#[async_trait::async_trait]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
impl TokenizationInterface for MockDb {
    async fn insert_tokenization(
        &self,
        _tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        _merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        Err(errors::StorageError::MockDbError)?
    }
    async fn get_entity_id_vault_id_by_token_id(
        &self,
        _token: &common_utils::id_type::GlobalTokenId,
        _merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_tokenization_record(
        &self,
        _tokenization_record: hyperswitch_domain_models::tokenization::Tokenization,
        _tokenization_update: hyperswitch_domain_models::tokenization::TokenizationUpdate,
        _merchant_key_store: &MerchantKeyStore,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        Err(errors::StorageError::MockDbError)?
    }
}

#[cfg(not(all(feature = "v2", feature = "tokenization_v2")))]
impl TokenizationInterface for MockDb {}

#[cfg(not(all(feature = "v2", feature = "tokenization_v2")))]
impl<T: DatabaseStore> TokenizationInterface for KVRouterStore<T> {}

#[cfg(not(all(feature = "v2", feature = "tokenization_v2")))]
impl<T: DatabaseStore> TokenizationInterface for RouterStore<T> {}
