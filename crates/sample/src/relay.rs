
// use error_stack::{report, ResultExt};

// use storage_impl::MockDb;

// use super::domain;
// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     db::kafka_store::KafkaStore,
//     services::Store,
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{merchant_key_store};
use common_utils::types::keymanager::KeyManagerState;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait RelayInterface {
    type Error;
    async fn insert_relay(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        new: hyperswitch_domain_models::relay::Relay,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, Self::Error>;

    async fn update_relay(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        current_state: hyperswitch_domain_models::relay::Relay,
        relay_update: hyperswitch_domain_models::relay::RelayUpdate,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, Self::Error>;

    async fn find_relay_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        relay_id: &common_utils::id_type::RelayId,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, Self::Error>;

    async fn find_relay_by_profile_id_connector_reference_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
        connector_reference_id: &str,
    ) -> CustomResult<hyperswitch_domain_models::relay::Relay, Self::Error>;
}

// #[async_trait::async_trait]
// impl RelayInterface for MockDb {
//     async fn insert_relay(
//         &self,
//         _key_manager_state: &KeyManagerState,
//         _merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         _new: hyperswitch_domain_models::relay::Relay,
//     ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn update_relay(
//         &self,
//         _key_manager_state: &KeyManagerState,
//         _merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         _current_state: hyperswitch_domain_models::relay::Relay,
//         _relay_update: hyperswitch_domain_models::relay::RelayUpdate,
//     ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_relay_by_id(
//         &self,
//         _key_manager_state: &KeyManagerState,
//         _merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         _relay_id: &common_utils::id_type::RelayId,
//     ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_relay_by_profile_id_connector_reference_id(
//         &self,
//         _key_manager_state: &KeyManagerState,
//         _merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         _profile_id: &common_utils::id_type::ProfileId,
//         _connector_reference_id: &str,
//     ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }
// }

// #[async_trait::async_trait]
// impl RelayInterface for KafkaStore {
//     async fn insert_relay(
//         &self,
//         key_manager_state: &KeyManagerState,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         new: hyperswitch_domain_models::relay::Relay,
//     ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
//         self.diesel_store
//             .insert_relay(key_manager_state, merchant_key_store, new)
//             .await
//     }

//     async fn update_relay(
//         &self,
//         key_manager_state: &KeyManagerState,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         current_state: hyperswitch_domain_models::relay::Relay,
//         relay_update: hyperswitch_domain_models::relay::RelayUpdate,
//     ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
//         self.diesel_store
//             .update_relay(
//                 key_manager_state,
//                 merchant_key_store,
//                 current_state,
//                 relay_update,
//             )
//             .await
//     }

//     async fn find_relay_by_id(
//         &self,
//         key_manager_state: &KeyManagerState,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         relay_id: &common_utils::id_type::RelayId,
//     ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
//         self.diesel_store
//             .find_relay_by_id(key_manager_state, merchant_key_store, relay_id)
//             .await
//     }

//     async fn find_relay_by_profile_id_connector_reference_id(
//         &self,
//         key_manager_state: &KeyManagerState,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         profile_id: &common_utils::id_type::ProfileId,
//         connector_reference_id: &str,
//     ) -> CustomResult<hyperswitch_domain_models::relay::Relay, errors::StorageError> {
//         self.diesel_store
//             .find_relay_by_profile_id_connector_reference_id(
//                 key_manager_state,
//                 merchant_key_store,
//                 profile_id,
//                 connector_reference_id,
//             )
//             .await
//     }
// }
