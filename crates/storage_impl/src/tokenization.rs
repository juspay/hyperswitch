#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use common_utils::{errors::CustomResult, types::keymanager::KeyManagerState};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::tokenization::Tokenization;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use hyperswitch_domain_models::{
    merchant_key_store::MerchantKeyStore,
};

use common_utils::errors::ValidationError;
use common_utils::types::keymanager;
use masking::Secret;

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
        key_manager_state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>;

    async fn get_entity_id_vault_id_by_token_id(
        &self,
        token: &common_utils::id_type::GlobalTokenId,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>;
}

#[async_trait::async_trait]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
impl<T: DatabaseStore> TokenizationInterface for RouterStore<T> {
    async fn insert_tokenization(
        &self,
        tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState,
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
        key_manager_state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        let conn = connection::pg_connection_read(self).await?;

        let tokenization = diesel_models::tokenization::Tokenization::find_by_id(&conn, token)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        let domain = tokenization
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)?;

        Ok(domain)
    }
}

#[async_trait::async_trait]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
impl<T: DatabaseStore> TokenizationInterface for KVRouterStore<T> {
    async fn insert_tokenization(
        &self,
        tokenization: hyperswitch_domain_models::tokenization::Tokenization,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        self.router_store
            .insert_tokenization(tokenization, merchant_key_store, key_manager_state)
            .await
    }

    async fn get_entity_id_vault_id_by_token_id(
        &self,
        token: &common_utils::id_type::GlobalTokenId,
        merchant_key_store: &MerchantKeyStore,
        key_manager_state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        self.router_store
            .get_entity_id_vault_id_by_token_id(token, merchant_key_store, key_manager_state)
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
        _key_manager_state: &KeyManagerState,
    ) -> CustomResult<hyperswitch_domain_models::tokenization::Tokenization, errors::StorageError>
    {
        Err(errors::StorageError::MockDbError)?
    }
    async fn get_entity_id_vault_id_by_token_id(
        &self,
        _token: &common_utils::id_type::GlobalTokenId,
        _merchant_key_store: &MerchantKeyStore,
        _key_manager_state: &KeyManagerState,
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


#[async_trait::async_trait]
impl super::behaviour::Conversion for Tokenization {
    type DstType = diesel_models::tokenization::Tokenization;
    type NewDstType = diesel_models::tokenization::Tokenization;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::tokenization::Tokenization {
            id: self.id,
            merchant_id: self.merchant_id,
            customer_id: self.customer_id,
            locker_id: self.locker_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            version: self.version,
            flag: self.flag,
        })
    }

    async fn convert_back(
        _state: &keymanager::KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError> {
        Ok(Self {
            id: item.id,
            merchant_id: item.merchant_id,
            customer_id: item.customer_id,
            locker_id: item.locker_id,
            created_at: item.created_at,
            updated_at: item.updated_at,
            flag: item.flag,
            version: item.version,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::tokenization::Tokenization {
            id: self.id,
            merchant_id: self.merchant_id,
            customer_id: self.customer_id,
            locker_id: self.locker_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            version: self.version,
            flag: self.flag,
        })
    }
}
