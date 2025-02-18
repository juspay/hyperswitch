use std::sync::Arc;

use common_enums::enums::MerchantStorageScheme;
use common_utils::{
    errors::CustomResult,
    id_type,
    types::{keymanager::KeyManagerState, theme::ThemeLineage},
};
#[cfg(feature = "v2")]
use diesel_models::ephemeral_key::{ClientSecretType, ClientSecretTypeNew};
use diesel_models::{
    enums,
    enums::ProcessTrackerStatus,
    ephemeral_key::{EphemeralKey, EphemeralKeyNew},
    reverse_lookup::{ReverseLookup, ReverseLookupNew},
    user_role as user_storage,
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::payouts::{
    payout_attempt::PayoutAttemptInterface, payouts::PayoutsInterface,
};
use hyperswitch_domain_models::{
    disputes,
    payments::{payment_attempt::PaymentAttemptInterface, payment_intent::PaymentIntentInterface},
    refunds,
};
#[cfg(not(feature = "payouts"))]
use hyperswitch_domain_models::{PayoutAttemptInterface, PayoutsInterface};
use masking::Secret;
use redis_interface::{errors::RedisError, RedisConnectionPool, RedisEntryId};
use router_env::{instrument, logger, tracing};
use scheduler::{
    db::{process_tracker::ProcessTrackerInterface, queue::QueueInterface},
    SchedulerInterface,
};
use serde::Serialize;
use storage_impl::{config::TenantConfig, redis::kv_store::RedisConnInterface};
use time::PrimitiveDateTime;

use super::{
    dashboard_metadata::DashboardMetadataInterface,
    ephemeral_key::ClientSecretInterface,
    role::RoleInterface,
    user::{sample_data::BatchSampleDataInterface, theme::ThemeInterface, UserInterface},
    user_authentication_method::UserAuthenticationMethodInterface,
    user_key_store::UserKeyStoreInterface,
    user_role::{ListUserRolesByOrgIdPayload, ListUserRolesByUserIdPayload, UserRoleInterface},
};
#[cfg(feature = "payouts")]
use crate::services::kafka::payout::KafkaPayout;
use crate::{
    core::errors::{self, ProcessTrackerError},
    db::{
        self,
        address::AddressInterface,
        api_keys::ApiKeyInterface,
        authentication::AuthenticationInterface,
        authorization::AuthorizationInterface,
        business_profile::ProfileInterface,
        callback_mapper::CallbackMapperInterface,
        capture::CaptureInterface,
        cards_info::CardsInfoInterface,
        configs::ConfigInterface,
        customers::CustomerInterface,
        dispute::DisputeInterface,
        ephemeral_key::EphemeralKeyInterface,
        events::EventInterface,
        file::FileMetadataInterface,
        generic_link::GenericLinkInterface,
        gsm::GsmInterface,
        health_check::HealthCheckDbInterface,
        locker_mock_up::LockerMockUpInterface,
        mandate::MandateInterface,
        merchant_account::MerchantAccountInterface,
        merchant_connector_account::{ConnectorAccessToken, MerchantConnectorAccountInterface},
        merchant_key_store::MerchantKeyStoreInterface,
        payment_link::PaymentLinkInterface,
        payment_method::PaymentMethodInterface,
        refund::RefundInterface,
        reverse_lookup::ReverseLookupInterface,
        routing_algorithm::RoutingAlgorithmInterface,
        unified_translations::UnifiedTranslationsInterface,
        AccountsStorageInterface, CommonStorageInterface, GlobalStorageInterface,
        MasterKeyInterface, StorageInterface,
    },
    services::{kafka::KafkaProducer, Store},
    types::{domain, storage, AccessToken},
};

#[derive(Debug, Clone, Serialize)]
pub struct TenantID(pub String);

#[derive(Clone)]
pub struct KafkaStore {
    pub kafka_producer: KafkaProducer,
    pub diesel_store: Store,
    pub tenant_id: TenantID,
}

impl KafkaStore {
    pub async fn new(
        store: Store,
        mut kafka_producer: KafkaProducer,
        tenant_id: TenantID,
        tenant_config: &dyn TenantConfig,
    ) -> Self {
        kafka_producer.set_tenancy(tenant_config);
        Self {
            kafka_producer,
            diesel_store: store,
            tenant_id,
        }
    }
}

#[async_trait::async_trait]
impl AddressInterface for KafkaStore {
    async fn find_address_by_address_id(
        &self,
        state: &KeyManagerState,
        address_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        self.diesel_store
            .find_address_by_address_id(state, address_id, key_store)
            .await
    }

    async fn update_address(
        &self,
        state: &KeyManagerState,
        address_id: String,
        address: storage::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        self.diesel_store
            .update_address(state, address_id, address, key_store)
            .await
    }

    async fn update_address_for_payments(
        &self,
        state: &KeyManagerState,
        this: domain::PaymentAddress,
        address: domain::AddressUpdate,
        payment_id: id_type::PaymentId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentAddress, errors::StorageError> {
        self.diesel_store
            .update_address_for_payments(
                state,
                this,
                address,
                payment_id,
                key_store,
                storage_scheme,
            )
            .await
    }

    async fn insert_address_for_payments(
        &self,
        state: &KeyManagerState,
        payment_id: &id_type::PaymentId,
        address: domain::PaymentAddress,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentAddress, errors::StorageError> {
        self.diesel_store
            .insert_address_for_payments(state, payment_id, address, key_store, storage_scheme)
            .await
    }

    async fn find_address_by_merchant_id_payment_id_address_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        payment_id: &id_type::PaymentId,
        address_id: &str,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentAddress, errors::StorageError> {
        self.diesel_store
            .find_address_by_merchant_id_payment_id_address_id(
                state,
                merchant_id,
                payment_id,
                address_id,
                key_store,
                storage_scheme,
            )
            .await
    }

    async fn insert_address_for_customers(
        &self,
        state: &KeyManagerState,
        address: domain::CustomerAddress,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        self.diesel_store
            .insert_address_for_customers(state, address, key_store)
            .await
    }

    async fn update_address_by_merchant_id_customer_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        address: storage::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Address>, errors::StorageError> {
        self.diesel_store
            .update_address_by_merchant_id_customer_id(
                state,
                customer_id,
                merchant_id,
                address,
                key_store,
            )
            .await
    }
}

#[async_trait::async_trait]
impl ApiKeyInterface for KafkaStore {
    async fn insert_api_key(
        &self,
        api_key: storage::ApiKeyNew,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        self.diesel_store.insert_api_key(api_key).await
    }

    async fn update_api_key(
        &self,
        merchant_id: id_type::MerchantId,
        key_id: id_type::ApiKeyId,
        api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        self.diesel_store
            .update_api_key(merchant_id, key_id, api_key)
            .await
    }

    async fn revoke_api_key(
        &self,
        merchant_id: &id_type::MerchantId,
        key_id: &id_type::ApiKeyId,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store.revoke_api_key(merchant_id, key_id).await
    }

    async fn find_api_key_by_merchant_id_key_id_optional(
        &self,
        merchant_id: &id_type::MerchantId,
        key_id: &id_type::ApiKeyId,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        self.diesel_store
            .find_api_key_by_merchant_id_key_id_optional(merchant_id, key_id)
            .await
    }

    async fn find_api_key_by_hash_optional(
        &self,
        hashed_api_key: storage::HashedApiKey,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        self.diesel_store
            .find_api_key_by_hash_optional(hashed_api_key)
            .await
    }

    async fn list_api_keys_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<storage::ApiKey>, errors::StorageError> {
        self.diesel_store
            .list_api_keys_by_merchant_id(merchant_id, limit, offset)
            .await
    }
}

#[async_trait::async_trait]
impl CardsInfoInterface for KafkaStore {
    async fn get_card_info(
        &self,
        card_iin: &str,
    ) -> CustomResult<Option<storage::CardInfo>, errors::StorageError> {
        self.diesel_store.get_card_info(card_iin).await
    }
}

#[async_trait::async_trait]
impl ConfigInterface for KafkaStore {
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.diesel_store.insert_config(config).await
    }

    async fn find_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.diesel_store.find_config_by_key(key).await
    }

    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.diesel_store.find_config_by_key_from_db(key).await
    }

    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.diesel_store
            .update_config_in_database(key, config_update)
            .await
    }

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.diesel_store
            .update_config_by_key(key, config_update)
            .await
    }

    async fn delete_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.diesel_store.delete_config_by_key(key).await
    }

    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        default_config: Option<String>,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.diesel_store
            .find_config_by_key_unwrap_or(key, default_config)
            .await
    }
}

#[async_trait::async_trait]
impl CustomerInterface for KafkaStore {
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_customer_by_customer_id_merchant_id(customer_id, merchant_id)
            .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
        self.diesel_store
            .find_customer_optional_by_customer_id_merchant_id(
                state,
                customer_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
        self.diesel_store
            .find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
                state,
                customer_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_optional_by_merchant_id_merchant_reference_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
        self.diesel_store
            .find_optional_by_merchant_id_merchant_reference_id(
                state,
                customer_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn update_customer_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: id_type::CustomerId,
        merchant_id: id_type::MerchantId,
        customer: domain::Customer,
        customer_update: storage::CustomerUpdate,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        self.diesel_store
            .update_customer_by_customer_id_merchant_id(
                state,
                customer_id,
                merchant_id,
                customer,
                customer_update,
                key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn update_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        customer: domain::Customer,
        merchant_id: &id_type::MerchantId,
        customer_update: storage::CustomerUpdate,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        self.diesel_store
            .update_customer_by_global_id(
                state,
                id,
                customer,
                merchant_id,
                customer_update,
                key_store,
                storage_scheme,
            )
            .await
    }

    async fn list_customers_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        constraints: super::customers::CustomerListConstraints,
    ) -> CustomResult<Vec<domain::Customer>, errors::StorageError> {
        self.diesel_store
            .list_customers_by_merchant_id(state, merchant_id, key_store, constraints)
            .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        self.diesel_store
            .find_customer_by_customer_id_merchant_id(
                state,
                customer_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_customer_by_merchant_reference_id_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_reference_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        self.diesel_store
            .find_customer_by_merchant_reference_id_merchant_id(
                state,
                merchant_reference_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        self.diesel_store
            .find_customer_by_global_id(state, id, merchant_id, key_store, storage_scheme)
            .await
    }

    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        self.diesel_store
            .insert_customer(customer_data, state, key_store, storage_scheme)
            .await
    }
}

#[async_trait::async_trait]
impl DisputeInterface for KafkaStore {
    async fn insert_dispute(
        &self,
        dispute_new: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let dispute = self.diesel_store.insert_dispute(dispute_new).await?;

        if let Err(er) = self
            .kafka_producer
            .log_dispute(&dispute, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to add analytics entry for Dispute {dispute:?}", error_message=?er);
        };

        Ok(dispute)
    }

    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &id_type::MerchantId,
        payment_id: &id_type::PaymentId,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError> {
        self.diesel_store
            .find_by_merchant_id_payment_id_connector_dispute_id(
                merchant_id,
                payment_id,
                connector_dispute_id,
            )
            .await
    }

    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &id_type::MerchantId,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        self.diesel_store
            .find_dispute_by_merchant_id_dispute_id(merchant_id, dispute_id)
            .await
    }

    async fn find_disputes_by_constraints(
        &self,
        merchant_id: &id_type::MerchantId,
        dispute_constraints: &disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        self.diesel_store
            .find_disputes_by_constraints(merchant_id, dispute_constraints)
            .await
    }

    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let dispute_new = self
            .diesel_store
            .update_dispute(this.clone(), dispute)
            .await?;
        if let Err(er) = self
            .kafka_producer
            .log_dispute(&dispute_new, Some(this), self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to add analytics entry for Dispute {dispute_new:?}", error_message=?er);
        };

        Ok(dispute_new)
    }

    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &id_type::MerchantId,
        payment_id: &id_type::PaymentId,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        self.diesel_store
            .find_disputes_by_merchant_id_payment_id(merchant_id, payment_id)
            .await
    }

    async fn get_dispute_status_with_count(
        &self,
        merchant_id: &id_type::MerchantId,
        profile_id_list: Option<Vec<id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(common_enums::DisputeStatus, i64)>, errors::StorageError> {
        self.diesel_store
            .get_dispute_status_with_count(merchant_id, profile_id_list, time_range)
            .await
    }
}

#[async_trait::async_trait]
impl EphemeralKeyInterface for KafkaStore {
    #[cfg(feature = "v1")]
    async fn create_ephemeral_key(
        &self,
        ek: EphemeralKeyNew,
        validity: i64,
    ) -> CustomResult<EphemeralKey, errors::StorageError> {
        self.diesel_store.create_ephemeral_key(ek, validity).await
    }

    #[cfg(feature = "v1")]
    async fn get_ephemeral_key(
        &self,
        key: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError> {
        self.diesel_store.get_ephemeral_key(key).await
    }

    #[cfg(feature = "v1")]
    async fn delete_ephemeral_key(
        &self,
        id: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError> {
        self.diesel_store.delete_ephemeral_key(id).await
    }
}

#[async_trait::async_trait]
impl ClientSecretInterface for KafkaStore {
    #[cfg(feature = "v2")]
    async fn create_client_secret(
        &self,
        ek: ClientSecretTypeNew,
        validity: i64,
    ) -> CustomResult<ClientSecretType, errors::StorageError> {
        self.diesel_store.create_client_secret(ek, validity).await
    }

    #[cfg(feature = "v2")]
    async fn get_client_secret(
        &self,
        key: &str,
    ) -> CustomResult<ClientSecretType, errors::StorageError> {
        self.diesel_store.get_client_secret(key).await
    }

    #[cfg(feature = "v2")]
    async fn delete_client_secret(
        &self,
        id: &str,
    ) -> CustomResult<ClientSecretType, errors::StorageError> {
        self.diesel_store.delete_client_secret(id).await
    }
}

#[async_trait::async_trait]
impl EventInterface for KafkaStore {
    async fn insert_event(
        &self,
        state: &KeyManagerState,
        event: domain::Event,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        self.diesel_store
            .insert_event(state, event, merchant_key_store)
            .await
    }

    async fn find_event_by_merchant_id_event_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        event_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        self.diesel_store
            .find_event_by_merchant_id_event_id(state, merchant_id, event_id, merchant_key_store)
            .await
    }

    async fn list_initial_events_by_merchant_id_primary_object_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        primary_object_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        self.diesel_store
            .list_initial_events_by_merchant_id_primary_object_id(
                state,
                merchant_id,
                primary_object_id,
                merchant_key_store,
            )
            .await
    }

    async fn list_initial_events_by_merchant_id_constraints(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        created_after: Option<PrimitiveDateTime>,
        created_before: Option<PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        self.diesel_store
            .list_initial_events_by_merchant_id_constraints(
                state,
                merchant_id,
                created_after,
                created_before,
                limit,
                offset,
                merchant_key_store,
            )
            .await
    }

    async fn list_events_by_merchant_id_initial_attempt_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        initial_attempt_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        self.diesel_store
            .list_events_by_merchant_id_initial_attempt_id(
                state,
                merchant_id,
                initial_attempt_id,
                merchant_key_store,
            )
            .await
    }

    async fn list_initial_events_by_profile_id_primary_object_id(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        primary_object_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        self.diesel_store
            .list_initial_events_by_profile_id_primary_object_id(
                state,
                profile_id,
                primary_object_id,
                merchant_key_store,
            )
            .await
    }

    async fn list_initial_events_by_profile_id_constraints(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        created_after: Option<PrimitiveDateTime>,
        created_before: Option<PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Event>, errors::StorageError> {
        self.diesel_store
            .list_initial_events_by_profile_id_constraints(
                state,
                profile_id,
                created_after,
                created_before,
                limit,
                offset,
                merchant_key_store,
            )
            .await
    }

    async fn update_event_by_merchant_id_event_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        event_id: &str,
        event: domain::EventUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Event, errors::StorageError> {
        self.diesel_store
            .update_event_by_merchant_id_event_id(
                state,
                merchant_id,
                event_id,
                event,
                merchant_key_store,
            )
            .await
    }
}

#[async_trait::async_trait]
impl LockerMockUpInterface for KafkaStore {
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        self.diesel_store.find_locker_by_card_id(card_id).await
    }

    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        self.diesel_store.insert_locker_mock_up(new).await
    }

    async fn delete_locker_mock_up(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        self.diesel_store.delete_locker_mock_up(card_id).await
    }
}

#[async_trait::async_trait]
impl MandateInterface for KafkaStore {
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &id_type::MerchantId,
        mandate_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        self.diesel_store
            .find_mandate_by_merchant_id_mandate_id(merchant_id, mandate_id, storage_scheme)
            .await
    }

    async fn find_mandate_by_merchant_id_connector_mandate_id(
        &self,
        merchant_id: &id_type::MerchantId,
        connector_mandate_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        self.diesel_store
            .find_mandate_by_merchant_id_connector_mandate_id(
                merchant_id,
                connector_mandate_id,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_mandate_by_global_customer_id(
        &self,
        id: &id_type::GlobalCustomerId,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        self.diesel_store
            .find_mandate_by_global_customer_id(id)
            .await
    }

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        self.diesel_store
            .find_mandate_by_merchant_id_customer_id(merchant_id, customer_id)
            .await
    }

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &id_type::MerchantId,
        mandate_id: &str,
        mandate_update: storage::MandateUpdate,
        mandate: storage::Mandate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        self.diesel_store
            .update_mandate_by_merchant_id_mandate_id(
                merchant_id,
                mandate_id,
                mandate_update,
                mandate,
                storage_scheme,
            )
            .await
    }

    async fn find_mandates_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        mandate_constraints: api_models::mandates::MandateListConstraints,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        self.diesel_store
            .find_mandates_by_merchant_id(merchant_id, mandate_constraints)
            .await
    }

    async fn insert_mandate(
        &self,
        mandate: storage::MandateNew,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        self.diesel_store
            .insert_mandate(mandate, storage_scheme)
            .await
    }
}

#[async_trait::async_trait]
impl PaymentLinkInterface for KafkaStore {
    async fn find_payment_link_by_payment_link_id(
        &self,
        payment_link_id: &str,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        self.diesel_store
            .find_payment_link_by_payment_link_id(payment_link_id)
            .await
    }

    async fn insert_payment_link(
        &self,
        payment_link_object: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        self.diesel_store
            .insert_payment_link(payment_link_object)
            .await
    }

    async fn list_payment_link_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        payment_link_constraints: api_models::payments::PaymentLinkListConstraints,
    ) -> CustomResult<Vec<storage::PaymentLink>, errors::StorageError> {
        self.diesel_store
            .list_payment_link_by_merchant_id(merchant_id, payment_link_constraints)
            .await
    }
}

#[async_trait::async_trait]
impl MerchantAccountInterface for KafkaStore {
    async fn insert_merchant(
        &self,
        state: &KeyManagerState,
        merchant_account: domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        self.diesel_store
            .insert_merchant(state, merchant_account, key_store)
            .await
    }

    async fn find_merchant_account_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        self.diesel_store
            .find_merchant_account_by_merchant_id(state, merchant_id, key_store)
            .await
    }

    async fn update_merchant(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantAccount,
        merchant_account: storage::MerchantAccountUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        self.diesel_store
            .update_merchant(state, this, merchant_account, key_store)
            .await
    }

    async fn update_specific_fields_in_merchant(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        merchant_account: storage::MerchantAccountUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        self.diesel_store
            .update_specific_fields_in_merchant(state, merchant_id, merchant_account, key_store)
            .await
    }

    async fn update_all_merchant_account(
        &self,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .update_all_merchant_account(merchant_account)
            .await
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        state: &KeyManagerState,
        publishable_key: &str,
    ) -> CustomResult<(domain::MerchantAccount, domain::MerchantKeyStore), errors::StorageError>
    {
        self.diesel_store
            .find_merchant_account_by_publishable_key(state, publishable_key)
            .await
    }

    #[cfg(feature = "olap")]
    async fn list_merchant_accounts_by_organization_id(
        &self,
        state: &KeyManagerState,
        organization_id: &id_type::OrganizationId,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        self.diesel_store
            .list_merchant_accounts_by_organization_id(state, organization_id)
            .await
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_merchant_account_by_merchant_id(merchant_id)
            .await
    }

    #[cfg(feature = "olap")]
    async fn list_multiple_merchant_accounts(
        &self,
        state: &KeyManagerState,
        merchant_ids: Vec<id_type::MerchantId>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        self.diesel_store
            .list_multiple_merchant_accounts(state, merchant_ids)
            .await
    }

    #[cfg(feature = "olap")]
    async fn list_merchant_and_org_ids(
        &self,
        state: &KeyManagerState,
        limit: u32,
        offset: Option<u32>,
    ) -> CustomResult<Vec<(id_type::MerchantId, id_type::OrganizationId)>, errors::StorageError>
    {
        self.diesel_store
            .list_merchant_and_org_ids(state, limit, offset)
            .await
    }
}

#[async_trait::async_trait]
impl ConnectorAccessToken for KafkaStore {
    async fn get_access_token(
        &self,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &str,
    ) -> CustomResult<Option<AccessToken>, errors::StorageError> {
        self.diesel_store
            .get_access_token(merchant_id, merchant_connector_id)
            .await
    }

    async fn set_access_token(
        &self,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &str,
        access_token: AccessToken,
    ) -> CustomResult<(), errors::StorageError> {
        self.diesel_store
            .set_access_token(merchant_id, merchant_connector_id, access_token)
            .await
    }
}

#[async_trait::async_trait]
impl FileMetadataInterface for KafkaStore {
    async fn insert_file_metadata(
        &self,
        file: storage::FileMetadataNew,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        self.diesel_store.insert_file_metadata(file).await
    }

    async fn find_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &id_type::MerchantId,
        file_id: &str,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        self.diesel_store
            .find_file_metadata_by_merchant_id_file_id(merchant_id, file_id)
            .await
    }

    async fn delete_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &id_type::MerchantId,
        file_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_file_metadata_by_merchant_id_file_id(merchant_id, file_id)
            .await
    }

    async fn update_file_metadata(
        &self,
        this: storage::FileMetadata,
        file_metadata: storage::FileMetadataUpdate,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        self.diesel_store
            .update_file_metadata(this, file_metadata)
            .await
    }
}

#[async_trait::async_trait]
impl MerchantConnectorAccountInterface for KafkaStore {
    async fn update_multiple_merchant_connector_accounts(
        &self,
        merchant_connector_accounts: Vec<(
            domain::MerchantConnectorAccount,
            storage::MerchantConnectorAccountUpdateInternal,
        )>,
    ) -> CustomResult<(), errors::StorageError> {
        self.diesel_store
            .update_multiple_merchant_connector_accounts(merchant_connector_accounts)
            .await
    }
    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        connector: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .find_merchant_connector_account_by_merchant_id_connector_label(
                state,
                merchant_id,
                connector,
                key_store,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        self.diesel_store
            .find_merchant_connector_account_by_merchant_id_connector_name(
                state,
                merchant_id,
                connector_name,
                key_store,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .find_merchant_connector_account_by_profile_id_connector_name(
                state,
                profile_id,
                connector_name,
                key_store,
            )
            .await
    }

    #[cfg(all(feature = "oltp", feature = "v2"))]
    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        key_store: &domain::MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        self.list_enabled_connector_accounts_by_profile_id(
            state,
            profile_id,
            key_store,
            connector_type,
        )
        .await
    }

    async fn insert_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        t: domain::MerchantConnectorAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .insert_merchant_connector_account(state, t, key_store)
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &id_type::MerchantConnectorAccountId,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                state,
                merchant_id,
                merchant_connector_id,
                key_store,
            )
            .await
    }

    #[cfg(feature = "v2")]
    async fn find_merchant_connector_account_by_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::MerchantConnectorAccountId,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .find_merchant_connector_account_by_id(state, id, key_store)
            .await
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        get_disabled: bool,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        self.diesel_store
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                state,
                merchant_id,
                get_disabled,
                key_store,
            )
            .await
    }

    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        self.diesel_store
            .list_connector_account_by_profile_id(state, profile_id, key_store)
            .await
    }

    async fn update_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .update_merchant_connector_account(state, this, merchant_connector_account, key_store)
            .await
    }

    #[cfg(feature = "v1")]
    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
                merchant_id,
                merchant_connector_id,
            )
            .await
    }

    #[cfg(feature = "v2")]
    async fn delete_merchant_connector_account_by_id(
        &self,
        id: &id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_merchant_connector_account_by_id(id)
            .await
    }
}

#[async_trait::async_trait]
impl QueueInterface for KafkaStore {
    async fn fetch_consumer_tasks(
        &self,
        stream_name: &str,
        group_name: &str,
        consumer_name: &str,
    ) -> CustomResult<Vec<storage::ProcessTracker>, ProcessTrackerError> {
        self.diesel_store
            .fetch_consumer_tasks(stream_name, group_name, consumer_name)
            .await
    }

    async fn consumer_group_create(
        &self,
        stream: &str,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<(), RedisError> {
        self.diesel_store
            .consumer_group_create(stream, group, id)
            .await
    }

    async fn acquire_pt_lock(
        &self,
        tag: &str,
        lock_key: &str,
        lock_val: &str,
        ttl: i64,
    ) -> CustomResult<bool, RedisError> {
        self.diesel_store
            .acquire_pt_lock(tag, lock_key, lock_val, ttl)
            .await
    }

    async fn release_pt_lock(&self, tag: &str, lock_key: &str) -> CustomResult<bool, RedisError> {
        self.diesel_store.release_pt_lock(tag, lock_key).await
    }

    async fn stream_append_entry(
        &self,
        stream: &str,
        entry_id: &RedisEntryId,
        fields: Vec<(&str, String)>,
    ) -> CustomResult<(), RedisError> {
        self.diesel_store
            .stream_append_entry(stream, entry_id, fields)
            .await
    }

    async fn get_key(&self, key: &str) -> CustomResult<Vec<u8>, RedisError> {
        self.diesel_store.get_key(key).await
    }
}

#[async_trait::async_trait]
impl PaymentAttemptInterface for KafkaStore {
    #[cfg(feature = "v1")]
    async fn insert_payment_attempt(
        &self,
        payment_attempt: storage::PaymentAttemptNew,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        let attempt = self
            .diesel_store
            .insert_payment_attempt(payment_attempt, storage_scheme)
            .await?;

        if let Err(er) = self
            .kafka_producer
            .log_payment_attempt(&attempt, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to log analytics event for payment attempt {attempt:?}", error_message=?er)
        }

        Ok(attempt)
    }

    #[cfg(feature = "v2")]
    async fn insert_payment_attempt(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        payment_attempt: storage::PaymentAttempt,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        let attempt = self
            .diesel_store
            .insert_payment_attempt(
                key_manager_state,
                merchant_key_store,
                payment_attempt,
                storage_scheme,
            )
            .await?;

        if let Err(er) = self
            .kafka_producer
            .log_payment_attempt(&attempt, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to log analytics event for payment attempt {attempt:?}", error_message=?er)
        }

        Ok(attempt)
    }

    #[cfg(feature = "v1")]
    async fn update_payment_attempt_with_attempt_id(
        &self,
        this: storage::PaymentAttempt,
        payment_attempt: storage::PaymentAttemptUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        let attempt = self
            .diesel_store
            .update_payment_attempt_with_attempt_id(this.clone(), payment_attempt, storage_scheme)
            .await?;

        if let Err(er) = self
            .kafka_producer
            .log_payment_attempt(&attempt, Some(this), self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to log analytics event for payment attempt {attempt:?}", error_message=?er)
        }

        Ok(attempt)
    }

    #[cfg(feature = "v2")]
    async fn update_payment_attempt(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        this: storage::PaymentAttempt,
        payment_attempt: storage::PaymentAttemptUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        let attempt = self
            .diesel_store
            .update_payment_attempt(
                key_manager_state,
                merchant_key_store,
                this.clone(),
                payment_attempt,
                storage_scheme,
            )
            .await?;

        if let Err(er) = self
            .kafka_producer
            .log_payment_attempt(&attempt, Some(this), self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to log analytics event for payment attempt {attempt:?}", error_message=?er)
        }

        Ok(attempt)
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        connector_transaction_id: &common_utils::types::ConnectorTransactionId,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
                connector_transaction_id,
                payment_id,
                merchant_id,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &id_type::MerchantId,
        connector_txn_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempt_by_merchant_id_connector_txn_id(
                merchant_id,
                connector_txn_id,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_attempt_by_profile_id_connector_transaction_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_id: &id_type::ProfileId,
        connector_transaction_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempt_by_profile_id_connector_transaction_id(
                key_manager_state,
                merchant_key_store,
                profile_id,
                connector_transaction_id,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        attempt_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                payment_id,
                merchant_id,
                attempt_id,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        attempt_id: &str,
        merchant_id: &id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempt_by_attempt_id_merchant_id(attempt_id, merchant_id, storage_scheme)
            .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_attempt_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        attempt_id: &id_type::GlobalAttemptId,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempt_by_id(
                key_manager_state,
                merchant_key_store,
                attempt_id,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_attempts_by_payment_intent_id(
        &self,
        key_manager_state: &KeyManagerState,
        payment_id: &id_type::GlobalPaymentId,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<storage::PaymentAttempt>, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempts_by_payment_intent_id(
                key_manager_state,
                payment_id,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
                payment_id,
                merchant_id,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
                payment_id,
                merchant_id,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        preprocessing_id: &str,
        merchant_id: &id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempt_by_preprocessing_id_merchant_id(
                preprocessing_id,
                merchant_id,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn get_filters_for_payments(
        &self,
        pi: &[hyperswitch_domain_models::payments::PaymentIntent],
        merchant_id: &id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<
        hyperswitch_domain_models::payments::payment_attempt::PaymentListFilters,
        errors::DataStorageError,
    > {
        self.diesel_store
            .get_filters_for_payments(pi, merchant_id, storage_scheme)
            .await
    }

    #[cfg(feature = "v1")]
    async fn get_total_count_of_filtered_payment_attempts(
        &self,
        merchant_id: &id_type::MerchantId,
        active_attempt_ids: &[String],
        connector: Option<Vec<api_models::enums::Connector>>,
        payment_method: Option<Vec<common_enums::PaymentMethod>>,
        payment_method_type: Option<Vec<common_enums::PaymentMethodType>>,
        authentication_type: Option<Vec<common_enums::AuthenticationType>>,
        merchant_connector_id: Option<Vec<id_type::MerchantConnectorAccountId>>,
        card_network: Option<Vec<common_enums::CardNetwork>>,
        card_discovery: Option<Vec<common_enums::CardDiscovery>>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<i64, errors::DataStorageError> {
        self.diesel_store
            .get_total_count_of_filtered_payment_attempts(
                merchant_id,
                active_attempt_ids,
                connector,
                payment_method,
                payment_method_type,
                authentication_type,
                merchant_connector_id,
                card_network,
                card_discovery,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        merchant_id: &id_type::MerchantId,
        payment_id: &id_type::PaymentId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::PaymentAttempt>, errors::DataStorageError> {
        self.diesel_store
            .find_attempts_by_merchant_id_payment_id(merchant_id, payment_id, storage_scheme)
            .await
    }
}

#[async_trait::async_trait]
impl PaymentIntentInterface for KafkaStore {
    async fn update_payment_intent(
        &self,
        state: &KeyManagerState,
        this: storage::PaymentIntent,
        payment_intent: storage::PaymentIntentUpdate,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentIntent, errors::DataStorageError> {
        let intent = self
            .diesel_store
            .update_payment_intent(
                state,
                this.clone(),
                payment_intent,
                key_store,
                storage_scheme,
            )
            .await?;

        if let Err(er) = self
            .kafka_producer
            .log_payment_intent(&intent, Some(this), self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to add analytics entry for Payment Intent {intent:?}", error_message=?er);
        };

        Ok(intent)
    }

    async fn insert_payment_intent(
        &self,
        state: &KeyManagerState,
        new: storage::PaymentIntent,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentIntent, errors::DataStorageError> {
        logger::debug!("Inserting PaymentIntent Via KafkaStore");
        let intent = self
            .diesel_store
            .insert_payment_intent(state, new, key_store, storage_scheme)
            .await?;

        if let Err(er) = self
            .kafka_producer
            .log_payment_intent(&intent, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to add analytics entry for Payment Intent {intent:?}", error_message=?er);
        };

        Ok(intent)
    }

    #[cfg(feature = "v1")]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        state: &KeyManagerState,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentIntent, errors::DataStorageError> {
        self.diesel_store
            .find_payment_intent_by_payment_id_merchant_id(
                state,
                payment_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_intent_by_id(
        &self,
        state: &KeyManagerState,
        payment_id: &id_type::GlobalPaymentId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentIntent, errors::DataStorageError> {
        self.diesel_store
            .find_payment_intent_by_id(state, payment_id, key_store, storage_scheme)
            .await
    }

    #[cfg(all(feature = "olap", feature = "v1"))]
    async fn filter_payment_intent_by_constraints(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        filters: &hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::PaymentIntent>, errors::DataStorageError> {
        self.diesel_store
            .filter_payment_intent_by_constraints(
                state,
                merchant_id,
                filters,
                key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "olap", feature = "v1"))]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        time_range: &common_utils::types::TimeRange,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::PaymentIntent>, errors::DataStorageError> {
        self.diesel_store
            .filter_payment_intents_by_time_range_constraints(
                state,
                merchant_id,
                time_range,
                key_store,
                storage_scheme,
            )
            .await
    }
    #[cfg(feature = "olap")]
    async fn get_intent_status_with_count(
        &self,
        merchant_id: &id_type::MerchantId,
        profile_id_list: Option<Vec<id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> error_stack::Result<Vec<(common_enums::IntentStatus, i64)>, errors::DataStorageError> {
        self.diesel_store
            .get_intent_status_with_count(merchant_id, profile_id_list, time_range)
            .await
    }

    #[cfg(all(feature = "olap", feature = "v1"))]
    async fn get_filtered_payment_intents_attempt(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        constraints: &hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<
        Vec<(
            hyperswitch_domain_models::payments::PaymentIntent,
            hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
        )>,
        errors::DataStorageError,
    > {
        self.diesel_store
            .get_filtered_payment_intents_attempt(
                state,
                merchant_id,
                constraints,
                key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "olap", feature = "v1"))]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &id_type::MerchantId,
        constraints: &hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<String>, errors::DataStorageError> {
        self.diesel_store
            .get_filtered_active_attempt_ids_for_total_count(
                merchant_id,
                constraints,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_intent_by_merchant_reference_id_profile_id(
        &self,
        state: &KeyManagerState,
        merchant_reference_id: &id_type::PaymentReferenceId,
        profile_id: &id_type::ProfileId,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: &MerchantStorageScheme,
    ) -> error_stack::Result<
        hyperswitch_domain_models::payments::PaymentIntent,
        errors::DataStorageError,
    > {
        self.diesel_store
            .find_payment_intent_by_merchant_reference_id_profile_id(
                state,
                merchant_reference_id,
                profile_id,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }
}

#[async_trait::async_trait]
impl PaymentMethodInterface for KafkaStore {
    #[cfg(all(
        any(feature = "v2", feature = "v1"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .find_payment_method(state, key_store, payment_method_id, storage_scheme)
            .await
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .find_payment_method(state, key_store, payment_method_id, storage_scheme)
            .await
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
        self.diesel_store
            .find_payment_method_by_customer_id_merchant_id_list(
                state,
                key_store,
                customer_id,
                merchant_id,
                limit,
            )
            .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_payment_method_list_by_global_customer_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        id: &id_type::GlobalCustomerId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
        self.diesel_store
            .find_payment_method_list_by_global_customer_id(state, key_store, id, limit)
            .await
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
        self.diesel_store
            .find_payment_method_by_customer_id_merchant_id_status(
                state,
                key_store,
                customer_id,
                merchant_id,
                status,
                limit,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_payment_method_by_global_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
        self.diesel_store
            .find_payment_method_by_global_customer_id_merchant_id_status(
                state,
                key_store,
                customer_id,
                merchant_id,
                status,
                limit,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError> {
        self.diesel_store
            .get_payment_method_count_by_customer_id_merchant_id_status(
                customer_id,
                merchant_id,
                status,
            )
            .await
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method_by_locker_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        locker_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .find_payment_method_by_locker_id(state, key_store, locker_id, storage_scheme)
            .await
    }

    async fn insert_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        m: domain::PaymentMethod,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .insert_payment_method(state, key_store, m, storage_scheme)
            .await
    }

    async fn update_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
        payment_method_update: storage::PaymentMethodUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .update_payment_method(
                state,
                key_store,
                payment_method,
                payment_method_update,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .delete_payment_method_by_merchant_id_payment_method_id(
                state,
                key_store,
                merchant_id,
                payment_method_id,
            )
            .await
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn delete_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .delete_payment_method(state, key_store, payment_method)
            .await
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn find_payment_method_by_fingerprint_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .find_payment_method_by_fingerprint_id(state, key_store, fingerprint_id)
            .await
    }
}

#[cfg(not(feature = "payouts"))]
impl PayoutAttemptInterface for KafkaStore {}

#[cfg(feature = "payouts")]
#[async_trait::async_trait]
impl PayoutAttemptInterface for KafkaStore {
    async fn find_payout_attempt_by_merchant_id_payout_attempt_id(
        &self,
        merchant_id: &id_type::MerchantId,
        payout_attempt_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PayoutAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payout_attempt_by_merchant_id_payout_attempt_id(
                merchant_id,
                payout_attempt_id,
                storage_scheme,
            )
            .await
    }

    async fn find_payout_attempt_by_merchant_id_connector_payout_id(
        &self,
        merchant_id: &id_type::MerchantId,
        connector_payout_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PayoutAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payout_attempt_by_merchant_id_connector_payout_id(
                merchant_id,
                connector_payout_id,
                storage_scheme,
            )
            .await
    }

    async fn update_payout_attempt(
        &self,
        this: &storage::PayoutAttempt,
        payout_attempt_update: storage::PayoutAttemptUpdate,
        payouts: &storage::Payouts,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PayoutAttempt, errors::DataStorageError> {
        let updated_payout_attempt = self
            .diesel_store
            .update_payout_attempt(this, payout_attempt_update, payouts, storage_scheme)
            .await?;
        if let Err(err) = self
            .kafka_producer
            .log_payout(
                &KafkaPayout::from_storage(payouts, &updated_payout_attempt),
                Some(KafkaPayout::from_storage(payouts, this)),
                self.tenant_id.clone(),
            )
            .await
        {
            logger::error!(message="Failed to update analytics entry for Payouts {payouts:?}\n{updated_payout_attempt:?}", error_message=?err);
        };

        Ok(updated_payout_attempt)
    }

    async fn insert_payout_attempt(
        &self,
        payout_attempt: storage::PayoutAttemptNew,
        payouts: &storage::Payouts,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PayoutAttempt, errors::DataStorageError> {
        let payout_attempt_new = self
            .diesel_store
            .insert_payout_attempt(payout_attempt, payouts, storage_scheme)
            .await?;
        if let Err(err) = self
            .kafka_producer
            .log_payout(
                &KafkaPayout::from_storage(payouts, &payout_attempt_new),
                None,
                self.tenant_id.clone(),
            )
            .await
        {
            logger::error!(message="Failed to add analytics entry for Payouts {payouts:?}\n{payout_attempt_new:?}", error_message=?err);
        };

        Ok(payout_attempt_new)
    }

    async fn get_filters_for_payouts(
        &self,
        payouts: &[hyperswitch_domain_models::payouts::payouts::Payouts],
        merchant_id: &id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<
        hyperswitch_domain_models::payouts::payout_attempt::PayoutListFilters,
        errors::DataStorageError,
    > {
        self.diesel_store
            .get_filters_for_payouts(payouts, merchant_id, storage_scheme)
            .await
    }
}

#[cfg(not(feature = "payouts"))]
impl PayoutsInterface for KafkaStore {}

#[cfg(feature = "payouts")]
#[async_trait::async_trait]
impl PayoutsInterface for KafkaStore {
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &id_type::MerchantId,
        payout_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Payouts, errors::DataStorageError> {
        self.diesel_store
            .find_payout_by_merchant_id_payout_id(merchant_id, payout_id, storage_scheme)
            .await
    }

    async fn update_payout(
        &self,
        this: &storage::Payouts,
        payout_update: storage::PayoutsUpdate,
        payout_attempt: &storage::PayoutAttempt,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Payouts, errors::DataStorageError> {
        let payout = self
            .diesel_store
            .update_payout(this, payout_update, payout_attempt, storage_scheme)
            .await?;
        if let Err(err) = self
            .kafka_producer
            .log_payout(
                &KafkaPayout::from_storage(&payout, payout_attempt),
                Some(KafkaPayout::from_storage(this, payout_attempt)),
                self.tenant_id.clone(),
            )
            .await
        {
            logger::error!(message="Failed to update analytics entry for Payouts {payout:?}\n{payout_attempt:?}", error_message=?err);
        };
        Ok(payout)
    }

    async fn insert_payout(
        &self,
        payout: storage::PayoutsNew,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Payouts, errors::DataStorageError> {
        self.diesel_store
            .insert_payout(payout, storage_scheme)
            .await
    }

    async fn find_optional_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &id_type::MerchantId,
        payout_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<storage::Payouts>, errors::DataStorageError> {
        self.diesel_store
            .find_optional_payout_by_merchant_id_payout_id(merchant_id, payout_id, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn filter_payouts_by_constraints(
        &self,
        merchant_id: &id_type::MerchantId,
        filters: &hyperswitch_domain_models::payouts::PayoutFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::Payouts>, errors::DataStorageError> {
        self.diesel_store
            .filter_payouts_by_constraints(merchant_id, filters, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn filter_payouts_and_attempts(
        &self,
        merchant_id: &id_type::MerchantId,
        filters: &hyperswitch_domain_models::payouts::PayoutFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<
        Vec<(
            storage::Payouts,
            storage::PayoutAttempt,
            Option<diesel_models::Customer>,
            Option<diesel_models::Address>,
        )>,
        errors::DataStorageError,
    > {
        self.diesel_store
            .filter_payouts_and_attempts(merchant_id, filters, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn filter_payouts_by_time_range_constraints(
        &self,
        merchant_id: &id_type::MerchantId,
        time_range: &common_utils::types::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::Payouts>, errors::DataStorageError> {
        self.diesel_store
            .filter_payouts_by_time_range_constraints(merchant_id, time_range, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_total_count_of_filtered_payouts(
        &self,
        merchant_id: &id_type::MerchantId,
        active_payout_ids: &[String],
        connector: Option<Vec<api_models::enums::PayoutConnectors>>,
        currency: Option<Vec<enums::Currency>>,
        status: Option<Vec<enums::PayoutStatus>>,
        payout_method: Option<Vec<enums::PayoutType>>,
    ) -> CustomResult<i64, errors::DataStorageError> {
        self.diesel_store
            .get_total_count_of_filtered_payouts(
                merchant_id,
                active_payout_ids,
                connector,
                currency,
                status,
                payout_method,
            )
            .await
    }

    #[cfg(feature = "olap")]
    async fn filter_active_payout_ids_by_constraints(
        &self,
        merchant_id: &id_type::MerchantId,
        constraints: &hyperswitch_domain_models::payouts::PayoutFetchConstraints,
    ) -> CustomResult<Vec<String>, errors::DataStorageError> {
        self.diesel_store
            .filter_active_payout_ids_by_constraints(merchant_id, constraints)
            .await
    }
}

#[async_trait::async_trait]
impl ProcessTrackerInterface for KafkaStore {
    async fn reinitialize_limbo_processes(
        &self,
        ids: Vec<String>,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .reinitialize_limbo_processes(ids, schedule_time)
            .await
    }

    async fn find_process_by_id(
        &self,
        id: &str,
    ) -> CustomResult<Option<storage::ProcessTracker>, errors::StorageError> {
        self.diesel_store.find_process_by_id(id).await
    }

    async fn update_process(
        &self,
        this: storage::ProcessTracker,
        process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        self.diesel_store.update_process(this, process).await
    }

    async fn process_tracker_update_process_status_by_ids(
        &self,
        task_ids: Vec<String>,
        task_update: storage::ProcessTrackerUpdate,
    ) -> CustomResult<usize, errors::StorageError> {
        self.diesel_store
            .process_tracker_update_process_status_by_ids(task_ids, task_update)
            .await
    }

    async fn insert_process(
        &self,
        new: storage::ProcessTrackerNew,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        self.diesel_store.insert_process(new).await
    }

    async fn reset_process(
        &self,
        this: storage::ProcessTracker,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), errors::StorageError> {
        self.diesel_store.reset_process(this, schedule_time).await
    }

    async fn retry_process(
        &self,
        this: storage::ProcessTracker,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), errors::StorageError> {
        self.diesel_store.retry_process(this, schedule_time).await
    }

    async fn finish_process_with_business_status(
        &self,
        this: storage::ProcessTracker,
        business_status: &'static str,
    ) -> CustomResult<(), errors::StorageError> {
        self.diesel_store
            .finish_process_with_business_status(this, business_status)
            .await
    }

    async fn find_processes_by_time_status(
        &self,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        status: ProcessTrackerStatus,
        limit: Option<i64>,
    ) -> CustomResult<Vec<storage::ProcessTracker>, errors::StorageError> {
        self.diesel_store
            .find_processes_by_time_status(time_lower_limit, time_upper_limit, status, limit)
            .await
    }
}

#[async_trait::async_trait]
impl CaptureInterface for KafkaStore {
    async fn insert_capture(
        &self,
        capture: storage::CaptureNew,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Capture, errors::StorageError> {
        self.diesel_store
            .insert_capture(capture, storage_scheme)
            .await
    }

    async fn update_capture_with_capture_id(
        &self,
        this: storage::Capture,
        capture: storage::CaptureUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Capture, errors::StorageError> {
        self.diesel_store
            .update_capture_with_capture_id(this, capture, storage_scheme)
            .await
    }

    async fn find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
        &self,
        merchant_id: &id_type::MerchantId,
        payment_id: &id_type::PaymentId,
        authorized_attempt_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::Capture>, errors::StorageError> {
        self.diesel_store
            .find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
                merchant_id,
                payment_id,
                authorized_attempt_id,
                storage_scheme,
            )
            .await
    }
}

#[async_trait::async_trait]
impl RefundInterface for KafkaStore {
    async fn find_refund_by_internal_reference_id_merchant_id(
        &self,
        internal_reference_id: &str,
        merchant_id: &id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        self.diesel_store
            .find_refund_by_internal_reference_id_merchant_id(
                internal_reference_id,
                merchant_id,
                storage_scheme,
            )
            .await
    }

    async fn find_refund_by_payment_id_merchant_id(
        &self,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::Refund>, errors::StorageError> {
        self.diesel_store
            .find_refund_by_payment_id_merchant_id(payment_id, merchant_id, storage_scheme)
            .await
    }

    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &id_type::MerchantId,
        refund_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        self.diesel_store
            .find_refund_by_merchant_id_refund_id(merchant_id, refund_id, storage_scheme)
            .await
    }

    async fn find_refund_by_merchant_id_connector_refund_id_connector(
        &self,
        merchant_id: &id_type::MerchantId,
        connector_refund_id: &str,
        connector: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        self.diesel_store
            .find_refund_by_merchant_id_connector_refund_id_connector(
                merchant_id,
                connector_refund_id,
                connector,
                storage_scheme,
            )
            .await
    }

    async fn update_refund(
        &self,
        this: storage::Refund,
        refund: storage::RefundUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        let refund = self
            .diesel_store
            .update_refund(this.clone(), refund, storage_scheme)
            .await?;

        if let Err(er) = self
            .kafka_producer
            .log_refund(&refund, Some(this), self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to insert analytics event for Refund Update {refund?}", error_message=?er);
        }
        Ok(refund)
    }

    async fn find_refund_by_merchant_id_connector_transaction_id(
        &self,
        merchant_id: &id_type::MerchantId,
        connector_transaction_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::Refund>, errors::StorageError> {
        self.diesel_store
            .find_refund_by_merchant_id_connector_transaction_id(
                merchant_id,
                connector_transaction_id,
                storage_scheme,
            )
            .await
    }

    async fn insert_refund(
        &self,
        new: storage::RefundNew,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        let refund = self.diesel_store.insert_refund(new, storage_scheme).await?;

        if let Err(er) = self
            .kafka_producer
            .log_refund(&refund, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to insert analytics event for Refund Create {refund?}", error_message=?er);
        }
        Ok(refund)
    }

    #[cfg(feature = "olap")]
    async fn filter_refund_by_constraints(
        &self,
        merchant_id: &id_type::MerchantId,
        refund_details: &refunds::RefundListConstraints,
        storage_scheme: MerchantStorageScheme,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::Refund>, errors::StorageError> {
        self.diesel_store
            .filter_refund_by_constraints(
                merchant_id,
                refund_details,
                storage_scheme,
                limit,
                offset,
            )
            .await
    }

    #[cfg(feature = "olap")]
    async fn filter_refund_by_meta_constraints(
        &self,
        merchant_id: &id_type::MerchantId,
        refund_details: &common_utils::types::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::StorageError> {
        self.diesel_store
            .filter_refund_by_meta_constraints(merchant_id, refund_details, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_refund_status_with_count(
        &self,
        merchant_id: &id_type::MerchantId,
        profile_id_list: Option<Vec<id_type::ProfileId>>,
        constraints: &common_utils::types::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<(common_enums::RefundStatus, i64)>, errors::StorageError> {
        self.diesel_store
            .get_refund_status_with_count(merchant_id, profile_id_list, constraints, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_total_count_of_refunds(
        &self,
        merchant_id: &id_type::MerchantId,
        refund_details: &refunds::RefundListConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<i64, errors::StorageError> {
        self.diesel_store
            .get_total_count_of_refunds(merchant_id, refund_details, storage_scheme)
            .await
    }
}

#[async_trait::async_trait]
impl MerchantKeyStoreInterface for KafkaStore {
    async fn insert_merchant_key_store(
        &self,
        state: &KeyManagerState,
        merchant_key_store: domain::MerchantKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError> {
        self.diesel_store
            .insert_merchant_key_store(state, merchant_key_store, key)
            .await
    }

    async fn get_merchant_key_store_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError> {
        self.diesel_store
            .get_merchant_key_store_by_merchant_id(state, merchant_id, key)
            .await
    }

    async fn delete_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_merchant_key_store_by_merchant_id(merchant_id)
            .await
    }

    #[cfg(feature = "olap")]
    async fn list_multiple_key_stores(
        &self,
        state: &KeyManagerState,
        merchant_ids: Vec<id_type::MerchantId>,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, errors::StorageError> {
        self.diesel_store
            .list_multiple_key_stores(state, merchant_ids, key)
            .await
    }
    async fn get_all_key_stores(
        &self,
        state: &KeyManagerState,
        key: &Secret<Vec<u8>>,
        from: u32,
        to: u32,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, errors::StorageError> {
        self.diesel_store
            .get_all_key_stores(state, key, from, to)
            .await
    }
}

#[async_trait::async_trait]
impl ProfileInterface for KafkaStore {
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        self.diesel_store
            .insert_business_profile(key_manager_state, merchant_key_store, business_profile)
            .await
    }

    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_id: &id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        self.diesel_store
            .find_business_profile_by_profile_id(key_manager_state, merchant_key_store, profile_id)
            .await
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        self.diesel_store
            .find_business_profile_by_merchant_id_profile_id(
                key_manager_state,
                merchant_key_store,
                merchant_id,
                profile_id,
            )
            .await
    }

    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: domain::Profile,
        business_profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        self.diesel_store
            .update_profile_by_profile_id(
                key_manager_state,
                merchant_key_store,
                current_state,
                business_profile_update,
            )
            .await
    }

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &id_type::ProfileId,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_profile_by_profile_id_merchant_id(profile_id, merchant_id)
            .await
    }

    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, errors::StorageError> {
        self.diesel_store
            .list_profile_by_merchant_id(key_manager_state, merchant_key_store, merchant_id)
            .await
    }

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_name: &str,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        self.diesel_store
            .find_business_profile_by_profile_name_merchant_id(
                key_manager_state,
                merchant_key_store,
                profile_name,
                merchant_id,
            )
            .await
    }
}

#[async_trait::async_trait]
impl ReverseLookupInterface for KafkaStore {
    async fn insert_reverse_lookup(
        &self,
        new: ReverseLookupNew,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        self.diesel_store
            .insert_reverse_lookup(new, storage_scheme)
            .await
    }

    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        self.diesel_store
            .get_lookup_by_lookup_id(id, storage_scheme)
            .await
    }
}

#[async_trait::async_trait]
impl RoutingAlgorithmInterface for KafkaStore {
    async fn insert_routing_algorithm(
        &self,
        routing_algorithm: storage::RoutingAlgorithm,
    ) -> CustomResult<storage::RoutingAlgorithm, errors::StorageError> {
        self.diesel_store
            .insert_routing_algorithm(routing_algorithm)
            .await
    }

    async fn find_routing_algorithm_by_profile_id_algorithm_id(
        &self,
        profile_id: &id_type::ProfileId,
        algorithm_id: &id_type::RoutingId,
    ) -> CustomResult<storage::RoutingAlgorithm, errors::StorageError> {
        self.diesel_store
            .find_routing_algorithm_by_profile_id_algorithm_id(profile_id, algorithm_id)
            .await
    }

    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        algorithm_id: &id_type::RoutingId,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<storage::RoutingAlgorithm, errors::StorageError> {
        self.diesel_store
            .find_routing_algorithm_by_algorithm_id_merchant_id(algorithm_id, merchant_id)
            .await
    }

    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        algorithm_id: &id_type::RoutingId,
        profile_id: &id_type::ProfileId,
    ) -> CustomResult<storage::RoutingProfileMetadata, errors::StorageError> {
        self.diesel_store
            .find_routing_algorithm_metadata_by_algorithm_id_profile_id(algorithm_id, profile_id)
            .await
    }

    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        profile_id: &id_type::ProfileId,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::RoutingProfileMetadata>, errors::StorageError> {
        self.diesel_store
            .list_routing_algorithm_metadata_by_profile_id(profile_id, limit, offset)
            .await
    }

    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        merchant_id: &id_type::MerchantId,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::RoutingProfileMetadata>, errors::StorageError> {
        self.diesel_store
            .list_routing_algorithm_metadata_by_merchant_id(merchant_id, limit, offset)
            .await
    }

    async fn list_routing_algorithm_metadata_by_merchant_id_transaction_type(
        &self,
        merchant_id: &id_type::MerchantId,
        transaction_type: &enums::TransactionType,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::RoutingProfileMetadata>, errors::StorageError> {
        self.diesel_store
            .list_routing_algorithm_metadata_by_merchant_id_transaction_type(
                merchant_id,
                transaction_type,
                limit,
                offset,
            )
            .await
    }
}

#[async_trait::async_trait]
impl GsmInterface for KafkaStore {
    async fn add_gsm_rule(
        &self,
        rule: storage::GatewayStatusMappingNew,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        self.diesel_store.add_gsm_rule(rule).await
    }

    async fn find_gsm_decision(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<String, errors::StorageError> {
        self.diesel_store
            .find_gsm_decision(connector, flow, sub_flow, code, message)
            .await
    }

    async fn find_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        self.diesel_store
            .find_gsm_rule(connector, flow, sub_flow, code, message)
            .await
    }

    async fn update_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
        data: storage::GatewayStatusMappingUpdate,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        self.diesel_store
            .update_gsm_rule(connector, flow, sub_flow, code, message, data)
            .await
    }

    async fn delete_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_gsm_rule(connector, flow, sub_flow, code, message)
            .await
    }
}

#[async_trait::async_trait]
impl UnifiedTranslationsInterface for KafkaStore {
    async fn add_unfied_translation(
        &self,
        translation: storage::UnifiedTranslationsNew,
    ) -> CustomResult<storage::UnifiedTranslations, errors::StorageError> {
        self.diesel_store.add_unfied_translation(translation).await
    }

    async fn find_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> CustomResult<String, errors::StorageError> {
        self.diesel_store
            .find_translation(unified_code, unified_message, locale)
            .await
    }

    async fn update_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
        data: storage::UnifiedTranslationsUpdate,
    ) -> CustomResult<storage::UnifiedTranslations, errors::StorageError> {
        self.diesel_store
            .update_translation(unified_code, unified_message, locale, data)
            .await
    }

    async fn delete_translation(
        &self,
        unified_code: String,
        unified_message: String,
        locale: String,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_translation(unified_code, unified_message, locale)
            .await
    }
}

#[async_trait::async_trait]
impl StorageInterface for KafkaStore {
    fn get_scheduler_db(&self) -> Box<dyn SchedulerInterface> {
        Box::new(self.clone())
    }

    fn get_cache_store(&self) -> Box<(dyn RedisConnInterface + Send + Sync + 'static)> {
        Box::new(self.clone())
    }
}

impl GlobalStorageInterface for KafkaStore {}
impl AccountsStorageInterface for KafkaStore {}

impl CommonStorageInterface for KafkaStore {
    fn get_storage_interface(&self) -> Box<dyn StorageInterface> {
        Box::new(self.clone())
    }
    fn get_global_storage_interface(&self) -> Box<dyn GlobalStorageInterface> {
        Box::new(self.clone())
    }
    fn get_accounts_storage_interface(&self) -> Box<dyn AccountsStorageInterface> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
impl SchedulerInterface for KafkaStore {}

impl MasterKeyInterface for KafkaStore {
    fn get_master_key(&self) -> &[u8] {
        self.diesel_store.get_master_key()
    }
}
#[async_trait::async_trait]
impl UserInterface for KafkaStore {
    async fn insert_user(
        &self,
        user_data: storage::UserNew,
    ) -> CustomResult<storage::User, errors::StorageError> {
        self.diesel_store.insert_user(user_data).await
    }

    async fn find_user_by_email(
        &self,
        user_email: &domain::UserEmail,
    ) -> CustomResult<storage::User, errors::StorageError> {
        self.diesel_store.find_user_by_email(user_email).await
    }

    async fn find_user_by_id(
        &self,
        user_id: &str,
    ) -> CustomResult<storage::User, errors::StorageError> {
        self.diesel_store.find_user_by_id(user_id).await
    }

    async fn update_user_by_user_id(
        &self,
        user_id: &str,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        self.diesel_store
            .update_user_by_user_id(user_id, user)
            .await
    }

    async fn update_user_by_email(
        &self,
        user_email: &domain::UserEmail,
        user: storage::UserUpdate,
    ) -> CustomResult<storage::User, errors::StorageError> {
        self.diesel_store
            .update_user_by_email(user_email, user)
            .await
    }

    async fn delete_user_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store.delete_user_by_user_id(user_id).await
    }

    async fn find_users_by_user_ids(
        &self,
        user_ids: Vec<String>,
    ) -> CustomResult<Vec<storage::User>, errors::StorageError> {
        self.diesel_store.find_users_by_user_ids(user_ids).await
    }
}

impl RedisConnInterface for KafkaStore {
    fn get_redis_conn(&self) -> CustomResult<Arc<RedisConnectionPool>, RedisError> {
        self.diesel_store.get_redis_conn()
    }
}

#[async_trait::async_trait]
impl UserRoleInterface for KafkaStore {
    async fn insert_user_role(
        &self,
        user_role: storage::UserRoleNew,
    ) -> CustomResult<user_storage::UserRole, errors::StorageError> {
        self.diesel_store.insert_user_role(user_role).await
    }

    async fn find_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        self.diesel_store
            .find_user_role_by_user_id_and_lineage(
                user_id,
                tenant_id,
                org_id,
                merchant_id,
                profile_id,
                version,
            )
            .await
    }

    async fn update_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: Option<&id_type::MerchantId>,
        profile_id: Option<&id_type::ProfileId>,
        update: user_storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        self.diesel_store
            .update_user_role_by_user_id_and_lineage(
                user_id,
                tenant_id,
                org_id,
                merchant_id,
                profile_id,
                update,
                version,
            )
            .await
    }

    async fn delete_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, errors::StorageError> {
        self.diesel_store
            .delete_user_role_by_user_id_and_lineage(
                user_id,
                tenant_id,
                org_id,
                merchant_id,
                profile_id,
                version,
            )
            .await
    }

    async fn list_user_roles_by_user_id<'a>(
        &self,
        payload: ListUserRolesByUserIdPayload<'a>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        self.diesel_store.list_user_roles_by_user_id(payload).await
    }

    async fn list_user_roles_by_user_id_across_tenants(
        &self,
        user_id: &str,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::UserRole>, errors::StorageError> {
        self.diesel_store
            .list_user_roles_by_user_id_across_tenants(user_id, limit)
            .await
    }

    async fn list_user_roles_by_org_id<'a>(
        &self,
        payload: ListUserRolesByOrgIdPayload<'a>,
    ) -> CustomResult<Vec<user_storage::UserRole>, errors::StorageError> {
        self.diesel_store.list_user_roles_by_org_id(payload).await
    }
}

#[async_trait::async_trait]
impl DashboardMetadataInterface for KafkaStore {
    async fn insert_metadata(
        &self,
        metadata: storage::DashboardMetadataNew,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError> {
        self.diesel_store.insert_metadata(metadata).await
    }

    async fn update_metadata(
        &self,
        user_id: Option<String>,
        merchant_id: id_type::MerchantId,
        org_id: id_type::OrganizationId,
        data_key: enums::DashboardMetadata,
        dashboard_metadata_update: storage::DashboardMetadataUpdate,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError> {
        self.diesel_store
            .update_metadata(
                user_id,
                merchant_id,
                org_id,
                data_key,
                dashboard_metadata_update,
            )
            .await
    }

    async fn find_user_scoped_dashboard_metadata(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError> {
        self.diesel_store
            .find_user_scoped_dashboard_metadata(user_id, merchant_id, org_id, data_keys)
            .await
    }

    async fn find_merchant_scoped_dashboard_metadata(
        &self,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError> {
        self.diesel_store
            .find_merchant_scoped_dashboard_metadata(merchant_id, org_id, data_keys)
            .await
    }

    async fn delete_all_user_scoped_dashboard_metadata_by_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_all_user_scoped_dashboard_metadata_by_merchant_id(user_id, merchant_id)
            .await
    }

    async fn delete_user_scoped_dashboard_metadata_by_merchant_id_data_key(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        data_key: enums::DashboardMetadata,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError> {
        self.diesel_store
            .delete_user_scoped_dashboard_metadata_by_merchant_id_data_key(
                user_id,
                merchant_id,
                data_key,
            )
            .await
    }
}

#[async_trait::async_trait]
impl BatchSampleDataInterface for KafkaStore {
    #[cfg(feature = "v1")]
    async fn insert_payment_intents_batch_for_sample_data(
        &self,
        state: &KeyManagerState,
        batch: Vec<hyperswitch_domain_models::payments::PaymentIntent>,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<
        Vec<hyperswitch_domain_models::payments::PaymentIntent>,
        hyperswitch_domain_models::errors::StorageError,
    > {
        let payment_intents_list = self
            .diesel_store
            .insert_payment_intents_batch_for_sample_data(state, batch, key_store)
            .await?;

        for payment_intent in payment_intents_list.iter() {
            let _ = self
                .kafka_producer
                .log_payment_intent(payment_intent, None, self.tenant_id.clone())
                .await;
        }
        Ok(payment_intents_list)
    }

    #[cfg(feature = "v1")]
    async fn insert_payment_attempts_batch_for_sample_data(
        &self,
        batch: Vec<diesel_models::user::sample_data::PaymentAttemptBatchNew>,
    ) -> CustomResult<
        Vec<hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt>,
        hyperswitch_domain_models::errors::StorageError,
    > {
        let payment_attempts_list = self
            .diesel_store
            .insert_payment_attempts_batch_for_sample_data(batch)
            .await?;

        for payment_attempt in payment_attempts_list.iter() {
            let _ = self
                .kafka_producer
                .log_payment_attempt(payment_attempt, None, self.tenant_id.clone())
                .await;
        }
        Ok(payment_attempts_list)
    }

    #[cfg(feature = "v1")]
    async fn insert_refunds_batch_for_sample_data(
        &self,
        batch: Vec<diesel_models::RefundNew>,
    ) -> CustomResult<Vec<diesel_models::Refund>, hyperswitch_domain_models::errors::StorageError>
    {
        let refunds_list = self
            .diesel_store
            .insert_refunds_batch_for_sample_data(batch)
            .await?;

        for refund in refunds_list.iter() {
            let _ = self
                .kafka_producer
                .log_refund(refund, None, self.tenant_id.clone())
                .await;
        }
        Ok(refunds_list)
    }

    #[cfg(feature = "v1")]
    async fn insert_disputes_batch_for_sample_data(
        &self,
        batch: Vec<diesel_models::DisputeNew>,
    ) -> CustomResult<Vec<diesel_models::Dispute>, hyperswitch_domain_models::errors::StorageError>
    {
        let disputes_list = self
            .diesel_store
            .insert_disputes_batch_for_sample_data(batch)
            .await?;

        for dispute in disputes_list.iter() {
            let _ = self
                .kafka_producer
                .log_dispute(dispute, None, self.tenant_id.clone())
                .await;
        }
        Ok(disputes_list)
    }

    #[cfg(feature = "v1")]
    async fn delete_payment_intents_for_sample_data(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<
        Vec<hyperswitch_domain_models::payments::PaymentIntent>,
        hyperswitch_domain_models::errors::StorageError,
    > {
        let payment_intents_list = self
            .diesel_store
            .delete_payment_intents_for_sample_data(state, merchant_id, key_store)
            .await?;

        for payment_intent in payment_intents_list.iter() {
            let _ = self
                .kafka_producer
                .log_payment_intent_delete(payment_intent, self.tenant_id.clone())
                .await;
        }
        Ok(payment_intents_list)
    }

    #[cfg(feature = "v1")]
    async fn delete_payment_attempts_for_sample_data(
        &self,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<
        Vec<hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt>,
        hyperswitch_domain_models::errors::StorageError,
    > {
        let payment_attempts_list = self
            .diesel_store
            .delete_payment_attempts_for_sample_data(merchant_id)
            .await?;

        for payment_attempt in payment_attempts_list.iter() {
            let _ = self
                .kafka_producer
                .log_payment_attempt_delete(payment_attempt, self.tenant_id.clone())
                .await;
        }

        Ok(payment_attempts_list)
    }

    #[cfg(feature = "v1")]
    async fn delete_refunds_for_sample_data(
        &self,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<Vec<diesel_models::Refund>, hyperswitch_domain_models::errors::StorageError>
    {
        let refunds_list = self
            .diesel_store
            .delete_refunds_for_sample_data(merchant_id)
            .await?;

        for refund in refunds_list.iter() {
            let _ = self
                .kafka_producer
                .log_refund_delete(refund, self.tenant_id.clone())
                .await;
        }

        Ok(refunds_list)
    }

    #[cfg(feature = "v1")]
    async fn delete_disputes_for_sample_data(
        &self,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<Vec<diesel_models::Dispute>, hyperswitch_domain_models::errors::StorageError>
    {
        let disputes_list = self
            .diesel_store
            .delete_disputes_for_sample_data(merchant_id)
            .await?;

        for dispute in disputes_list.iter() {
            let _ = self
                .kafka_producer
                .log_dispute_delete(dispute, self.tenant_id.clone())
                .await;
        }

        Ok(disputes_list)
    }
}

#[async_trait::async_trait]
impl AuthorizationInterface for KafkaStore {
    async fn insert_authorization(
        &self,
        authorization: storage::AuthorizationNew,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        self.diesel_store.insert_authorization(authorization).await
    }

    async fn find_all_authorizations_by_merchant_id_payment_id(
        &self,
        merchant_id: &id_type::MerchantId,
        payment_id: &id_type::PaymentId,
    ) -> CustomResult<Vec<storage::Authorization>, errors::StorageError> {
        self.diesel_store
            .find_all_authorizations_by_merchant_id_payment_id(merchant_id, payment_id)
            .await
    }

    async fn update_authorization_by_merchant_id_authorization_id(
        &self,
        merchant_id: id_type::MerchantId,
        authorization_id: String,
        authorization: storage::AuthorizationUpdate,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        self.diesel_store
            .update_authorization_by_merchant_id_authorization_id(
                merchant_id,
                authorization_id,
                authorization,
            )
            .await
    }
}

#[async_trait::async_trait]
impl AuthenticationInterface for KafkaStore {
    async fn insert_authentication(
        &self,
        authentication: storage::AuthenticationNew,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let auth = self
            .diesel_store
            .insert_authentication(authentication)
            .await?;

        if let Err(er) = self
            .kafka_producer
            .log_authentication(&auth, None, self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to log analytics event for authentication {auth:?}", error_message=?er)
        }

        Ok(auth)
    }

    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: &id_type::MerchantId,
        authentication_id: String,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        self.diesel_store
            .find_authentication_by_merchant_id_authentication_id(merchant_id, authentication_id)
            .await
    }

    async fn find_authentication_by_merchant_id_connector_authentication_id(
        &self,
        merchant_id: id_type::MerchantId,
        connector_authentication_id: String,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        self.diesel_store
            .find_authentication_by_merchant_id_connector_authentication_id(
                merchant_id,
                connector_authentication_id,
            )
            .await
    }

    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: storage::Authentication,
        authentication_update: storage::AuthenticationUpdate,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let auth = self
            .diesel_store
            .update_authentication_by_merchant_id_authentication_id(
                previous_state.clone(),
                authentication_update,
            )
            .await?;

        if let Err(er) = self
            .kafka_producer
            .log_authentication(&auth, Some(previous_state.clone()), self.tenant_id.clone())
            .await
        {
            logger::error!(message="Failed to log analytics event for authentication {auth:?}", error_message=?er)
        }

        Ok(auth)
    }
}

#[async_trait::async_trait]
impl HealthCheckDbInterface for KafkaStore {
    async fn health_check_db(&self) -> CustomResult<(), errors::HealthCheckDBError> {
        self.diesel_store.health_check_db().await
    }
}

#[async_trait::async_trait]
impl RoleInterface for KafkaStore {
    async fn insert_role(
        &self,
        role: storage::RoleNew,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        self.diesel_store.insert_role(role).await
    }

    async fn find_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        self.diesel_store.find_role_by_role_id(role_id).await
    }

    async fn find_role_by_role_id_in_lineage(
        &self,
        role_id: &str,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
        profile_id: &id_type::ProfileId,
        tenant_id: &id_type::TenantId,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        self.diesel_store
            .find_role_by_role_id_in_lineage(role_id, merchant_id, org_id, profile_id, tenant_id)
            .await
    }

    async fn find_by_role_id_org_id_tenant_id(
        &self,
        role_id: &str,
        org_id: &id_type::OrganizationId,
        tenant_id: &id_type::TenantId,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        self.diesel_store
            .find_by_role_id_org_id_tenant_id(role_id, org_id, tenant_id)
            .await
    }

    async fn update_role_by_role_id(
        &self,
        role_id: &str,
        role_update: storage::RoleUpdate,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        self.diesel_store
            .update_role_by_role_id(role_id, role_update)
            .await
    }

    async fn delete_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        self.diesel_store.delete_role_by_role_id(role_id).await
    }

    //TODO: Remove once generic_list_roles_by_entity_type is stable
    async fn list_roles_for_org_by_parameters(
        &self,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: Option<&id_type::MerchantId>,
        entity_type: Option<enums::EntityType>,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        self.diesel_store
            .list_roles_for_org_by_parameters(tenant_id, org_id, merchant_id, entity_type, limit)
            .await
    }

    async fn generic_list_roles_by_entity_type(
        &self,
        payload: diesel_models::role::ListRolesByEntityPayload,
        is_lineage_data_required: bool,
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        self.diesel_store
            .generic_list_roles_by_entity_type(payload, is_lineage_data_required, tenant_id, org_id)
            .await
    }
}

#[async_trait::async_trait]
impl GenericLinkInterface for KafkaStore {
    async fn find_generic_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::GenericLinkState, errors::StorageError> {
        self.diesel_store
            .find_generic_link_by_link_id(link_id)
            .await
    }

    async fn find_pm_collect_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::PaymentMethodCollectLink, errors::StorageError> {
        self.diesel_store
            .find_pm_collect_link_by_link_id(link_id)
            .await
    }

    async fn find_payout_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
        self.diesel_store.find_payout_link_by_link_id(link_id).await
    }

    async fn insert_generic_link(
        &self,
        generic_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::GenericLinkState, errors::StorageError> {
        self.diesel_store.insert_generic_link(generic_link).await
    }

    async fn insert_pm_collect_link(
        &self,
        pm_collect_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::PaymentMethodCollectLink, errors::StorageError> {
        self.diesel_store
            .insert_pm_collect_link(pm_collect_link)
            .await
    }

    async fn insert_payout_link(
        &self,
        pm_collect_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
        self.diesel_store.insert_payout_link(pm_collect_link).await
    }

    async fn update_payout_link(
        &self,
        payout_link: storage::PayoutLink,
        payout_link_update: storage::PayoutLinkUpdate,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
        self.diesel_store
            .update_payout_link(payout_link, payout_link_update)
            .await
    }
}

#[async_trait::async_trait]
impl UserKeyStoreInterface for KafkaStore {
    async fn insert_user_key_store(
        &self,
        state: &KeyManagerState,
        user_key_store: domain::UserKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, errors::StorageError> {
        self.diesel_store
            .insert_user_key_store(state, user_key_store, key)
            .await
    }

    async fn get_user_key_store_by_user_id(
        &self,
        state: &KeyManagerState,
        user_id: &str,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, errors::StorageError> {
        self.diesel_store
            .get_user_key_store_by_user_id(state, user_id, key)
            .await
    }

    async fn get_all_user_key_store(
        &self,
        state: &KeyManagerState,
        key: &Secret<Vec<u8>>,
        from: u32,
        limit: u32,
    ) -> CustomResult<Vec<domain::UserKeyStore>, errors::StorageError> {
        self.diesel_store
            .get_all_user_key_store(state, key, from, limit)
            .await
    }
}

#[async_trait::async_trait]
impl UserAuthenticationMethodInterface for KafkaStore {
    async fn insert_user_authentication_method(
        &self,
        user_authentication_method: storage::UserAuthenticationMethodNew,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError> {
        self.diesel_store
            .insert_user_authentication_method(user_authentication_method)
            .await
    }

    async fn get_user_authentication_method_by_id(
        &self,
        id: &str,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError> {
        self.diesel_store
            .get_user_authentication_method_by_id(id)
            .await
    }

    async fn list_user_authentication_methods_for_auth_id(
        &self,
        auth_id: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError> {
        self.diesel_store
            .list_user_authentication_methods_for_auth_id(auth_id)
            .await
    }

    async fn list_user_authentication_methods_for_owner_id(
        &self,
        owner_id: &str,
    ) -> CustomResult<Vec<storage::UserAuthenticationMethod>, errors::StorageError> {
        self.diesel_store
            .list_user_authentication_methods_for_owner_id(owner_id)
            .await
    }

    async fn update_user_authentication_method(
        &self,
        id: &str,
        user_authentication_method_update: storage::UserAuthenticationMethodUpdate,
    ) -> CustomResult<storage::UserAuthenticationMethod, errors::StorageError> {
        self.diesel_store
            .update_user_authentication_method(id, user_authentication_method_update)
            .await
    }

    async fn list_user_authentication_methods_for_email_domain(
        &self,
        email_domain: &str,
    ) -> CustomResult<
        Vec<diesel_models::user_authentication_method::UserAuthenticationMethod>,
        errors::StorageError,
    > {
        self.diesel_store
            .list_user_authentication_methods_for_email_domain(email_domain)
            .await
    }
}

#[async_trait::async_trait]
impl ThemeInterface for KafkaStore {
    async fn insert_theme(
        &self,
        theme: storage::theme::ThemeNew,
    ) -> CustomResult<storage::theme::Theme, errors::StorageError> {
        self.diesel_store.insert_theme(theme).await
    }

    async fn find_theme_by_theme_id(
        &self,
        theme_id: String,
    ) -> CustomResult<storage::theme::Theme, errors::StorageError> {
        self.diesel_store.find_theme_by_theme_id(theme_id).await
    }

    async fn find_most_specific_theme_in_lineage(
        &self,
        lineage: ThemeLineage,
    ) -> CustomResult<diesel_models::user::theme::Theme, errors::StorageError> {
        self.diesel_store
            .find_most_specific_theme_in_lineage(lineage)
            .await
    }

    async fn find_theme_by_lineage(
        &self,
        lineage: ThemeLineage,
    ) -> CustomResult<storage::theme::Theme, errors::StorageError> {
        self.diesel_store.find_theme_by_lineage(lineage).await
    }

    async fn delete_theme_by_lineage_and_theme_id(
        &self,
        theme_id: String,
        lineage: ThemeLineage,
    ) -> CustomResult<storage::theme::Theme, errors::StorageError> {
        self.diesel_store
            .delete_theme_by_lineage_and_theme_id(theme_id, lineage)
            .await
    }
}

#[async_trait::async_trait]
#[cfg(feature = "v2")]
impl db::payment_method_session::PaymentMethodsSessionInterface for KafkaStore {
    async fn insert_payment_methods_session(
        &self,
        state: &KeyManagerState,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        payment_methods_session: hyperswitch_domain_models::payment_methods::PaymentMethodsSession,
        validity: i64,
    ) -> CustomResult<(), errors::StorageError> {
        self.diesel_store
            .insert_payment_methods_session(state, key_store, payment_methods_session, validity)
            .await
    }

    async fn get_payment_methods_session(
        &self,
        state: &KeyManagerState,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        id: &id_type::GlobalPaymentMethodSessionId,
    ) -> CustomResult<
        hyperswitch_domain_models::payment_methods::PaymentMethodsSession,
        errors::StorageError,
    > {
        self.diesel_store
            .get_payment_methods_session(state, key_store, id)
            .await
    }
}

#[async_trait::async_trait]
#[cfg(feature = "v1")]
impl db::payment_method_session::PaymentMethodsSessionInterface for KafkaStore {}

#[async_trait::async_trait]
impl CallbackMapperInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: domain::CallbackMapper,
    ) -> CustomResult<domain::CallbackMapper, errors::StorageError> {
        self.diesel_store
            .insert_call_back_mapper(call_back_mapper)
            .await
    }

    #[instrument(skip_all)]
    async fn find_call_back_mapper_by_id(
        &self,
        id: &str,
    ) -> CustomResult<domain::CallbackMapper, errors::StorageError> {
        self.diesel_store.find_call_back_mapper_by_id(id).await
    }
}
