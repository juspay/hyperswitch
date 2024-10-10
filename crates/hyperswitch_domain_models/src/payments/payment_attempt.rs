#[cfg(all(feature = "v1", feature = "olap"))]
use api_models::enums::Connector;
use common_enums as storage_enums;
use common_utils::{
    errors::{CustomResult, ValidationError},
    id_type, pii,
    types::{
        keymanager::{self, KeyManagerState},
        MinorUnit,
    },
};
use diesel_models::{
    PaymentAttempt as DieselPaymentAttempt, PaymentAttemptNew as DieselPaymentAttemptNew,
};
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use masking::PeekInterface;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[cfg(all(feature = "v1", feature = "olap"))]
use super::PaymentIntent;
#[cfg(feature = "v2")]
use crate::merchant_key_store::MerchantKeyStore;
use crate::{
    behaviour, errors,
    mandates::{MandateDataType, MandateDetails},
    ForeignIDRef,
};

#[async_trait::async_trait]
pub trait PaymentAttemptInterface {
    #[cfg(feature = "v1")]
    async fn insert_payment_attempt(
        &self,
        payment_attempt: PaymentAttemptNew,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v2")]
    async fn insert_payment_attempt(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        payment_attempt: PaymentAttempt,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn update_payment_attempt_with_attempt_id(
        &self,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v2")]
    async fn update_payment_attempt(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        connector_transaction_id: &str,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &id_type::MerchantId,
        connector_txn_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        attempt_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        attempt_id: &str,
        merchant_id: &id_type::MerchantId,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v2")]
    async fn find_payment_attempt_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        attempt_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        preprocessing_id: &str,
        merchant_id: &id_type::MerchantId,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        merchant_id: &id_type::MerchantId,
        payment_id: &id_type::PaymentId,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentAttempt>, errors::StorageError>;

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_filters_for_payments(
        &self,
        pi: &[PaymentIntent],
        merchant_id: &id_type::MerchantId,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentListFilters, errors::StorageError>;

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[allow(clippy::too_many_arguments)]
    async fn get_total_count_of_filtered_payment_attempts(
        &self,
        merchant_id: &id_type::MerchantId,
        active_attempt_ids: &[String],
        connector: Option<Vec<Connector>>,
        payment_method: Option<Vec<storage_enums::PaymentMethod>>,
        payment_method_type: Option<Vec<storage_enums::PaymentMethodType>>,
        authentication_type: Option<Vec<storage_enums::AuthenticationType>>,
        merchant_connector_id: Option<Vec<id_type::MerchantConnectorAccountId>>,
        profile_id_list: Option<Vec<id_type::ProfileId>>,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<i64, errors::StorageError>;
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct AttemptAmountDetails {
    /// The total amount for this payment attempt. This includes all the surcharge and tax amounts.
    pub net_amount: MinorUnit,
    /// The amount that has to be captured,
    pub amount_to_capture: Option<MinorUnit>,
    /// Surcharge amount for the payment attempt.
    /// This is either derived by surcharge rules, or sent by the merchant
    pub surcharge_amount: Option<MinorUnit>,
    /// Tax amount for the payment attempt
    /// This is either derived by surcharge rules, or sent by the merchant
    pub tax_on_surcharge: Option<MinorUnit>,
    /// The total amount that can be captured for this payment attempt.
    pub amount_capturable: MinorUnit,
    /// Shipping cost for the payment attempt.
    pub shipping_cost: Option<MinorUnit>,
    /// Tax amount for the order.
    /// This is either derived by calling an external tax processor, or sent by the merchant
    pub order_tax_amount: Option<MinorUnit>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ErrorDetails {
    /// The error code that was returned by the connector.
    /// This is a mandatory field. This is used to lookup the global status map record for unified code and retries
    pub code: String,
    /// The error message that was returned by the connector.
    /// This is a mandatory field. This is used to lookup the global status map record for unified message and retries
    pub message: String,
    /// The detaild error reason that was returned by the connector.
    pub reason: Option<String>,
    /// The unified code that is generated by the application based on the global status map record.
    /// This can be relied upon for common error code across all connectors
    pub unified_code: Option<String>,
    /// The unified message that is generated by the application based on the global status map record.
    /// This can be relied upon for common error code across all connectors
    /// If there is translation avaiilable, message will be translated to the requested language
    pub unified_message: Option<String>,
}

/// Domain model for the payment attempt.
/// Few fields which are related are grouped together for better readability and understandability.
/// These fields will be flattened and stored in the database in individual columns
#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PaymentAttempt {
    /// Payment id for the payment attempt
    pub payment_id: id_type::GlobalPaymentId,
    /// Merchant id for the payment attempt
    pub merchant_id: id_type::MerchantId,
    /// Amount details for the payment attempt
    pub amount_details: AttemptAmountDetails,
    /// Status of the payment attempt. This is the status that is updated by the connector.
    /// The intent status is updated by the AttemptStatus.
    pub status: storage_enums::AttemptStatus,
    /// Name of the connector that was used for the payment attempt. The connector is either decided by
    /// either running the routing algorithm or by straight through processing request.
    /// This will be updated before calling the connector
    // TODO: use connector enum, this should be done in v1 as well as a part of moving to domain types wherever possible
    pub connector: Option<String>,
    /// Error details in case the payment attempt failed
    pub error: Option<ErrorDetails>,
    /// The authentication type that was requsted for the payment attempt.
    /// This authentication type maybe decided by step up 3ds or by running the decision engine.
    pub authentication_type: storage_enums::AuthenticationType,
    /// The time at which the payment attempt was created
    pub created_at: PrimitiveDateTime,
    /// The time at which the payment attempt was last modified
    pub modified_at: PrimitiveDateTime,
    pub last_synced: Option<PrimitiveDateTime>,
    /// The reason for the cancellation of the payment attempt. Some connectors will have strict rules regarding the values this can have
    /// Cancellation reason will be validated at the connector level when building the request
    pub cancellation_reason: Option<String>,
    /// Browser information required for 3DS authentication
    pub browser_info: Option<common_utils::types::BrowserInformation>,
    /// Payment token is the token used for temporary use in case the payment method is stored in vault
    pub payment_token: Option<String>,
    /// Metadata that is returned by the connector.
    pub connector_metadata: Option<pii::SecretSerdeValue>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    /// The insensitive data of the payment method stored
    // TODO: evaluate what details should be stored here. Use a domain type instead of serde_json::Value
    pub payment_method_data: Option<pii::SecretSerdeValue>,
    /// The result of the routing algorithm.
    /// This will store the list of connectors and other related information that was used to route the payment.
    pub routing_result: Option<serde_json::Value>,
    pub preprocessing_step_id: Option<String>,
    /// Number of captures that have happened for the payment attempt
    pub multiple_capture_count: Option<i16>,
    /// A reference to the payment at connector side. This is returned by the connector
    pub connector_response_reference_id: Option<String>,
    /// Whether the payment was updated by postgres or redis
    pub updated_by: String,
    /// The authentication data which is used for external authentication
    pub authentication_data: Option<pii::SecretSerdeValue>,
    pub encoded_data: Option<Secret<String>>,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    /// Whether external 3DS authentication was attempted for this payment.
    /// This is based on the configuration of the merchant in the business profile
    pub external_three_ds_authentication_attempted: Option<bool>,
    /// The connector that was used for external authentication
    pub authentication_connector: Option<String>,
    /// The foreign key reference to the authentication details
    pub authentication_id: Option<String>,
    pub fingerprint_id: Option<String>,
    pub charge_id: Option<String>,
    pub client_source: Option<String>,
    pub client_version: Option<String>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    /// The profile id for the payment attempt. This will be derived from payment intent.
    pub profile_id: id_type::ProfileId,
    /// The organization id for the payment attempt. This will be derived from payment intent.
    pub organization_id: id_type::OrganizationId,
    /// Payment method type for the payment attempt
    pub payment_method_type: storage_enums::PaymentMethod,
    /// Foreig key reference of Payment method id in case the payment instrument was stored
    pub payment_method_id: Option<id_type::GlobalPaymentMethodId>,
    /// The reference to the payment at the connector side
    pub connector_payment_id: Option<String>,
    /// The payment method subtype for the payment attempt.
    pub payment_method_subtype: storage_enums::PaymentMethodType,
    /// The authentication type that was applied for the payment attempt.
    pub authentication_applied: Option<common_enums::AuthenticationType>,
    /// A reference to the payment at connector side. This is returned by the connector
    pub external_reference_id: Option<String>,
    /// The billing address for the payment method
    pub payment_method_billing_address: common_utils::crypto::OptionalEncryptableValue,
    /// The global identifier for the payment attempt
    pub id: id_type::GlobalAttemptId,
}

impl PaymentAttempt {
    #[cfg(feature = "v1")]
    pub fn get_payment_method(&self) -> Option<storage_enums::PaymentMethod> {
        self.payment_method
    }

    #[cfg(feature = "v2")]
    pub fn get_payment_method(&self) -> Option<storage_enums::PaymentMethod> {
        // TODO: check if we can fix this
        Some(self.payment_method_type)
    }

    #[cfg(feature = "v1")]
    pub fn get_payment_method_type(&self) -> Option<storage_enums::PaymentMethodType> {
        self.payment_method_type
    }

    #[cfg(feature = "v2")]
    pub fn get_payment_method_type(&self) -> Option<storage_enums::PaymentMethodType> {
        // TODO: check if we can fix this
        Some(self.payment_method_subtype)
    }

    #[cfg(feature = "v1")]
    pub fn get_id(&self) -> &str {
        &self.attempt_id
    }

    #[cfg(feature = "v2")]
    pub fn get_id(&self) -> &id_type::GlobalAttemptId {
        &self.id
    }

    #[cfg(feature = "v1")]
    pub fn get_connector_payment_id(&self) -> Option<&str> {
        self.connector_transaction_id.as_deref()
    }

    #[cfg(feature = "v2")]
    pub fn get_connector_payment_id(&self) -> Option<&str> {
        self.connector_payment_id.as_deref()
    }

    /// Construct the domain model from the ConfirmIntentRequest
    #[cfg(feature = "v2")]
    pub async fn create_domain_model_from_request(
        state: &KeyManagerState,
        payment_intent: &super::PaymentIntent,
        cell_id: id_type::CellId,
        storage_scheme: storage_enums::MerchantStorageScheme,
        request: &api_models::payments::PaymentsConfirmIntentRequest,
    ) -> common_utils::errors::CustomResult<Self, errors::api_error_response::ApiErrorResponse>
    {
        let id = id_type::GlobalAttemptId::generate(&cell_id);
        let intent_amount_details = payment_intent.amount_details.clone();

        // TODO: move this to a impl function on payment attempt
        let attempt_amount_details = AttemptAmountDetails {
            net_amount: intent_amount_details.order_amount,
            amount_to_capture: None,
            surcharge_amount: None,
            tax_on_surcharge: None,
            amount_capturable: common_utils::types::MinorUnit::new(0),
            shipping_cost: None,
            order_tax_amount: None,
        };

        let now = common_utils::date_time::now();

        Ok(PaymentAttempt {
            payment_id: payment_intent.id.clone(),
            merchant_id: payment_intent.merchant_id.clone(),
            amount_details: attempt_amount_details,
            status: common_enums::AttemptStatus::Started,
            // This will be decided by the routing algorithm and updated in update trackers
            // right before calling the connector
            connector: None,
            authentication_type: payment_intent.authentication_type.clone(),
            created_at: now,
            modified_at: now,
            last_synced: None,
            cancellation_reason: None,
            browser_info: request.browser_info.clone(),
            payment_token: None,
            connector_metadata: None,
            payment_experience: None,
            payment_method_data: None,
            routing_result: None,
            preprocessing_step_id: None,
            multiple_capture_count: None,
            connector_response_reference_id: None,
            updated_by: storage_scheme.to_string(),
            authentication_data: None,
            encoded_data: None,
            merchant_connector_id: None,
            external_three_ds_authentication_attempted: None,
            authentication_connector: None,
            authentication_id: None,
            fingerprint_id: None,
            charge_id: None,
            client_source: None,
            client_version: None,
            customer_acceptance: None,
            profile_id: payment_intent.profile_id.clone(),
            organization_id: payment_intent.organization_id.clone(),
            payment_method_type: request.payment_method_type.clone(),
            payment_method_id: None,
            connector_payment_id: None,
            payment_method_subtype: request.payment_method_subtype,
            authentication_applied: None,
            external_reference_id: None,
            payment_method_billing_address: None,
            error: None,
            id,
        })
    }
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PaymentAttempt {
    pub payment_id: id_type::PaymentId,
    pub merchant_id: id_type::MerchantId,
    pub attempt_id: String,
    pub status: storage_enums::AttemptStatus,
    pub amount: MinorUnit,
    pub net_amount: MinorUnit,
    pub currency: Option<storage_enums::Currency>,
    pub save_to_locker: Option<bool>,
    pub connector: Option<String>,
    pub error_message: Option<String>,
    pub offer_amount: Option<MinorUnit>,
    pub surcharge_amount: Option<MinorUnit>,
    pub tax_amount: Option<MinorUnit>,
    pub payment_method_id: Option<String>,
    pub payment_method: Option<storage_enums::PaymentMethod>,
    pub connector_transaction_id: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub capture_on: Option<PrimitiveDateTime>,
    pub confirm: bool,
    pub authentication_type: Option<storage_enums::AuthenticationType>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub cancellation_reason: Option<String>,
    pub amount_to_capture: Option<MinorUnit>,
    pub mandate_id: Option<String>,
    pub browser_info: Option<serde_json::Value>,
    pub error_code: Option<String>,
    pub payment_token: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub payment_method_data: Option<serde_json::Value>,
    pub business_sub_label: Option<String>,
    pub straight_through_algorithm: Option<serde_json::Value>,
    pub preprocessing_step_id: Option<String>,
    // providing a location to store mandate details intermediately for transaction
    pub mandate_details: Option<MandateDataType>,
    pub error_reason: Option<String>,
    pub multiple_capture_count: Option<i16>,
    // reference to the payment at connector side
    pub connector_response_reference_id: Option<String>,
    pub amount_capturable: MinorUnit,
    pub updated_by: String,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub external_three_ds_authentication_attempted: Option<bool>,
    pub authentication_connector: Option<String>,
    pub authentication_id: Option<String>,
    pub mandate_data: Option<MandateDetails>,
    pub payment_method_billing_address_id: Option<String>,
    pub fingerprint_id: Option<String>,
    pub charge_id: Option<String>,
    pub client_source: Option<String>,
    pub client_version: Option<String>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub profile_id: id_type::ProfileId,
    pub organization_id: id_type::OrganizationId,
    pub shipping_cost: Option<MinorUnit>,
    pub order_tax_amount: Option<MinorUnit>,
}

#[cfg(feature = "v2")]
impl PaymentAttempt {
    pub fn get_total_amount(&self) -> MinorUnit {
        todo!();
    }

    pub fn get_total_surcharge_amount(&self) -> Option<MinorUnit> {
        todo!();
    }
}

#[cfg(feature = "v1")]
impl PaymentAttempt {
    pub fn get_total_amount(&self) -> MinorUnit {
        self.amount
            + self.surcharge_amount.unwrap_or_default()
            + self.tax_amount.unwrap_or_default()
    }

    pub fn get_total_surcharge_amount(&self) -> Option<MinorUnit> {
        self.surcharge_amount
            .map(|surcharge_amount| surcharge_amount + self.tax_amount.unwrap_or_default())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentListFilters {
    pub connector: Vec<String>,
    pub currency: Vec<storage_enums::Currency>,
    pub status: Vec<storage_enums::IntentStatus>,
    pub payment_method: Vec<storage_enums::PaymentMethod>,
    pub payment_method_type: Vec<storage_enums::PaymentMethodType>,
    pub authentication_type: Vec<storage_enums::AuthenticationType>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentAttemptNew {
    pub payment_id: id_type::PaymentId,
    pub merchant_id: id_type::MerchantId,
    pub attempt_id: String,
    pub status: storage_enums::AttemptStatus,
    pub amount: MinorUnit,
    /// amount + surcharge_amount + tax_amount
    /// This field will always be derived before updating in the Database
    pub net_amount: MinorUnit,
    pub currency: Option<storage_enums::Currency>,
    // pub auto_capture: Option<bool>,
    pub save_to_locker: Option<bool>,
    pub connector: Option<String>,
    pub error_message: Option<String>,
    pub offer_amount: Option<MinorUnit>,
    pub surcharge_amount: Option<MinorUnit>,
    pub tax_amount: Option<MinorUnit>,
    pub payment_method_id: Option<String>,
    pub payment_method: Option<storage_enums::PaymentMethod>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub capture_on: Option<PrimitiveDateTime>,
    pub confirm: bool,
    pub authentication_type: Option<storage_enums::AuthenticationType>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub modified_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub cancellation_reason: Option<String>,
    pub amount_to_capture: Option<MinorUnit>,
    pub mandate_id: Option<String>,
    pub browser_info: Option<serde_json::Value>,
    pub payment_token: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub payment_method_data: Option<serde_json::Value>,
    pub business_sub_label: Option<String>,
    pub straight_through_algorithm: Option<serde_json::Value>,
    pub preprocessing_step_id: Option<String>,
    pub mandate_details: Option<MandateDataType>,
    pub error_reason: Option<String>,
    pub connector_response_reference_id: Option<String>,
    pub multiple_capture_count: Option<i16>,
    pub amount_capturable: MinorUnit,
    pub updated_by: String,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub external_three_ds_authentication_attempted: Option<bool>,
    pub authentication_connector: Option<String>,
    pub authentication_id: Option<String>,
    pub mandate_data: Option<MandateDetails>,
    pub payment_method_billing_address_id: Option<String>,
    pub fingerprint_id: Option<String>,
    pub charge_id: Option<String>,
    pub client_source: Option<String>,
    pub client_version: Option<String>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub profile_id: id_type::ProfileId,
    pub organization_id: id_type::OrganizationId,
    pub shipping_cost: Option<MinorUnit>,
    pub order_tax_amount: Option<MinorUnit>,
}

#[cfg(feature = "v1")]
impl PaymentAttemptNew {
    /// returns amount + surcharge_amount + tax_amount
    pub fn calculate_net_amount(&self) -> MinorUnit {
        self.amount
            + self.surcharge_amount.unwrap_or_default()
            + self.tax_amount.unwrap_or_default()
            + self.shipping_cost.unwrap_or_default()
            + self.order_tax_amount.unwrap_or_default()
    }

    pub fn populate_derived_fields(self) -> Self {
        let mut payment_attempt_new = self;
        payment_attempt_new.net_amount = payment_attempt_new.calculate_net_amount();
        payment_attempt_new
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentAttemptUpdate {
    Update {
        amount: MinorUnit,
        currency: storage_enums::Currency,
        status: storage_enums::AttemptStatus,
        authentication_type: Option<storage_enums::AuthenticationType>,
        payment_method: Option<storage_enums::PaymentMethod>,
        payment_token: Option<String>,
        payment_method_data: Option<serde_json::Value>,
        payment_method_type: Option<storage_enums::PaymentMethodType>,
        payment_experience: Option<storage_enums::PaymentExperience>,
        business_sub_label: Option<String>,
        amount_to_capture: Option<MinorUnit>,
        capture_method: Option<storage_enums::CaptureMethod>,
        surcharge_amount: Option<MinorUnit>,
        tax_amount: Option<MinorUnit>,
        fingerprint_id: Option<String>,
        payment_method_billing_address_id: Option<String>,
        updated_by: String,
    },
    UpdateTrackers {
        payment_token: Option<String>,
        connector: Option<String>,
        straight_through_algorithm: Option<serde_json::Value>,
        amount_capturable: Option<MinorUnit>,
        surcharge_amount: Option<MinorUnit>,
        tax_amount: Option<MinorUnit>,
        updated_by: String,
        merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    },
    AuthenticationTypeUpdate {
        authentication_type: storage_enums::AuthenticationType,
        updated_by: String,
    },
    ConfirmUpdate {
        amount: MinorUnit,
        currency: storage_enums::Currency,
        status: storage_enums::AttemptStatus,
        authentication_type: Option<storage_enums::AuthenticationType>,
        capture_method: Option<storage_enums::CaptureMethod>,
        payment_method: Option<storage_enums::PaymentMethod>,
        browser_info: Option<serde_json::Value>,
        connector: Option<String>,
        payment_token: Option<String>,
        payment_method_data: Option<serde_json::Value>,
        payment_method_type: Option<storage_enums::PaymentMethodType>,
        payment_experience: Option<storage_enums::PaymentExperience>,
        business_sub_label: Option<String>,
        straight_through_algorithm: Option<serde_json::Value>,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
        amount_capturable: Option<MinorUnit>,
        updated_by: String,
        surcharge_amount: Option<MinorUnit>,
        tax_amount: Option<MinorUnit>,
        merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
        external_three_ds_authentication_attempted: Option<bool>,
        authentication_connector: Option<String>,
        authentication_id: Option<String>,
        payment_method_billing_address_id: Option<String>,
        fingerprint_id: Option<String>,
        payment_method_id: Option<String>,
        client_source: Option<String>,
        client_version: Option<String>,
        customer_acceptance: Option<pii::SecretSerdeValue>,
        shipping_cost: Option<MinorUnit>,
        order_tax_amount: Option<MinorUnit>,
    },
    RejectUpdate {
        status: storage_enums::AttemptStatus,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
        updated_by: String,
    },
    BlocklistUpdate {
        status: storage_enums::AttemptStatus,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
        updated_by: String,
    },
    PaymentMethodDetailsUpdate {
        payment_method_id: Option<String>,
        updated_by: String,
    },
    VoidUpdate {
        status: storage_enums::AttemptStatus,
        cancellation_reason: Option<String>,
        updated_by: String,
    },
    ResponseUpdate {
        status: storage_enums::AttemptStatus,
        connector: Option<String>,
        connector_transaction_id: Option<String>,
        authentication_type: Option<storage_enums::AuthenticationType>,
        payment_method_id: Option<String>,
        mandate_id: Option<String>,
        connector_metadata: Option<serde_json::Value>,
        payment_token: Option<String>,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
        error_reason: Option<Option<String>>,
        connector_response_reference_id: Option<String>,
        amount_capturable: Option<MinorUnit>,
        updated_by: String,
        authentication_data: Option<serde_json::Value>,
        encoded_data: Option<String>,
        unified_code: Option<Option<String>>,
        unified_message: Option<Option<String>>,
        payment_method_data: Option<serde_json::Value>,
        charge_id: Option<String>,
    },
    UnresolvedResponseUpdate {
        status: storage_enums::AttemptStatus,
        connector: Option<String>,
        connector_transaction_id: Option<String>,
        payment_method_id: Option<String>,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
        error_reason: Option<Option<String>>,
        connector_response_reference_id: Option<String>,
        updated_by: String,
    },
    StatusUpdate {
        status: storage_enums::AttemptStatus,
        updated_by: String,
    },
    ErrorUpdate {
        connector: Option<String>,
        status: storage_enums::AttemptStatus,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
        error_reason: Option<Option<String>>,
        amount_capturable: Option<MinorUnit>,
        updated_by: String,
        unified_code: Option<Option<String>>,
        unified_message: Option<Option<String>>,
        connector_transaction_id: Option<String>,
        payment_method_data: Option<serde_json::Value>,
        authentication_type: Option<storage_enums::AuthenticationType>,
    },
    CaptureUpdate {
        amount_to_capture: Option<MinorUnit>,
        multiple_capture_count: Option<i16>,
        updated_by: String,
    },
    AmountToCaptureUpdate {
        status: storage_enums::AttemptStatus,
        amount_capturable: MinorUnit,
        updated_by: String,
    },
    PreprocessingUpdate {
        status: storage_enums::AttemptStatus,
        payment_method_id: Option<String>,
        connector_metadata: Option<serde_json::Value>,
        preprocessing_step_id: Option<String>,
        connector_transaction_id: Option<String>,
        connector_response_reference_id: Option<String>,
        updated_by: String,
    },
    ConnectorResponse {
        authentication_data: Option<serde_json::Value>,
        encoded_data: Option<String>,
        connector_transaction_id: Option<String>,
        connector: Option<String>,
        charge_id: Option<String>,
        updated_by: String,
    },
    IncrementalAuthorizationAmountUpdate {
        amount: MinorUnit,
        amount_capturable: MinorUnit,
    },
    AuthenticationUpdate {
        status: storage_enums::AttemptStatus,
        external_three_ds_authentication_attempted: Option<bool>,
        authentication_connector: Option<String>,
        authentication_id: Option<String>,
        updated_by: String,
    },
    ManualUpdate {
        status: Option<storage_enums::AttemptStatus>,
        error_code: Option<String>,
        error_message: Option<String>,
        error_reason: Option<String>,
        updated_by: String,
        unified_code: Option<String>,
        unified_message: Option<String>,
        connector_transaction_id: Option<String>,
    },
}

// TODO: Add fields and variants as necessary
#[cfg(feature = "v2")]
#[derive(Debug, Clone, Serialize)]
pub enum PaymentAttemptUpdate {
    /// Update the payment attempt on confirming the intent, before calling the connector
    ConfirmIntent {
        status: storage_enums::AttemptStatus,
        updated_by: String,
        connector: String,
        merchant_connector_id: id_type::MerchantConnectorAccountId,
    },
    /// Update the payment attempt on confirming the intent, after calling the connector on success response
    ConfirmIntentResponse {
        status: storage_enums::AttemptStatus,
        connector_payment_id: Option<String>,
        updated_by: String,
    },
    /// Update the payment attempt on confirming the intent, after calling the connector on error response
    ConfirmIntentError {
        status: storage_enums::AttemptStatus,
        error: ErrorDetails,
        updated_by: String,
    },
}

#[cfg(feature = "v2")]
impl ForeignIDRef for PaymentAttempt {
    fn foreign_id(&self) -> String {
        todo!()
    }
}

#[cfg(feature = "v1")]
impl ForeignIDRef for PaymentAttempt {
    fn foreign_id(&self) -> String {
        self.attempt_id.clone()
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl behaviour::Conversion for PaymentAttempt {
    type DstType = DieselPaymentAttempt;
    type NewDstType = DieselPaymentAttemptNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let card_network = self
            .payment_method_data
            .as_ref()
            .and_then(|data| data.as_object())
            .and_then(|card| card.get("card"))
            .and_then(|data| data.as_object())
            .and_then(|card| card.get("card_network"))
            .and_then(|network| network.as_str())
            .map(|network| network.to_string());
        Ok(DieselPaymentAttempt {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.surcharge_amount,
            tax_amount: self.tax_amount,
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            connector_transaction_id: self.connector_transaction_id,
            capture_method: self.capture_method,
            capture_on: self.capture_on,
            confirm: self.confirm,
            authentication_type: self.authentication_type,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            cancellation_reason: self.cancellation_reason,
            amount_to_capture: self.amount_to_capture,
            mandate_id: self.mandate_id,
            browser_info: self.browser_info,
            error_code: self.error_code,
            payment_token: self.payment_token,
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_type: self.payment_method_type,
            payment_method_data: self.payment_method_data,
            business_sub_label: self.business_sub_label,
            straight_through_algorithm: self.straight_through_algorithm,
            preprocessing_step_id: self.preprocessing_step_id,
            mandate_details: self.mandate_details.map(Into::into),
            error_reason: self.error_reason,
            multiple_capture_count: self.multiple_capture_count,
            connector_response_reference_id: self.connector_response_reference_id,
            amount_capturable: self.amount_capturable,
            updated_by: self.updated_by,
            merchant_connector_id: self.merchant_connector_id,
            authentication_data: self.authentication_data,
            encoded_data: self.encoded_data,
            unified_code: self.unified_code,
            unified_message: self.unified_message,
            net_amount: Some(self.net_amount),
            external_three_ds_authentication_attempted: self
                .external_three_ds_authentication_attempted,
            authentication_connector: self.authentication_connector,
            authentication_id: self.authentication_id,
            mandate_data: self.mandate_data.map(Into::into),
            fingerprint_id: self.fingerprint_id,
            payment_method_billing_address_id: self.payment_method_billing_address_id,
            charge_id: self.charge_id,
            client_source: self.client_source,
            client_version: self.client_version,
            customer_acceptance: self.customer_acceptance,
            profile_id: self.profile_id,
            organization_id: self.organization_id,
            card_network,
            order_tax_amount: self.order_tax_amount,
            shipping_cost: self.shipping_cost,
        })
    }

    async fn convert_back(
        _state: &KeyManagerState,
        storage_model: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        async {
            let net_amount = storage_model.get_or_calculate_net_amount();
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                payment_id: storage_model.payment_id,
                merchant_id: storage_model.merchant_id,
                attempt_id: storage_model.attempt_id,
                status: storage_model.status,
                amount: storage_model.amount,
                net_amount,
                currency: storage_model.currency,
                save_to_locker: storage_model.save_to_locker,
                connector: storage_model.connector,
                error_message: storage_model.error_message,
                offer_amount: storage_model.offer_amount,
                surcharge_amount: storage_model.surcharge_amount,
                tax_amount: storage_model.tax_amount,
                payment_method_id: storage_model.payment_method_id,
                payment_method: storage_model.payment_method,
                connector_transaction_id: storage_model.connector_transaction_id,
                capture_method: storage_model.capture_method,
                capture_on: storage_model.capture_on,
                confirm: storage_model.confirm,
                authentication_type: storage_model.authentication_type,
                created_at: storage_model.created_at,
                modified_at: storage_model.modified_at,
                last_synced: storage_model.last_synced,
                cancellation_reason: storage_model.cancellation_reason,
                amount_to_capture: storage_model.amount_to_capture,
                mandate_id: storage_model.mandate_id,
                browser_info: storage_model.browser_info,
                error_code: storage_model.error_code,
                payment_token: storage_model.payment_token,
                connector_metadata: storage_model.connector_metadata,
                payment_experience: storage_model.payment_experience,
                payment_method_type: storage_model.payment_method_type,
                payment_method_data: storage_model.payment_method_data,
                business_sub_label: storage_model.business_sub_label,
                straight_through_algorithm: storage_model.straight_through_algorithm,
                preprocessing_step_id: storage_model.preprocessing_step_id,
                mandate_details: storage_model.mandate_details.map(Into::into),
                error_reason: storage_model.error_reason,
                multiple_capture_count: storage_model.multiple_capture_count,
                connector_response_reference_id: storage_model.connector_response_reference_id,
                amount_capturable: storage_model.amount_capturable,
                updated_by: storage_model.updated_by,
                authentication_data: storage_model.authentication_data,
                encoded_data: storage_model.encoded_data,
                merchant_connector_id: storage_model.merchant_connector_id,
                unified_code: storage_model.unified_code,
                unified_message: storage_model.unified_message,
                external_three_ds_authentication_attempted: storage_model
                    .external_three_ds_authentication_attempted,
                authentication_connector: storage_model.authentication_connector,
                authentication_id: storage_model.authentication_id,
                mandate_data: storage_model.mandate_data.map(Into::into),
                payment_method_billing_address_id: storage_model.payment_method_billing_address_id,
                fingerprint_id: storage_model.fingerprint_id,
                charge_id: storage_model.charge_id,
                client_source: storage_model.client_source,
                client_version: storage_model.client_version,
                customer_acceptance: storage_model.customer_acceptance,
                profile_id: storage_model.profile_id,
                organization_id: storage_model.organization_id,
                order_tax_amount: storage_model.order_tax_amount,
                shipping_cost: storage_model.shipping_cost,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment attempt".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let card_network = self
            .payment_method_data
            .as_ref()
            .and_then(|data| data.as_object())
            .and_then(|card| card.get("card"))
            .and_then(|data| data.as_object())
            .and_then(|card| card.get("card_network"))
            .and_then(|network| network.as_str())
            .map(|network| network.to_string());
        Ok(DieselPaymentAttemptNew {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.surcharge_amount,
            tax_amount: self.tax_amount,
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            capture_method: self.capture_method,
            capture_on: self.capture_on,
            confirm: self.confirm,
            authentication_type: self.authentication_type,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            cancellation_reason: self.cancellation_reason,
            amount_to_capture: self.amount_to_capture,
            mandate_id: self.mandate_id,
            browser_info: self.browser_info,
            payment_token: self.payment_token,
            error_code: self.error_code,
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_type: self.payment_method_type,
            payment_method_data: self.payment_method_data,
            business_sub_label: self.business_sub_label,
            straight_through_algorithm: self.straight_through_algorithm,
            preprocessing_step_id: self.preprocessing_step_id,
            mandate_details: self.mandate_details.map(Into::into),
            error_reason: self.error_reason,
            connector_response_reference_id: self.connector_response_reference_id,
            multiple_capture_count: self.multiple_capture_count,
            amount_capturable: self.amount_capturable,
            updated_by: self.updated_by,
            merchant_connector_id: self.merchant_connector_id,
            authentication_data: self.authentication_data,
            encoded_data: self.encoded_data,
            unified_code: self.unified_code,
            unified_message: self.unified_message,
            net_amount: Some(self.net_amount),
            external_three_ds_authentication_attempted: self
                .external_three_ds_authentication_attempted,
            authentication_connector: self.authentication_connector,
            authentication_id: self.authentication_id,
            mandate_data: self.mandate_data.map(Into::into),
            fingerprint_id: self.fingerprint_id,
            payment_method_billing_address_id: self.payment_method_billing_address_id,
            charge_id: self.charge_id,
            client_source: self.client_source,
            client_version: self.client_version,
            customer_acceptance: self.customer_acceptance,
            profile_id: self.profile_id,
            organization_id: self.organization_id,
            card_network,
            order_tax_amount: self.order_tax_amount,
            shipping_cost: self.shipping_cost,
        })
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl behaviour::Conversion for PaymentAttempt {
    type DstType = DieselPaymentAttempt;
    type NewDstType = DieselPaymentAttemptNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        use common_utils::encryption::Encryption;

        let card_network = self
            .payment_method_data
            .as_ref()
            .and_then(|data| data.peek().as_object())
            .and_then(|card| card.get("card"))
            .and_then(|data| data.as_object())
            .and_then(|card| card.get("card_network"))
            .and_then(|network| network.as_str())
            .map(|network| network.to_string());

        let Self {
            payment_id,
            merchant_id,
            status,
            error,
            amount_details,
            authentication_type,
            created_at,
            modified_at,
            last_synced,
            cancellation_reason,
            browser_info,
            payment_token,
            connector_metadata,
            payment_experience,
            payment_method_data,
            routing_result,
            preprocessing_step_id,
            multiple_capture_count,
            connector_response_reference_id,
            updated_by,
            authentication_data,
            encoded_data,
            merchant_connector_id,
            external_three_ds_authentication_attempted,
            authentication_connector,
            authentication_id,
            fingerprint_id,
            charge_id,
            client_source,
            client_version,
            customer_acceptance,
            profile_id,
            organization_id,
            payment_method_type,
            connector_payment_id,
            payment_method_subtype,
            authentication_applied,
            external_reference_id,
            id,
            payment_method_id,
            payment_method_billing_address,
            connector,
        } = self;

        let AttemptAmountDetails {
            net_amount,
            tax_on_surcharge,
            surcharge_amount,
            order_tax_amount,
            shipping_cost,
            amount_capturable,
            amount_to_capture,
        } = amount_details;

        Ok(DieselPaymentAttempt {
            payment_id,
            merchant_id,
            id,
            status,
            error_message: error.as_ref().map(|details| details.message.clone()),
            payment_method_id,
            payment_method_type_v2: payment_method_type,
            connector_payment_id,
            authentication_type,
            created_at,
            modified_at,
            last_synced,
            cancellation_reason,
            amount_to_capture,
            browser_info,
            error_code: error.as_ref().map(|details| details.code.clone()),
            payment_token,
            connector_metadata,
            payment_experience,
            payment_method_subtype,
            payment_method_data,
            preprocessing_step_id,
            error_reason: error.as_ref().and_then(|details| details.reason.clone()),
            multiple_capture_count,
            connector_response_reference_id,
            amount_capturable,
            updated_by,
            merchant_connector_id,
            authentication_data,
            encoded_data,
            unified_code: error
                .as_ref()
                .and_then(|details| details.unified_code.clone()),
            unified_message: error
                .as_ref()
                .and_then(|details| details.unified_message.clone()),
            net_amount,
            external_three_ds_authentication_attempted,
            authentication_connector,
            authentication_id,
            fingerprint_id,
            charge_id,
            client_source,
            client_version,
            customer_acceptance,
            profile_id,
            organization_id,
            card_network,
            order_tax_amount,
            shipping_cost,
            routing_result,
            authentication_applied,
            external_reference_id,
            connector,
            surcharge_amount,
            tax_on_surcharge,
            payment_method_billing_address: payment_method_billing_address.map(Encryption::from),
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        storage_model: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        async {
            let amount_details = AttemptAmountDetails {
                net_amount: storage_model.net_amount,
                tax_on_surcharge: storage_model.tax_on_surcharge,
                surcharge_amount: storage_model.surcharge_amount,
                order_tax_amount: storage_model.order_tax_amount,
                shipping_cost: storage_model.shipping_cost,
                amount_capturable: storage_model.amount_capturable,
                amount_to_capture: storage_model.amount_to_capture,
            };

            let inner_decrypt = |inner| async {
                crate::type_encryption::crypto_operation(
                    state,
                    common_utils::type_name!(Self::DstType),
                    crate::type_encryption::CryptoOperation::DecryptOptional(inner),
                    key_manager_identifier.clone(),
                    key.peek(),
                )
                .await
                .and_then(|val| val.try_into_optionaloperation())
            };

            let error = storage_model
                .error_code
                .zip(storage_model.error_message)
                .map(|(error_code, error_message)| ErrorDetails {
                    code: error_code,
                    message: error_message,
                    reason: storage_model.error_reason,
                    unified_code: storage_model.unified_code,
                    unified_message: storage_model.unified_message,
                });

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                payment_id: storage_model.payment_id,
                merchant_id: storage_model.merchant_id,
                id: storage_model.id,
                status: storage_model.status,
                amount_details,
                error,
                payment_method_id: storage_model.payment_method_id,
                payment_method_type: storage_model.payment_method_type_v2,
                connector_payment_id: storage_model.connector_payment_id,
                authentication_type: storage_model.authentication_type,
                created_at: storage_model.created_at,
                modified_at: storage_model.modified_at,
                last_synced: storage_model.last_synced,
                cancellation_reason: storage_model.cancellation_reason,
                browser_info: storage_model.browser_info,
                payment_token: storage_model.payment_token,
                connector_metadata: storage_model.connector_metadata,
                payment_experience: storage_model.payment_experience,
                payment_method_data: storage_model.payment_method_data,
                routing_result: storage_model.routing_result,
                preprocessing_step_id: storage_model.preprocessing_step_id,
                multiple_capture_count: storage_model.multiple_capture_count,
                connector_response_reference_id: storage_model.connector_response_reference_id,
                updated_by: storage_model.updated_by,
                authentication_data: storage_model.authentication_data,
                encoded_data: storage_model.encoded_data,
                merchant_connector_id: storage_model.merchant_connector_id,
                external_three_ds_authentication_attempted: storage_model
                    .external_three_ds_authentication_attempted,
                authentication_connector: storage_model.authentication_connector,
                authentication_id: storage_model.authentication_id,
                fingerprint_id: storage_model.fingerprint_id,
                charge_id: storage_model.charge_id,
                client_source: storage_model.client_source,
                client_version: storage_model.client_version,
                customer_acceptance: storage_model.customer_acceptance,
                profile_id: storage_model.profile_id,
                organization_id: storage_model.organization_id,
                payment_method_subtype: storage_model.payment_method_subtype,
                authentication_applied: storage_model.authentication_applied,
                external_reference_id: storage_model.external_reference_id,
                connector: storage_model.connector,
                payment_method_billing_address: inner_decrypt(
                    storage_model.payment_method_billing_address,
                )
                .await?,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment attempt".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        use common_utils::encryption::Encryption;

        let card_network = self
            .payment_method_data
            .as_ref()
            .and_then(|data| data.peek().as_object())
            .and_then(|card| card.get("card"))
            .and_then(|data| data.as_object())
            .and_then(|card| card.get("card_network"))
            .and_then(|network| network.as_str())
            .map(|network| network.to_string());

        let error_details = self.error;

        Ok(DieselPaymentAttemptNew {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            error_message: error_details
                .as_ref()
                .map(|details| details.message.clone()),
            surcharge_amount: self.amount_details.surcharge_amount,
            tax_on_surcharge: self.amount_details.tax_on_surcharge,
            payment_method_id: self.payment_method_id,
            authentication_type: self.authentication_type,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            cancellation_reason: self.cancellation_reason,
            browser_info: self.browser_info,
            payment_token: self.payment_token,
            error_code: error_details.as_ref().map(|details| details.code.clone()),
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_data: self.payment_method_data,
            preprocessing_step_id: self.preprocessing_step_id,
            error_reason: error_details
                .as_ref()
                .and_then(|details| details.reason.clone()),
            connector_response_reference_id: self.connector_response_reference_id,
            multiple_capture_count: self.multiple_capture_count,
            amount_capturable: self.amount_details.amount_capturable,
            updated_by: self.updated_by,
            merchant_connector_id: self.merchant_connector_id,
            authentication_data: self.authentication_data,
            encoded_data: self.encoded_data,
            unified_code: error_details
                .as_ref()
                .and_then(|details| details.unified_code.clone()),
            unified_message: error_details
                .as_ref()
                .and_then(|details| details.unified_message.clone()),
            net_amount: self.amount_details.net_amount,
            external_three_ds_authentication_attempted: self
                .external_three_ds_authentication_attempted,
            authentication_connector: self.authentication_connector,
            authentication_id: self.authentication_id,
            fingerprint_id: self.fingerprint_id,
            charge_id: self.charge_id,
            client_source: self.client_source,
            client_version: self.client_version,
            customer_acceptance: self.customer_acceptance,
            profile_id: self.profile_id,
            organization_id: self.organization_id,
            card_network,
            order_tax_amount: self.amount_details.order_tax_amount,
            shipping_cost: self.amount_details.shipping_cost,
            amount_to_capture: self.amount_details.amount_to_capture,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(Encryption::from),
            payment_method_subtype: self.payment_method_subtype,
            payment_method_type_v2: self.payment_method_type,
            id: self.id,
        })
    }
}

#[cfg(feature = "v2")]
impl From<PaymentAttemptUpdate> for diesel_models::PaymentAttemptUpdateInternal {
    fn from(update: PaymentAttemptUpdate) -> Self {
        match update {
            PaymentAttemptUpdate::ConfirmIntent {
                status,
                updated_by,
                connector,
                merchant_connector_id,
            } => diesel_models::PaymentAttemptUpdateInternal {
                status: Some(status),
                error_message: None,
                modified_at: common_utils::date_time::now(),
                browser_info: None,
                error_code: None,
                error_reason: None,
                updated_by,
                merchant_connector_id: Some(merchant_connector_id),
                unified_code: None,
                unified_message: None,
                connector_payment_id: None,
                connector: Some(connector),
            },
            PaymentAttemptUpdate::ConfirmIntentError {
                status,
                error,
                updated_by,
            } => diesel_models::PaymentAttemptUpdateInternal {
                status: Some(status),
                error_message: Some(error.message),
                error_code: Some(error.code),
                modified_at: common_utils::date_time::now(),
                browser_info: None,
                error_reason: error.reason,
                updated_by,
                merchant_connector_id: None,
                unified_code: None,
                unified_message: None,
                connector_payment_id: None,
                connector: None,
            },
            PaymentAttemptUpdate::ConfirmIntentResponse {
                status,
                connector_payment_id,
                updated_by,
            } => diesel_models::PaymentAttemptUpdateInternal {
                status: Some(status),
                error_message: None,
                error_code: None,
                modified_at: common_utils::date_time::now(),
                browser_info: None,
                error_reason: None,
                updated_by,
                merchant_connector_id: None,
                unified_code: None,
                unified_message: None,
                connector_payment_id,
                connector: None,
            },
        }
    }
}
