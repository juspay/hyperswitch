use common_utils::types::keymanager::KeyManagerState;
use diesel_models;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::relay::RelayUpdate;
use storage_impl::{
    behaviour::{Conversion, ReverseConversion},
    MockDb,
};

use super::domain;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    services::Store,
    types::transformers::ForeignFrom,
};

impl ForeignFrom<RelayUpdate> for diesel_models::relay::RelayUpdateInternal {
    fn foreign_from(value: RelayUpdate) -> Self {
        match value {
            RelayUpdate::ErrorUpdate {
                error_code,
                error_message,
                status,
            } => Self {
                error_code: Some(error_code),
                error_message: Some(error_message),
                connector_reference_id: None,
                status: Some(status),
                modified_at: common_utils::date_time::now(),
            },
            RelayUpdate::StatusUpdate {
                connector_reference_id,
                status,
            } => Self {
                connector_reference_id,
                status: Some(status),
                error_code: None,
                error_message: None,
                modified_at: common_utils::date_time::now(),
            },
        }
    }
}

#[async_trait::async_trait]
pub trait RelayInterface {
    async fn insert_relay(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        new: hyperswitch_domain_models::relay::Relay,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError>;

    async fn update_relay(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: hyperswitch_domain_models::relay::Relay,
        relay_update: RelayUpdate,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError>;

    async fn find_relay_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        relay_id: &common_utils::id_type::RelayId,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError>;

    async fn find_relay_by_profile_id_connector_reference_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
        connector_reference_id: &str,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError>;
}

#[async_trait::async_trait]
impl RelayInterface for Store {
    async fn insert_relay(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
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
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: hyperswitch_domain_models::relay::Relay,
        relay_update: RelayUpdate,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        Conversion::convert(current_state)
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .update(
                &conn,
                diesel_models::relay::RelayUpdateInternal::foreign_from(relay_update),
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
        merchant_key_store: &domain::MerchantKeyStore,
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
        merchant_key_store: &domain::MerchantKeyStore,
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

#[async_trait::async_trait]
impl RelayInterface for MockDb {
    async fn insert_relay(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &domain::MerchantKeyStore,
        _new: hyperswitch_domain_models::relay::Relay,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_relay(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &domain::MerchantKeyStore,
        _current_state: hyperswitch_domain_models::relay::Relay,
        _relay_update: RelayUpdate,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_relay_by_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &domain::MerchantKeyStore,
        _relay_id: &common_utils::id_type::RelayId,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_relay_by_profile_id_connector_reference_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &domain::MerchantKeyStore,
        _profile_id: &common_utils::id_type::ProfileId,
        _connector_reference_id: &str,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl RelayInterface for KafkaStore {
    async fn insert_relay(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        new: hyperswitch_domain_models::relay::Relay,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        self.diesel_store
            .insert_relay(key_manager_state, merchant_key_store, new)
            .await
    }

    async fn update_relay(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: hyperswitch_domain_models::relay::Relay,
        relay_update: RelayUpdate,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        self.diesel_store
            .update_relay(
                key_manager_state,
                merchant_key_store,
                current_state,
                relay_update,
            )
            .await
    }

    async fn find_relay_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        relay_id: &common_utils::id_type::RelayId,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        self.diesel_store
            .find_relay_by_id(key_manager_state, merchant_key_store, relay_id)
            .await
    }

    async fn find_relay_by_profile_id_connector_reference_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
        connector_reference_id: &str,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
        self.diesel_store
            .find_relay_by_profile_id_connector_reference_id(
                key_manager_state,
                merchant_key_store,
                profile_id,
                connector_reference_id,
            )
            .await
    }
}
