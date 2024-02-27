use std::sync::Arc;

use common_enums::enums::MerchantStorageScheme;
use common_utils::errors::CustomResult;
use data_models::payments::{
    payment_attempt::PaymentAttemptInterface, payment_intent::PaymentIntentInterface,
};
use diesel_models::{
    enums,
    enums::ProcessTrackerStatus,
    ephemeral_key::{EphemeralKey, EphemeralKeyNew},
    reverse_lookup::{ReverseLookup, ReverseLookupNew},
    user_role as user_storage,
};
use masking::Secret;
use redis_interface::{errors::RedisError, RedisConnectionPool, RedisEntryId};
use router_env::logger;
use scheduler::{
    db::{process_tracker::ProcessTrackerInterface, queue::QueueInterface},
    SchedulerInterface,
};
use storage_impl::redis::kv_store::RedisConnInterface;
use time::PrimitiveDateTime;

use super::{
    dashboard_metadata::DashboardMetadataInterface,
    role::RoleInterface,
    user::{sample_data::BatchSampleDataInterface, UserInterface},
    user_role::UserRoleInterface,
};
use crate::{
    core::errors::{self, ProcessTrackerError},
    db::{
        address::AddressInterface,
        api_keys::ApiKeyInterface,
        authentication::AuthenticationInterface,
        authorization::AuthorizationInterface,
        business_profile::BusinessProfileInterface,
        capture::CaptureInterface,
        cards_info::CardsInfoInterface,
        configs::ConfigInterface,
        customers::CustomerInterface,
        dispute::DisputeInterface,
        ephemeral_key::EphemeralKeyInterface,
        events::EventInterface,
        file::FileMetadataInterface,
        gsm::GsmInterface,
        health_check::HealthCheckDbInterface,
        locker_mock_up::LockerMockUpInterface,
        mandate::MandateInterface,
        merchant_account::MerchantAccountInterface,
        merchant_connector_account::{ConnectorAccessToken, MerchantConnectorAccountInterface},
        merchant_key_store::MerchantKeyStoreInterface,
        payment_link::PaymentLinkInterface,
        payment_method::PaymentMethodInterface,
        payout_attempt::PayoutAttemptInterface,
        payouts::PayoutsInterface,
        refund::RefundInterface,
        reverse_lookup::ReverseLookupInterface,
        routing_algorithm::RoutingAlgorithmInterface,
        MasterKeyInterface, StorageInterface,
    },
    services::{authentication, kafka::KafkaProducer, Store},
    types::{
        domain,
        storage::{self, business_profile},
        AccessToken,
    },
};

#[derive(Clone)]
pub struct KafkaStore {
    kafka_producer: KafkaProducer,
    pub diesel_store: Store,
}

impl KafkaStore {
    pub async fn new(store: Store, kafka_producer: KafkaProducer) -> Self {
        Self {
            kafka_producer,
            diesel_store: store,
        }
    }
}

#[async_trait::async_trait]
impl AddressInterface for KafkaStore {
    async fn find_address_by_address_id(
        &self,
        address_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        self.diesel_store
            .find_address_by_address_id(address_id, key_store)
            .await
    }

    async fn update_address(
        &self,
        address_id: String,
        address: storage::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        self.diesel_store
            .update_address(address_id, address, key_store)
            .await
    }

    async fn update_address_for_payments(
        &self,
        this: domain::Address,
        address: domain::AddressUpdate,
        payment_id: String,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        self.diesel_store
            .update_address_for_payments(this, address, payment_id, key_store, storage_scheme)
            .await
    }

    async fn insert_address_for_payments(
        &self,
        payment_id: &str,
        address: domain::Address,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        self.diesel_store
            .insert_address_for_payments(payment_id, address, key_store, storage_scheme)
            .await
    }

    async fn find_address_by_merchant_id_payment_id_address_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        address_id: &str,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        self.diesel_store
            .find_address_by_merchant_id_payment_id_address_id(
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
        address: domain::Address,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Address, errors::StorageError> {
        self.diesel_store
            .insert_address_for_customers(address, key_store)
            .await
    }

    async fn update_address_by_merchant_id_customer_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        address: storage::AddressUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Address>, errors::StorageError> {
        self.diesel_store
            .update_address_by_merchant_id_customer_id(customer_id, merchant_id, address, key_store)
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
        merchant_id: String,
        key_id: String,
        api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        self.diesel_store
            .update_api_key(merchant_id, key_id, api_key)
            .await
    }

    async fn revoke_api_key(
        &self,
        merchant_id: &str,
        key_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store.revoke_api_key(merchant_id, key_id).await
    }

    async fn find_api_key_by_merchant_id_key_id_optional(
        &self,
        merchant_id: &str,
        key_id: &str,
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
        merchant_id: &str,
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
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_customer_by_customer_id_merchant_id(customer_id, merchant_id)
            .await
    }

    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
        self.diesel_store
            .find_customer_optional_by_customer_id_merchant_id(customer_id, merchant_id, key_store)
            .await
    }

    async fn update_customer_by_customer_id_merchant_id(
        &self,
        customer_id: String,
        merchant_id: String,
        customer: storage::CustomerUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        self.diesel_store
            .update_customer_by_customer_id_merchant_id(
                customer_id,
                merchant_id,
                customer,
                key_store,
            )
            .await
    }

    async fn list_customers_by_merchant_id(
        &self,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Customer>, errors::StorageError> {
        self.diesel_store
            .list_customers_by_merchant_id(merchant_id, key_store)
            .await
    }

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        self.diesel_store
            .find_customer_by_customer_id_merchant_id(customer_id, merchant_id, key_store)
            .await
    }

    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        self.diesel_store
            .insert_customer(customer_data, key_store)
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

        if let Err(er) = self.kafka_producer.log_dispute(&dispute, None).await {
            logger::error!(message="Failed to add analytics entry for Dispute {dispute:?}", error_message=?er);
        };

        Ok(dispute)
    }

    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
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
        merchant_id: &str,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        self.diesel_store
            .find_dispute_by_merchant_id_dispute_id(merchant_id, dispute_id)
            .await
    }

    async fn find_disputes_by_merchant_id(
        &self,
        merchant_id: &str,
        dispute_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        self.diesel_store
            .find_disputes_by_merchant_id(merchant_id, dispute_constraints)
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
            .log_dispute(&dispute_new, Some(this))
            .await
        {
            logger::error!(message="Failed to add analytics entry for Dispute {dispute_new:?}", error_message=?er);
        };

        Ok(dispute_new)
    }

    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        self.diesel_store
            .find_disputes_by_merchant_id_payment_id(merchant_id, payment_id)
            .await
    }
}

#[async_trait::async_trait]
impl EphemeralKeyInterface for KafkaStore {
    async fn create_ephemeral_key(
        &self,
        ek: EphemeralKeyNew,
        validity: i64,
    ) -> CustomResult<EphemeralKey, errors::StorageError> {
        self.diesel_store.create_ephemeral_key(ek, validity).await
    }
    async fn get_ephemeral_key(
        &self,
        key: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError> {
        self.diesel_store.get_ephemeral_key(key).await
    }
    async fn delete_ephemeral_key(
        &self,
        id: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError> {
        self.diesel_store.delete_ephemeral_key(id).await
    }
}

#[async_trait::async_trait]
impl EventInterface for KafkaStore {
    async fn insert_event(
        &self,
        event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        self.diesel_store.insert_event(event).await
    }

    async fn update_event(
        &self,
        event_id: String,
        event: storage::EventUpdate,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        self.diesel_store.update_event(event_id, event).await
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
        merchant_id: &str,
        mandate_id: &str,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        self.diesel_store
            .find_mandate_by_merchant_id_mandate_id(merchant_id, mandate_id)
            .await
    }

    async fn find_mandate_by_merchant_id_connector_mandate_id(
        &self,
        merchant_id: &str,
        connector_mandate_id: &str,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        self.diesel_store
            .find_mandate_by_merchant_id_connector_mandate_id(merchant_id, connector_mandate_id)
            .await
    }

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        self.diesel_store
            .find_mandate_by_merchant_id_customer_id(merchant_id, customer_id)
            .await
    }

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        mandate: storage::MandateUpdate,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        self.diesel_store
            .update_mandate_by_merchant_id_mandate_id(merchant_id, mandate_id, mandate)
            .await
    }

    async fn find_mandates_by_merchant_id(
        &self,
        merchant_id: &str,
        mandate_constraints: api_models::mandates::MandateListConstraints,
    ) -> CustomResult<Vec<storage::Mandate>, errors::StorageError> {
        self.diesel_store
            .find_mandates_by_merchant_id(merchant_id, mandate_constraints)
            .await
    }

    async fn insert_mandate(
        &self,
        mandate: storage::MandateNew,
    ) -> CustomResult<storage::Mandate, errors::StorageError> {
        self.diesel_store.insert_mandate(mandate).await
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
        merchant_id: &str,
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
        merchant_account: domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        self.diesel_store
            .insert_merchant(merchant_account, key_store)
            .await
    }

    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        self.diesel_store
            .find_merchant_account_by_merchant_id(merchant_id, key_store)
            .await
    }

    async fn update_merchant(
        &self,
        this: domain::MerchantAccount,
        merchant_account: storage::MerchantAccountUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        self.diesel_store
            .update_merchant(this, merchant_account, key_store)
            .await
    }

    async fn update_specific_fields_in_merchant(
        &self,
        merchant_id: &str,
        merchant_account: storage::MerchantAccountUpdate,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        self.diesel_store
            .update_specific_fields_in_merchant(merchant_id, merchant_account, key_store)
            .await
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<authentication::AuthenticationData, errors::StorageError> {
        self.diesel_store
            .find_merchant_account_by_publishable_key(publishable_key)
            .await
    }

    #[cfg(feature = "olap")]
    async fn list_merchant_accounts_by_organization_id(
        &self,
        organization_id: &str,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        self.diesel_store
            .list_merchant_accounts_by_organization_id(organization_id)
            .await
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_merchant_account_by_merchant_id(merchant_id)
            .await
    }

    #[cfg(feature = "olap")]
    async fn list_multiple_merchant_accounts(
        &self,
        merchant_ids: Vec<String>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        self.diesel_store
            .list_multiple_merchant_accounts(merchant_ids)
            .await
    }
}

#[async_trait::async_trait]
impl ConnectorAccessToken for KafkaStore {
    async fn get_access_token(
        &self,
        merchant_id: &str,
        connector_name: &str,
    ) -> CustomResult<Option<AccessToken>, errors::StorageError> {
        self.diesel_store
            .get_access_token(merchant_id, connector_name)
            .await
    }

    async fn set_access_token(
        &self,
        merchant_id: &str,
        connector_name: &str,
        access_token: AccessToken,
    ) -> CustomResult<(), errors::StorageError> {
        self.diesel_store
            .set_access_token(merchant_id, connector_name, access_token)
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
        merchant_id: &str,
        file_id: &str,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        self.diesel_store
            .find_file_metadata_by_merchant_id_file_id(merchant_id, file_id)
            .await
    }

    async fn delete_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &str,
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
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        merchant_id: &str,
        connector: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .find_merchant_connector_account_by_merchant_id_connector_label(
                merchant_id,
                connector,
                key_store,
            )
            .await
    }

    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        merchant_id: &str,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        self.diesel_store
            .find_merchant_connector_account_by_merchant_id_connector_name(
                merchant_id,
                connector_name,
                key_store,
            )
            .await
    }

    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        profile_id: &str,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .find_merchant_connector_account_by_profile_id_connector_name(
                profile_id,
                connector_name,
                key_store,
            )
            .await
    }

    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .insert_merchant_connector_account(t, key_store)
            .await
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                merchant_id,
                merchant_connector_id,
                key_store,
            )
            .await
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &str,
        get_disabled: bool,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        self.diesel_store
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                merchant_id,
                get_disabled,
                key_store,
            )
            .await
    }

    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        self.diesel_store
            .update_merchant_connector_account(this, merchant_connector_account, key_store)
            .await
    }

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
                merchant_id,
                merchant_connector_id,
            )
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
            .log_payment_attempt(&attempt, None)
            .await
        {
            logger::error!(message="Failed to log analytics event for payment attempt {attempt:?}", error_message=?er)
        }

        Ok(attempt)
    }

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
            .log_payment_attempt(&attempt, Some(this))
            .await
        {
            logger::error!(message="Failed to log analytics event for payment attempt {attempt:?}", error_message=?er)
        }

        Ok(attempt)
    }

    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        connector_transaction_id: &str,
        payment_id: &str,
        merchant_id: &str,
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

    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &str,
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

    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
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

    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        attempt_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .find_payment_attempt_by_attempt_id_merchant_id(attempt_id, merchant_id, storage_scheme)
            .await
    }

    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
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

    async fn find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
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

    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        preprocessing_id: &str,
        merchant_id: &str,
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

    async fn get_filters_for_payments(
        &self,
        pi: &[data_models::payments::PaymentIntent],
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<
        data_models::payments::payment_attempt::PaymentListFilters,
        errors::DataStorageError,
    > {
        self.diesel_store
            .get_filters_for_payments(pi, merchant_id, storage_scheme)
            .await
    }

    async fn get_total_count_of_filtered_payment_attempts(
        &self,
        merchant_id: &str,
        active_attempt_ids: &[String],
        connector: Option<Vec<api_models::enums::Connector>>,
        payment_method: Option<Vec<common_enums::PaymentMethod>>,
        payment_method_type: Option<Vec<common_enums::PaymentMethodType>>,
        authentication_type: Option<Vec<common_enums::AuthenticationType>>,
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
                storage_scheme,
            )
            .await
    }

    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
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
        this: storage::PaymentIntent,
        payment_intent: storage::PaymentIntentUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentIntent, errors::DataStorageError> {
        let intent = self
            .diesel_store
            .update_payment_intent(this.clone(), payment_intent, storage_scheme)
            .await?;

        if let Err(er) = self
            .kafka_producer
            .log_payment_intent(&intent, Some(this))
            .await
        {
            logger::error!(message="Failed to add analytics entry for Payment Intent {intent:?}", error_message=?er);
        };

        Ok(intent)
    }

    async fn insert_payment_intent(
        &self,
        new: storage::PaymentIntentNew,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentIntent, errors::DataStorageError> {
        logger::debug!("Inserting PaymentIntent Via KafkaStore");
        let intent = self
            .diesel_store
            .insert_payment_intent(new, storage_scheme)
            .await?;

        if let Err(er) = self.kafka_producer.log_payment_intent(&intent, None).await {
            logger::error!(message="Failed to add analytics entry for Payment Intent {intent:?}", error_message=?er);
        };

        Ok(intent)
    }

    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::PaymentIntent, errors::DataStorageError> {
        self.diesel_store
            .find_payment_intent_by_payment_id_merchant_id(payment_id, merchant_id, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn filter_payment_intent_by_constraints(
        &self,
        merchant_id: &str,
        filters: &data_models::payments::payment_intent::PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::PaymentIntent>, errors::DataStorageError> {
        self.diesel_store
            .filter_payment_intent_by_constraints(merchant_id, filters, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        merchant_id: &str,
        time_range: &api_models::payments::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::PaymentIntent>, errors::DataStorageError> {
        self.diesel_store
            .filter_payment_intents_by_time_range_constraints(
                merchant_id,
                time_range,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_filtered_payment_intents_attempt(
        &self,
        merchant_id: &str,
        constraints: &data_models::payments::payment_intent::PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<
        Vec<(
            data_models::payments::PaymentIntent,
            data_models::payments::payment_attempt::PaymentAttempt,
        )>,
        errors::DataStorageError,
    > {
        self.diesel_store
            .get_filtered_payment_intents_attempt(merchant_id, constraints, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &str,
        constraints: &data_models::payments::payment_intent::PaymentIntentFetchConstraints,
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

    async fn get_active_payment_attempt(
        &self,
        payment: &mut storage::PaymentIntent,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<storage::PaymentAttempt, errors::DataStorageError> {
        self.diesel_store
            .get_active_payment_attempt(payment, storage_scheme)
            .await
    }
}

#[async_trait::async_trait]
impl PaymentMethodInterface for KafkaStore {
    async fn find_payment_method(
        &self,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .find_payment_method(payment_method_id)
            .await
    }

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<storage::PaymentMethod>, errors::StorageError> {
        self.diesel_store
            .find_payment_method_by_customer_id_merchant_id_list(customer_id, merchant_id)
            .await
    }

    async fn find_payment_method_by_locker_id(
        &self,
        locker_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .find_payment_method_by_locker_id(locker_id)
            .await
    }

    async fn insert_payment_method(
        &self,
        m: storage::PaymentMethodNew,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        self.diesel_store.insert_payment_method(m).await
    }

    async fn update_payment_method(
        &self,
        payment_method: storage::PaymentMethod,
        payment_method_update: storage::PaymentMethodUpdate,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .update_payment_method(payment_method, payment_method_update)
            .await
    }

    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
        self.diesel_store
            .delete_payment_method_by_merchant_id_payment_method_id(merchant_id, payment_method_id)
            .await
    }
}

#[async_trait::async_trait]
impl PayoutAttemptInterface for KafkaStore {
    async fn find_payout_attempt_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        self.diesel_store
            .find_payout_attempt_by_merchant_id_payout_id(merchant_id, payout_id)
            .await
    }

    async fn update_payout_attempt_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
        payout: storage::PayoutAttemptUpdate,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        self.diesel_store
            .update_payout_attempt_by_merchant_id_payout_id(merchant_id, payout_id, payout)
            .await
    }

    async fn insert_payout_attempt(
        &self,
        payout: storage::PayoutAttemptNew,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        self.diesel_store.insert_payout_attempt(payout).await
    }
}

#[async_trait::async_trait]
impl PayoutsInterface for KafkaStore {
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        self.diesel_store
            .find_payout_by_merchant_id_payout_id(merchant_id, payout_id)
            .await
    }

    async fn update_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
        payout: storage::PayoutsUpdate,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        self.diesel_store
            .update_payout_by_merchant_id_payout_id(merchant_id, payout_id, payout)
            .await
    }

    async fn insert_payout(
        &self,
        payout: storage::PayoutsNew,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        self.diesel_store.insert_payout(payout).await
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
        business_status: String,
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
        merchant_id: &str,
        payment_id: &str,
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
        merchant_id: &str,
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
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::Refund>, errors::StorageError> {
        self.diesel_store
            .find_refund_by_payment_id_merchant_id(payment_id, merchant_id, storage_scheme)
            .await
    }

    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &str,
        refund_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage::Refund, errors::StorageError> {
        self.diesel_store
            .find_refund_by_merchant_id_refund_id(merchant_id, refund_id, storage_scheme)
            .await
    }

    async fn find_refund_by_merchant_id_connector_refund_id_connector(
        &self,
        merchant_id: &str,
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

        if let Err(er) = self.kafka_producer.log_refund(&refund, Some(this)).await {
            logger::error!(message="Failed to insert analytics event for Refund Update {refund?}", error_message=?er);
        }
        Ok(refund)
    }

    async fn find_refund_by_merchant_id_connector_transaction_id(
        &self,
        merchant_id: &str,
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

        if let Err(er) = self.kafka_producer.log_refund(&refund, None).await {
            logger::error!(message="Failed to insert analytics event for Refund Create {refund?}", error_message=?er);
        }
        Ok(refund)
    }

    #[cfg(feature = "olap")]
    async fn filter_refund_by_constraints(
        &self,
        merchant_id: &str,
        refund_details: &api_models::refunds::RefundListRequest,
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
        merchant_id: &str,
        refund_details: &api_models::payments::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::StorageError> {
        self.diesel_store
            .filter_refund_by_meta_constraints(merchant_id, refund_details, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_total_count_of_refunds(
        &self,
        merchant_id: &str,
        refund_details: &api_models::refunds::RefundListRequest,
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
        merchant_key_store: domain::MerchantKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError> {
        self.diesel_store
            .insert_merchant_key_store(merchant_key_store, key)
            .await
    }

    async fn get_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &str,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError> {
        self.diesel_store
            .get_merchant_key_store_by_merchant_id(merchant_id, key)
            .await
    }

    async fn delete_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_merchant_key_store_by_merchant_id(merchant_id)
            .await
    }

    #[cfg(feature = "olap")]
    async fn list_multiple_key_stores(
        &self,
        merchant_ids: Vec<String>,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, errors::StorageError> {
        self.diesel_store
            .list_multiple_key_stores(merchant_ids, key)
            .await
    }
}

#[async_trait::async_trait]
impl BusinessProfileInterface for KafkaStore {
    async fn insert_business_profile(
        &self,
        business_profile: business_profile::BusinessProfileNew,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        self.diesel_store
            .insert_business_profile(business_profile)
            .await
    }

    async fn find_business_profile_by_profile_id(
        &self,
        profile_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        self.diesel_store
            .find_business_profile_by_profile_id(profile_id)
            .await
    }

    async fn update_business_profile_by_profile_id(
        &self,
        current_state: business_profile::BusinessProfile,
        business_profile_update: business_profile::BusinessProfileUpdateInternal,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        self.diesel_store
            .update_business_profile_by_profile_id(current_state, business_profile_update)
            .await
    }

    async fn delete_business_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_business_profile_by_profile_id_merchant_id(profile_id, merchant_id)
            .await
    }

    async fn list_business_profile_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<business_profile::BusinessProfile>, errors::StorageError> {
        self.diesel_store
            .list_business_profile_by_merchant_id(merchant_id)
            .await
    }

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        profile_name: &str,
        merchant_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        self.diesel_store
            .find_business_profile_by_profile_name_merchant_id(profile_name, merchant_id)
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
        profile_id: &str,
        algorithm_id: &str,
    ) -> CustomResult<storage::RoutingAlgorithm, errors::StorageError> {
        self.diesel_store
            .find_routing_algorithm_by_profile_id_algorithm_id(profile_id, algorithm_id)
            .await
    }

    async fn find_routing_algorithm_by_algorithm_id_merchant_id(
        &self,
        algorithm_id: &str,
        merchant_id: &str,
    ) -> CustomResult<storage::RoutingAlgorithm, errors::StorageError> {
        self.diesel_store
            .find_routing_algorithm_by_algorithm_id_merchant_id(algorithm_id, merchant_id)
            .await
    }

    async fn find_routing_algorithm_metadata_by_algorithm_id_profile_id(
        &self,
        algorithm_id: &str,
        profile_id: &str,
    ) -> CustomResult<storage::RoutingProfileMetadata, errors::StorageError> {
        self.diesel_store
            .find_routing_algorithm_metadata_by_algorithm_id_profile_id(algorithm_id, profile_id)
            .await
    }

    async fn list_routing_algorithm_metadata_by_profile_id(
        &self,
        profile_id: &str,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::RoutingAlgorithmMetadata>, errors::StorageError> {
        self.diesel_store
            .list_routing_algorithm_metadata_by_profile_id(profile_id, limit, offset)
            .await
    }

    async fn list_routing_algorithm_metadata_by_merchant_id(
        &self,
        merchant_id: &str,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::RoutingProfileMetadata>, errors::StorageError> {
        self.diesel_store
            .list_routing_algorithm_metadata_by_merchant_id(merchant_id, limit, offset)
            .await
    }

    async fn list_routing_algorithm_metadata_by_merchant_id_transaction_type(
        &self,
        merchant_id: &str,
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
impl StorageInterface for KafkaStore {
    fn get_scheduler_db(&self) -> Box<dyn SchedulerInterface> {
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
        user_email: &str,
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
        user_email: &str,
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

    async fn find_users_and_roles_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<(storage::User, user_storage::UserRole)>, errors::StorageError> {
        self.diesel_store
            .find_users_and_roles_by_merchant_id(merchant_id)
            .await
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
        user_role: user_storage::UserRoleNew,
    ) -> CustomResult<user_storage::UserRole, errors::StorageError> {
        self.diesel_store.insert_user_role(user_role).await
    }

    async fn find_user_role_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<user_storage::UserRole, errors::StorageError> {
        self.diesel_store.find_user_role_by_user_id(user_id).await
    }

    async fn find_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<user_storage::UserRole, errors::StorageError> {
        self.diesel_store
            .find_user_role_by_user_id_merchant_id(user_id, merchant_id)
            .await
    }

    async fn update_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
        update: user_storage::UserRoleUpdate,
    ) -> CustomResult<user_storage::UserRole, errors::StorageError> {
        self.diesel_store
            .update_user_role_by_user_id_merchant_id(user_id, merchant_id, update)
            .await
    }

    async fn update_user_roles_by_user_id_org_id(
        &self,
        user_id: &str,
        org_id: &str,
        update: user_storage::UserRoleUpdate,
    ) -> CustomResult<Vec<user_storage::UserRole>, errors::StorageError> {
        self.diesel_store
            .update_user_roles_by_user_id_org_id(user_id, org_id, update)
            .await
    }

    async fn delete_user_role_by_user_id_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_user_role_by_user_id_merchant_id(user_id, merchant_id)
            .await
    }

    async fn list_user_roles_by_user_id(
        &self,
        user_id: &str,
    ) -> CustomResult<Vec<user_storage::UserRole>, errors::StorageError> {
        self.diesel_store.list_user_roles_by_user_id(user_id).await
    }

    async fn transfer_org_ownership_between_users(
        &self,
        from_user_id: &str,
        to_user_id: &str,
        org_id: &str,
    ) -> CustomResult<(), errors::StorageError> {
        self.diesel_store
            .transfer_org_ownership_between_users(from_user_id, to_user_id, org_id)
            .await
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
        merchant_id: String,
        org_id: String,
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
        merchant_id: &str,
        org_id: &str,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError> {
        self.diesel_store
            .find_user_scoped_dashboard_metadata(user_id, merchant_id, org_id, data_keys)
            .await
    }

    async fn find_merchant_scoped_dashboard_metadata(
        &self,
        merchant_id: &str,
        org_id: &str,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError> {
        self.diesel_store
            .find_merchant_scoped_dashboard_metadata(merchant_id, org_id, data_keys)
            .await
    }

    async fn delete_all_user_scoped_dashboard_metadata_by_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_all_user_scoped_dashboard_metadata_by_merchant_id(user_id, merchant_id)
            .await
    }

    async fn delete_user_scoped_dashboard_metadata_by_merchant_id_data_key(
        &self,
        user_id: &str,
        merchant_id: &str,
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
    async fn insert_payment_intents_batch_for_sample_data(
        &self,
        batch: Vec<data_models::payments::payment_intent::PaymentIntentNew>,
    ) -> CustomResult<Vec<data_models::payments::PaymentIntent>, data_models::errors::StorageError>
    {
        let payment_intents_list = self
            .diesel_store
            .insert_payment_intents_batch_for_sample_data(batch)
            .await?;

        for payment_intent in payment_intents_list.iter() {
            let _ = self
                .kafka_producer
                .log_payment_intent(payment_intent, None)
                .await;
        }
        Ok(payment_intents_list)
    }

    async fn insert_payment_attempts_batch_for_sample_data(
        &self,
        batch: Vec<diesel_models::user::sample_data::PaymentAttemptBatchNew>,
    ) -> CustomResult<
        Vec<data_models::payments::payment_attempt::PaymentAttempt>,
        data_models::errors::StorageError,
    > {
        let payment_attempts_list = self
            .diesel_store
            .insert_payment_attempts_batch_for_sample_data(batch)
            .await?;

        for payment_attempt in payment_attempts_list.iter() {
            let _ = self
                .kafka_producer
                .log_payment_attempt(payment_attempt, None)
                .await;
        }
        Ok(payment_attempts_list)
    }

    async fn insert_refunds_batch_for_sample_data(
        &self,
        batch: Vec<diesel_models::RefundNew>,
    ) -> CustomResult<Vec<diesel_models::Refund>, data_models::errors::StorageError> {
        let refunds_list = self
            .diesel_store
            .insert_refunds_batch_for_sample_data(batch)
            .await?;

        for refund in refunds_list.iter() {
            let _ = self.kafka_producer.log_refund(refund, None).await;
        }
        Ok(refunds_list)
    }

    async fn delete_payment_intents_for_sample_data(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<data_models::payments::PaymentIntent>, data_models::errors::StorageError>
    {
        let payment_intents_list = self
            .diesel_store
            .delete_payment_intents_for_sample_data(merchant_id)
            .await?;

        for payment_intent in payment_intents_list.iter() {
            let _ = self
                .kafka_producer
                .log_payment_intent_delete(payment_intent)
                .await;
        }
        Ok(payment_intents_list)
    }

    async fn delete_payment_attempts_for_sample_data(
        &self,
        merchant_id: &str,
    ) -> CustomResult<
        Vec<data_models::payments::payment_attempt::PaymentAttempt>,
        data_models::errors::StorageError,
    > {
        let payment_attempts_list = self
            .diesel_store
            .delete_payment_attempts_for_sample_data(merchant_id)
            .await?;

        for payment_attempt in payment_attempts_list.iter() {
            let _ = self
                .kafka_producer
                .log_payment_attempt_delete(payment_attempt)
                .await;
        }

        Ok(payment_attempts_list)
    }

    async fn delete_refunds_for_sample_data(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<diesel_models::Refund>, data_models::errors::StorageError> {
        let refunds_list = self
            .diesel_store
            .delete_refunds_for_sample_data(merchant_id)
            .await?;

        for refund in refunds_list.iter() {
            let _ = self.kafka_producer.log_refund_delete(refund).await;
        }

        Ok(refunds_list)
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
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Authorization>, errors::StorageError> {
        self.diesel_store
            .find_all_authorizations_by_merchant_id_payment_id(merchant_id, payment_id)
            .await
    }

    async fn update_authorization_by_merchant_id_authorization_id(
        &self,
        merchant_id: String,
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
        self.diesel_store
            .insert_authentication(authentication)
            .await
    }

    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: String,
        authentication_id: String,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        self.diesel_store
            .find_authentication_by_merchant_id_authentication_id(merchant_id, authentication_id)
            .await
    }

    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: storage::Authentication,
        authentication_update: storage::AuthenticationUpdate,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        self.diesel_store
            .update_authentication_by_merchant_id_authentication_id(
                previous_state,
                authentication_update,
            )
            .await
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

    async fn find_role_by_role_id_in_merchant_scope(
        &self,
        role_id: &str,
        merchant_id: &str,
        org_id: &str,
    ) -> CustomResult<storage::Role, errors::StorageError> {
        self.diesel_store
            .find_role_by_role_id_in_merchant_scope(role_id, merchant_id, org_id)
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

    async fn list_all_roles(
        &self,
        merchant_id: &str,
        org_id: &str,
    ) -> CustomResult<Vec<storage::Role>, errors::StorageError> {
        self.diesel_store.list_all_roles(merchant_id, org_id).await
    }
}
