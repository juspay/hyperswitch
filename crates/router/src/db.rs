pub mod address;
pub mod api_keys;
pub mod authorization;
pub mod blocklist;
pub mod blocklist_fingerprint;
pub mod blocklist_lookup;
pub mod business_profile;
pub mod cache;
pub mod capture;
pub mod cards_info;
pub mod configs;
pub mod customers;
pub mod dashboard_metadata;
pub mod dispute;
pub mod ephemeral_key;
pub mod events;
pub mod file;
pub mod fraud_check;
pub mod gsm;
pub mod health_check;
mod kafka_store;
pub mod locker_mock_up;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod organization;
pub mod payment_link;
pub mod payment_method;
pub mod payout_attempt;
pub mod payouts;
pub mod refund;
pub mod reverse_lookup;
pub mod routing_algorithm;
pub mod user;
pub mod user_role;

use data_models::payments::{
    payment_attempt::PaymentAttemptInterface, payment_intent::PaymentIntentInterface,
};
use diesel_models::{
    fraud_check::{FraudCheck, FraudCheckNew, FraudCheckUpdate},
    organization::{Organization, OrganizationNew, OrganizationUpdate},
};
use error_stack::ResultExt;
use masking::PeekInterface;
use redis_interface::errors::RedisError;
use storage_impl::{errors::StorageError, redis::kv_store::RedisConnInterface, MockDb};

pub use self::kafka_store::KafkaStore;
use self::{fraud_check::FraudCheckInterface, organization::OrganizationInterface};
pub use crate::{
    errors::CustomResult,
    services::{
        kafka::{KafkaError, KafkaProducer, MQResult},
        Store,
    },
};

#[derive(PartialEq, Eq)]
pub enum StorageImpl {
    Postgresql,
    PostgresqlTest,
    Mock,
}

#[async_trait::async_trait]
pub trait StorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + address::AddressInterface
    + api_keys::ApiKeyInterface
    + blocklist_lookup::BlocklistLookupInterface
    + configs::ConfigInterface
    + capture::CaptureInterface
    + customers::CustomerInterface
    + dashboard_metadata::DashboardMetadataInterface
    + dispute::DisputeInterface
    + ephemeral_key::EphemeralKeyInterface
    + events::EventInterface
    + file::FileMetadataInterface
    + FraudCheckInterface
    + locker_mock_up::LockerMockUpInterface
    + mandate::MandateInterface
    + merchant_account::MerchantAccountInterface
    + merchant_connector_account::ConnectorAccessToken
    + merchant_connector_account::MerchantConnectorAccountInterface
    + PaymentAttemptInterface
    + PaymentIntentInterface
    + payment_method::PaymentMethodInterface
    + blocklist::BlocklistInterface
    + blocklist_fingerprint::BlocklistFingerprintInterface
    + scheduler::SchedulerInterface
    + payout_attempt::PayoutAttemptInterface
    + payouts::PayoutsInterface
    + refund::RefundInterface
    + reverse_lookup::ReverseLookupInterface
    + cards_info::CardsInfoInterface
    + merchant_key_store::MerchantKeyStoreInterface
    + MasterKeyInterface
    + payment_link::PaymentLinkInterface
    + RedisConnInterface
    + RequestIdStore
    + business_profile::BusinessProfileInterface
    + OrganizationInterface
    + routing_algorithm::RoutingAlgorithmInterface
    + gsm::GsmInterface
    + user::UserInterface
    + user_role::UserRoleInterface
    + authorization::AuthorizationInterface
    + user::sample_data::BatchSampleDataInterface
    + health_check::HealthCheckDbInterface
    + 'static
{
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface>;
}

pub trait MasterKeyInterface {
    fn get_master_key(&self) -> &[u8];
}

impl MasterKeyInterface for Store {
        /// Retrieves the master key as a reference to a slice of unsigned 8-bit integers.
    fn get_master_key(&self) -> &[u8] {
        self.master_key().peek()
    }
}

/// Default dummy key for MockDb
impl MasterKeyInterface for MockDb {
        /// Returns the master key used for encryption and decryption.
    fn get_master_key(&self) -> &[u8] {
        &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]
    }
}

#[async_trait::async_trait]
impl StorageInterface for Store {
        /// Retrieves the scheduler database interface.
    /// 
    /// This method returns a Boxed trait object that implements the SchedulerInterface
    /// trait, allowing access to the scheduler database functionality.
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
impl StorageInterface for MockDb {
    fn get_scheduler_db(&self) -> Box<dyn scheduler::SchedulerInterface> {
        Box::new(self.clone())
    }
}

pub trait RequestIdStore {
        /// Adds a request ID to the current instance of the struct.
    ///
    /// # Arguments
    ///
    /// * `_request_id` - A String containing the request ID to be added.
    ///
    fn add_request_id(&mut self, _request_id: String) {}
        /// This method returns the request ID, if available.
    fn get_request_id(&self) -> Option<String> {
        None
    }
}

impl RequestIdStore for MockDb {}

impl RequestIdStore for Store {
        /// Adds a request ID to the struct.
    ///
    /// This method takes a request ID as a `String` and sets it as the `request_id` field of the struct.
    ///
    /// # Arguments
    ///
    /// * `request_id` - A `String` representing the request ID to be added.
    ///
    fn add_request_id(&mut self, request_id: String) {
        self.request_id = Some(request_id)
    }

        /// Returns the request ID associated with the current instance, if available.
    /// 
    /// # Returns
    /// 
    /// - `Some(String)`: If the request ID is available, it returns a cloned string containing the request ID.
    /// - `None`: If the request ID is not available, it returns `None`.
    fn get_request_id(&self) -> Option<String> {
        self.request_id.clone()
    }
}

/// Asynchronously retrieves a value from the specified key in the database and deserializes it into the given type using serde.
pub async fn get_and_deserialize_key<T>(
    db: &dyn StorageInterface,
    key: &str,
    type_name: &'static str,
) -> CustomResult<T, RedisError>
where
    T: serde::de::DeserializeOwned,
{
    use common_utils::ext_traits::ByteSliceExt;

    let bytes = db.get_key(key).await?;
    bytes
        .parse_struct(type_name)
        .change_context(redis_interface::errors::RedisError::JsonDeserializationFailed)
}

dyn_clone::clone_trait_object!(StorageInterface);

impl RequestIdStore for KafkaStore {
        /// Adds a request ID to the current instance of the struct.
    /// 
    /// # Arguments
    /// 
    /// * `request_id` - A String that represents the request ID to be added.
    /// 
    fn add_request_id(&mut self, request_id: String) {
        self.diesel_store.add_request_id(request_id)
    }
}

#[async_trait::async_trait]
impl FraudCheckInterface for KafkaStore {
        /// Inserts a new fraud check response into the storage and returns the inserted fraud check.
    /// 
    /// # Arguments
    /// 
    /// * `new` - The new fraud check response to be inserted.
    /// 
    /// # Returns
    /// 
    /// * `CustomResult<FraudCheck, StorageError>` - A result indicating the success or failure of the operation, containing the inserted fraud check if successful.
    /// 
    async fn insert_fraud_check_response(
        &self,
        new: FraudCheckNew,
    ) -> CustomResult<FraudCheck, StorageError> {
        self.diesel_store.insert_fraud_check_response(new).await
    }
        /// Updates the fraud check response with the provided attempt ID using the given fraud check and fraud check update.
    async fn update_fraud_check_response_with_attempt_id(
        &self,
        fraud_check: FraudCheck,
        fraud_check_update: FraudCheckUpdate,
    ) -> CustomResult<FraudCheck, StorageError> {
        self.diesel_store
            .update_fraud_check_response_with_attempt_id(fraud_check, fraud_check_update)
            .await
    }
        /// Asynchronously finds a fraud check for a specific payment and merchant by their IDs.
    /// 
    /// # Arguments
    /// 
    /// * `payment_id` - The unique identifier of the payment.
    /// * `merchant_id` - The unique identifier of the merchant.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a `FraudCheck` if the operation is successful, otherwise a `StorageError`.
    /// 
    async fn find_fraud_check_by_payment_id(
        &self,
        payment_id: String,
        merchant_id: String,
    ) -> CustomResult<FraudCheck, StorageError> {
        self.diesel_store
            .find_fraud_check_by_payment_id(payment_id, merchant_id)
            .await
    }
        /// Asynchronously finds a fraud check by payment ID if present in the database for a given merchant. 
    /// 
    /// # Arguments
    /// * `payment_id` - A string representing the payment ID for which the fraud check needs to be found.
    /// * `merchant_id` - A string representing the merchant ID for which the fraud check needs to be found.
    /// 
    /// # Returns
    /// * `CustomResult<Option<FraudCheck>, StorageError>` - A Result containing an Option of FraudCheck if found, otherwise a StorageError.
    /// 
    async fn find_fraud_check_by_payment_id_if_present(
        &self,
        payment_id: String,
        merchant_id: String,
    ) -> CustomResult<Option<FraudCheck>, StorageError> {
        self.diesel_store
            .find_fraud_check_by_payment_id_if_present(payment_id, merchant_id)
            .await
    }
}

#[async_trait::async_trait]
impl OrganizationInterface for KafkaStore {
        /// Asynchronously inserts a new organization into the storage.
    ///
    /// # Arguments
    ///
    /// * `organization` - The new organization to be inserted.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing the inserted `Organization` if successful, otherwise a `StorageError`.
    ///
    async fn insert_organization(
        &self,
        organization: OrganizationNew,
    ) -> CustomResult<Organization, StorageError> {
        self.diesel_store.insert_organization(organization).await
    }
        /// Asynchronously finds an organization by its organization ID.
    ///
    /// # Arguments
    ///
    /// * `org_id` - A reference to a string representing the organization ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the found `Organization` if successful, otherwise a `StorageError`.
    ///
    async fn find_organization_by_org_id(
        &self,
        org_id: &str,
    ) -> CustomResult<Organization, StorageError> {
        self.diesel_store.find_organization_by_org_id(org_id).await
    }

        /// Asynchronously updates an organization using the provided organization ID and update information.
    ///
    /// # Arguments
    ///
    /// * `org_id` - A string reference representing the unique identifier of the organization to update.
    /// * `update` - An `OrganizationUpdate` struct containing the information to update the organization with.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the updated `Organization` if successful, otherwise a `StorageError`.
    ///
    async fn update_organization_by_org_id(
        &self,
        org_id: &str,
        update: OrganizationUpdate,
    ) -> CustomResult<Organization, StorageError> {
        self.diesel_store
            .update_organization_by_org_id(org_id, update)
            .await
    }
}
