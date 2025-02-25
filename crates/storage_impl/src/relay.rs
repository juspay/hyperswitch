use common_utils::{errors::CustomResult, types::keymanager::KeyManagerState};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store,
};
use sample::relay::RelayInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> RelayInterface for RouterStore<T> {
    type Error = errors::StorageError;

    async fn insert_relay(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        new: hyperswitch_domain_models::relay::Relay,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.construct_new()
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

    async fn update_relay(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        current_state: hyperswitch_domain_models::relay::Relay,
        relay_update: hyperswitch_domain_models::relay::RelayUpdate,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        Conversion::convert(current_state)
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .update(
                &conn,
                diesel_models::relay::RelayUpdateInternal::from(relay_update),
            )
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

    async fn find_relay_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        relay_id: &common_utils::id_type::RelayId,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        diesel_models::relay::Relay::find_by_id(&conn, relay_id)
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

    async fn find_relay_by_profile_id_connector_reference_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
        connector_reference_id: &str,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        diesel_models::relay::Relay::find_by_profile_id_connector_reference_id(
            &conn,
            profile_id,
            connector_reference_id,
        )
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
