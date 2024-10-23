#[cfg(all(feature = "v1", feature = "olap"))]
use api_models::enums::Connector;
use common_enums as storage_enums;
use common_utils::{
    errors::{CustomResult, ValidationError},
    id_type, pii,
    types::{
        keymanager::{self, KeyManagerState},
        ConnectorTransactionId, ConnectorTransactionIdTrait, MinorUnit,
    },
};
use diesel_models::{
    ConnectorMandateReferenceId, PaymentAttempt as DieselPaymentAttempt,
    PaymentAttemptNew as DieselPaymentAttemptNew,
    PaymentAttemptUpdate as DieselPaymentAttemptUpdate,
};
use error_stack::ResultExt;
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
    router_request_types, ForeignIDRef,
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
    async fn update_payment_attempt_with_attempt_id(
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
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<i64, errors::StorageError>;
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PaymentAttempt {
    pub payment_id: id_type::GlobalPaymentId,
    pub merchant_id: id_type::MerchantId,
    pub status: storage_enums::AttemptStatus,
    pub net_amount: MinorUnit,
    pub connector: Option<String>,
    pub amount_to_capture: Option<MinorUnit>,
    pub error_message: Option<String>,
    pub surcharge_amount: Option<MinorUnit>,
    pub tax_on_surcharge: Option<MinorUnit>,
    pub confirm: bool,
    pub authentication_type: Option<storage_enums::AuthenticationType>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub cancellation_reason: Option<String>,
    pub browser_info: Option<serde_json::Value>,
    pub error_code: Option<String>,
    pub payment_token: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    pub payment_method_data: Option<serde_json::Value>,
    pub routing_result: Option<serde_json::Value>,
    pub preprocessing_step_id: Option<String>,
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
    pub payment_method_billing_address_id: Option<String>,
    pub fingerprint_id: Option<String>,
    pub charge_id: Option<String>,
    pub client_source: Option<String>,
    pub client_version: Option<String>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub profile_id: id_type::ProfileId,
    pub organization_id: id_type::OrganizationId,
    pub payment_method_type: Option<storage_enums::PaymentMethod>,
    pub payment_method_id: Option<String>,
    pub connector_payment_id: Option<String>,
    pub payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    pub authentication_applied: Option<common_enums::AuthenticationType>,
    pub external_reference_id: Option<String>,
    pub shipping_cost: Option<MinorUnit>,
    pub order_tax_amount: Option<MinorUnit>,
    pub id: String,
    pub connector_mandate_detail: Option<ConnectorMandateReferenceId>,
}

impl PaymentAttempt {
    #[cfg(feature = "v1")]
    pub fn get_payment_method(&self) -> Option<storage_enums::PaymentMethod> {
        self.payment_method
    }

    #[cfg(feature = "v2")]
    pub fn get_payment_method(&self) -> Option<storage_enums::PaymentMethod> {
        self.payment_method_type
    }

    #[cfg(feature = "v1")]
    pub fn get_payment_method_type(&self) -> Option<storage_enums::PaymentMethodType> {
        self.payment_method_type
    }

    #[cfg(feature = "v2")]
    pub fn get_payment_method_type(&self) -> Option<storage_enums::PaymentMethodType> {
        self.payment_method_subtype
    }

    #[cfg(feature = "v1")]
    pub fn get_id(&self) -> &str {
        &self.attempt_id
    }

    #[cfg(feature = "v2")]
    pub fn get_id(&self) -> &str {
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
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PaymentAttempt {
    pub payment_id: id_type::PaymentId,
    pub merchant_id: id_type::MerchantId,
    pub attempt_id: String,
    pub status: storage_enums::AttemptStatus,
    pub net_amount: NetAmount,
    pub currency: Option<storage_enums::Currency>,
    pub save_to_locker: Option<bool>,
    pub connector: Option<String>,
    pub error_message: Option<String>,
    pub offer_amount: Option<MinorUnit>,
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
    pub connector_mandate_detail: Option<ConnectorMandateReferenceId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct NetAmount {
    /// The payment amount
    order_amount: MinorUnit,
    /// The shipping cost of the order
    shipping_cost: Option<MinorUnit>,
    /// Tax amount related to the order
    order_tax_amount: Option<MinorUnit>,
    /// The surcharge amount to be added to the order
    surcharge_amount: Option<MinorUnit>,
    /// tax on surcharge amount
    tax_on_surcharge: Option<MinorUnit>,
}

impl NetAmount {
    pub fn new(
        order_amount: MinorUnit,
        shipping_cost: Option<MinorUnit>,
        order_tax_amount: Option<MinorUnit>,
        surcharge_amount: Option<MinorUnit>,
        tax_on_surcharge: Option<MinorUnit>,
    ) -> Self {
        Self {
            order_amount,
            shipping_cost,
            order_tax_amount,
            surcharge_amount,
            tax_on_surcharge,
        }
    }

    pub fn get_order_amount(&self) -> MinorUnit {
        self.order_amount
    }

    pub fn get_shipping_cost(&self) -> Option<MinorUnit> {
        self.shipping_cost
    }

    pub fn get_order_tax_amount(&self) -> Option<MinorUnit> {
        self.order_tax_amount
    }

    pub fn get_surcharge_amount(&self) -> Option<MinorUnit> {
        self.surcharge_amount
    }

    pub fn get_tax_on_surcharge(&self) -> Option<MinorUnit> {
        self.tax_on_surcharge
    }

    pub fn get_total_surcharge_amount(&self) -> Option<MinorUnit> {
        self.surcharge_amount
            .map(|surcharge_amount| surcharge_amount + self.tax_on_surcharge.unwrap_or_default())
    }

    pub fn get_total_amount(&self) -> MinorUnit {
        self.order_amount
            + self.shipping_cost.unwrap_or_default()
            + self.order_tax_amount.unwrap_or_default()
            + self.surcharge_amount.unwrap_or_default()
            + self.tax_on_surcharge.unwrap_or_default()
    }

    pub fn set_order_amount(&mut self, order_amount: MinorUnit) {
        self.order_amount = order_amount;
    }

    pub fn set_order_tax_amount(&mut self, order_tax_amount: Option<MinorUnit>) {
        self.order_tax_amount = order_tax_amount;
    }

    pub fn set_surcharge_details(
        &mut self,
        surcharge_details: Option<router_request_types::SurchargeDetails>,
    ) {
        self.surcharge_amount = surcharge_details
            .clone()
            .map(|details| details.surcharge_amount);
        self.tax_on_surcharge = surcharge_details.map(|details| details.tax_on_surcharge_amount);
    }

    pub fn from_payments_request(
        payments_request: &api_models::payments::PaymentsRequest,
        order_amount: MinorUnit,
    ) -> Self {
        let surcharge_amount = payments_request
            .surcharge_details
            .map(|surcharge_details| surcharge_details.surcharge_amount);
        let tax_on_surcharge = payments_request
            .surcharge_details
            .and_then(|surcharge_details| surcharge_details.tax_amount);
        Self {
            order_amount,
            shipping_cost: payments_request.shipping_cost,
            order_tax_amount: None,
            surcharge_amount,
            tax_on_surcharge,
        }
    }

    #[cfg(feature = "v1")]
    pub fn from_payments_request_and_payment_attempt(
        payments_request: &api_models::payments::PaymentsRequest,
        payment_attempt: Option<&PaymentAttempt>,
    ) -> Option<Self> {
        let option_order_amount = payments_request
            .amount
            .map(MinorUnit::from)
            .or(payment_attempt
                .map(|payment_attempt| payment_attempt.net_amount.get_order_amount()));
        option_order_amount.map(|order_amount| {
            let shipping_cost = payments_request.shipping_cost.or(payment_attempt
                .and_then(|payment_attempt| payment_attempt.net_amount.get_shipping_cost()));
            let order_tax_amount = payment_attempt
                .and_then(|payment_attempt| payment_attempt.net_amount.get_order_tax_amount());
            let surcharge_amount = payments_request
                .surcharge_details
                .map(|surcharge_details| surcharge_details.get_surcharge_amount())
                .or_else(|| {
                    payment_attempt.and_then(|payment_attempt| {
                        payment_attempt.net_amount.get_surcharge_amount()
                    })
                });
            let tax_on_surcharge = payments_request
                .surcharge_details
                .and_then(|surcharge_details| surcharge_details.get_tax_amount())
                .or_else(|| {
                    payment_attempt.and_then(|payment_attempt| {
                        payment_attempt.net_amount.get_tax_on_surcharge()
                    })
                });
            Self {
                order_amount,
                shipping_cost,
                order_tax_amount,
                surcharge_amount,
                tax_on_surcharge,
            }
        })
    }
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
        self.net_amount.get_total_amount()
    }

    pub fn get_total_surcharge_amount(&self) -> Option<MinorUnit> {
        self.net_amount.get_total_surcharge_amount()
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

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentAttemptNew {
    pub payment_id: id_type::PaymentId,
    pub merchant_id: id_type::MerchantId,
    pub status: storage_enums::AttemptStatus,
    pub error_message: Option<String>,
    pub surcharge_amount: Option<MinorUnit>,
    pub tax_amount: Option<MinorUnit>,
    pub payment_method_id: Option<String>,
    pub confirm: bool,
    pub authentication_type: Option<storage_enums::AuthenticationType>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub last_synced: Option<PrimitiveDateTime>,
    pub cancellation_reason: Option<String>,
    pub browser_info: Option<serde_json::Value>,
    pub payment_token: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    pub payment_method_data: Option<serde_json::Value>,
    pub straight_through_algorithm: Option<serde_json::Value>,
    pub preprocessing_step_id: Option<String>,
    pub error_reason: Option<String>,
    pub connector_response_reference_id: Option<String>,
    pub multiple_capture_count: Option<i16>,
    pub amount_capturable: MinorUnit,
    pub updated_by: String,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub net_amount: Option<MinorUnit>,
    pub external_three_ds_authentication_attempted: Option<bool>,
    pub authentication_connector: Option<String>,
    pub authentication_id: Option<String>,
    pub fingerprint_id: Option<String>,
    pub payment_method_billing_address_id: Option<String>,
    pub charge_id: Option<String>,
    pub client_source: Option<String>,
    pub client_version: Option<String>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub profile_id: id_type::ProfileId,
    pub organization_id: id_type::OrganizationId,
    pub card_network: Option<String>,
    pub shipping_cost: Option<MinorUnit>,
    pub order_tax_amount: Option<MinorUnit>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentAttemptNew {
    pub payment_id: id_type::PaymentId,
    pub merchant_id: id_type::MerchantId,
    pub attempt_id: String,
    pub status: storage_enums::AttemptStatus,
    /// amount + surcharge_amount + tax_amount
    /// This field will always be derived before updating in the Database
    pub net_amount: NetAmount,
    pub currency: Option<storage_enums::Currency>,
    // pub auto_capture: Option<bool>,
    pub save_to_locker: Option<bool>,
    pub connector: Option<String>,
    pub error_message: Option<String>,
    pub offer_amount: Option<MinorUnit>,
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
    pub connector_mandate_detail: Option<ConnectorMandateReferenceId>,
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentAttemptUpdate {
    Update {
        net_amount: NetAmount,
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
        net_amount: NetAmount,
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
        connector_mandate_detail: Option<ConnectorMandateReferenceId>,
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
        net_amount: NetAmount,
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
    PostSessionTokensUpdate {
        updated_by: String,
        connector_metadata: Option<serde_json::Value>,
    },
}

#[cfg(feature = "v1")]
impl PaymentAttemptUpdate {
    pub fn to_storage_model(self) -> diesel_models::PaymentAttemptUpdate {
        match self {
            Self::Update {
                net_amount,
                currency,
                status,
                authentication_type,
                payment_method,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                amount_to_capture,
                capture_method,
                fingerprint_id,
                payment_method_billing_address_id,
                updated_by,
            } => DieselPaymentAttemptUpdate::Update {
                amount: net_amount.get_order_amount(),
                currency,
                status,
                authentication_type,
                payment_method,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                amount_to_capture,
                capture_method,
                surcharge_amount: net_amount.get_surcharge_amount(),
                tax_amount: net_amount.get_tax_on_surcharge(),
                fingerprint_id,
                payment_method_billing_address_id,
                updated_by,
            },
            Self::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
                updated_by,
                surcharge_amount,
                tax_amount,
                merchant_connector_id,
            } => DieselPaymentAttemptUpdate::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
                surcharge_amount,
                tax_amount,
                updated_by,
                merchant_connector_id,
            },
            Self::AuthenticationTypeUpdate {
                authentication_type,
                updated_by,
            } => DieselPaymentAttemptUpdate::AuthenticationTypeUpdate {
                authentication_type,
                updated_by,
            },
            Self::BlocklistUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            } => DieselPaymentAttemptUpdate::BlocklistUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            },
            Self::PaymentMethodDetailsUpdate {
                payment_method_id,
                updated_by,
            } => DieselPaymentAttemptUpdate::PaymentMethodDetailsUpdate {
                payment_method_id,
                updated_by,
            },
            Self::ConfirmUpdate {
                net_amount,
                currency,
                status,
                authentication_type,
                capture_method,
                payment_method,
                browser_info,
                connector,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                straight_through_algorithm,
                error_code,
                error_message,
                amount_capturable,
                fingerprint_id,
                updated_by,
                merchant_connector_id: connector_id,
                payment_method_id,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                payment_method_billing_address_id,
                client_source,
                client_version,
                customer_acceptance,
            } => DieselPaymentAttemptUpdate::ConfirmUpdate {
                amount: net_amount.get_order_amount(),
                currency,
                status,
                authentication_type,
                capture_method,
                payment_method,
                browser_info,
                connector,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                straight_through_algorithm,
                error_code,
                error_message,
                amount_capturable,
                surcharge_amount: net_amount.get_surcharge_amount(),
                tax_amount: net_amount.get_tax_on_surcharge(),
                fingerprint_id,
                updated_by,
                merchant_connector_id: connector_id,
                payment_method_id,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                payment_method_billing_address_id,
                client_source,
                client_version,
                customer_acceptance,
                shipping_cost: net_amount.get_shipping_cost(),
                order_tax_amount: net_amount.get_order_tax_amount(),
            },
            Self::VoidUpdate {
                status,
                cancellation_reason,
                updated_by,
            } => DieselPaymentAttemptUpdate::VoidUpdate {
                status,
                cancellation_reason,
                updated_by,
            },
            Self::ResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                mandate_id,
                connector_metadata,
                payment_token,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                amount_capturable,
                updated_by,
                authentication_data,
                encoded_data,
                unified_code,
                unified_message,
                payment_method_data,
                charge_id,
                connector_mandate_detail,
            } => DieselPaymentAttemptUpdate::ResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                mandate_id,
                connector_metadata,
                payment_token,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                amount_capturable,
                updated_by,
                authentication_data,
                encoded_data,
                unified_code,
                unified_message,
                payment_method_data,
                charge_id,
                connector_mandate_detail,
            },
            Self::UnresolvedResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                payment_method_id,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                updated_by,
            } => DieselPaymentAttemptUpdate::UnresolvedResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                payment_method_id,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                updated_by,
            },
            Self::StatusUpdate { status, updated_by } => {
                DieselPaymentAttemptUpdate::StatusUpdate { status, updated_by }
            }
            Self::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
                amount_capturable,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
                payment_method_data,
                authentication_type,
            } => DieselPaymentAttemptUpdate::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
                amount_capturable,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
                payment_method_data,
                authentication_type,
            },
            Self::CaptureUpdate {
                multiple_capture_count,
                updated_by,
                amount_to_capture,
            } => DieselPaymentAttemptUpdate::CaptureUpdate {
                multiple_capture_count,
                updated_by,
                amount_to_capture,
            },
            Self::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                updated_by,
            } => DieselPaymentAttemptUpdate::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                updated_by,
            },
            Self::RejectUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            } => DieselPaymentAttemptUpdate::RejectUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            },
            Self::AmountToCaptureUpdate {
                status,
                amount_capturable,
                updated_by,
            } => DieselPaymentAttemptUpdate::AmountToCaptureUpdate {
                status,
                amount_capturable,
                updated_by,
            },
            Self::ConnectorResponse {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                connector,
                charge_id,
                updated_by,
            } => DieselPaymentAttemptUpdate::ConnectorResponse {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                connector,
                charge_id,
                updated_by,
            },
            Self::IncrementalAuthorizationAmountUpdate {
                net_amount,
                amount_capturable,
            } => DieselPaymentAttemptUpdate::IncrementalAuthorizationAmountUpdate {
                amount: net_amount.get_order_amount(),
                amount_capturable,
            },
            Self::AuthenticationUpdate {
                status,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                updated_by,
            } => DieselPaymentAttemptUpdate::AuthenticationUpdate {
                status,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                updated_by,
            },
            Self::ManualUpdate {
                status,
                error_code,
                error_message,
                error_reason,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
            } => DieselPaymentAttemptUpdate::ManualUpdate {
                status,
                error_code,
                error_message,
                error_reason,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
            },
            Self::PostSessionTokensUpdate {
                updated_by,
                connector_metadata,
            } => DieselPaymentAttemptUpdate::PostSessionTokensUpdate {
                updated_by,
                connector_metadata,
            },
        }
    }
}

// TODO: Add fields as necessary
#[cfg(feature = "v2")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentAttemptUpdate {}

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
        let (connector_transaction_id, connector_transaction_data) = self
            .connector_transaction_id
            .map(ConnectorTransactionId::form_id_and_data)
            .map(|(txn_id, txn_data)| (Some(txn_id), txn_data))
            .unwrap_or((None, None));
        Ok(DieselPaymentAttempt {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.net_amount.get_order_amount(),
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.net_amount.get_surcharge_amount(),
            tax_amount: self.net_amount.get_tax_on_surcharge(),
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            connector_transaction_id,
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
            net_amount: Some(self.net_amount.get_total_amount()),
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
            connector_transaction_data,
            order_tax_amount: self.net_amount.get_order_tax_amount(),
            shipping_cost: self.net_amount.get_shipping_cost(),
            connector_mandate_detail: self.connector_mandate_detail,
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
            let connector_transaction_id = storage_model
                .get_optional_connector_transaction_id()
                .cloned();
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                payment_id: storage_model.payment_id,
                merchant_id: storage_model.merchant_id,
                attempt_id: storage_model.attempt_id,
                status: storage_model.status,
                net_amount: NetAmount::new(
                    storage_model.amount,
                    storage_model.shipping_cost,
                    storage_model.order_tax_amount,
                    storage_model.surcharge_amount,
                    storage_model.tax_amount,
                ),
                currency: storage_model.currency,
                save_to_locker: storage_model.save_to_locker,
                connector: storage_model.connector,
                error_message: storage_model.error_message,
                offer_amount: storage_model.offer_amount,
                payment_method_id: storage_model.payment_method_id,
                payment_method: storage_model.payment_method,
                connector_transaction_id,
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
                connector_mandate_detail: storage_model.connector_mandate_detail,
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
            amount: self.net_amount.get_order_amount(),
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.net_amount.get_surcharge_amount(),
            tax_amount: self.net_amount.get_tax_on_surcharge(),
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
            net_amount: Some(self.net_amount.get_total_amount()),
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
            order_tax_amount: self.net_amount.get_order_tax_amount(),
            shipping_cost: self.net_amount.get_shipping_cost(),
            connector_mandate_detail: self.connector_mandate_detail,
        })
    }
}

#[cfg(feature = "v2")]
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

        let Self {
            payment_id,
            merchant_id,
            status,
            net_amount,
            error_message,
            surcharge_amount,
            tax_on_surcharge,
            confirm,
            authentication_type,
            created_at,
            modified_at,
            last_synced,
            cancellation_reason,
            browser_info,
            error_code,
            payment_token,
            connector_metadata,
            payment_experience,
            payment_method_data,
            routing_result,
            preprocessing_step_id,
            error_reason,
            multiple_capture_count,
            connector_response_reference_id,
            amount_capturable,
            updated_by,
            authentication_data,
            encoded_data,
            merchant_connector_id,
            unified_code,
            unified_message,
            external_three_ds_authentication_attempted,
            authentication_connector,
            authentication_id,
            payment_method_billing_address_id,
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
            amount_to_capture,
            payment_method_id,
            shipping_cost,
            order_tax_amount,
            connector,
            connector_mandate_detail,
        } = self;

        let (connector_payment_id, connector_payment_data) = connector_payment_id
            .map(ConnectorTransactionId::form_id_and_data)
            .map(|(txn_id, txn_data)| (Some(txn_id), txn_data))
            .unwrap_or((None, None));

        Ok(DieselPaymentAttempt {
            payment_id,
            merchant_id,
            id,
            status,
            error_message,
            surcharge_amount,
            tax_on_surcharge,
            payment_method_id,
            payment_method_type_v2: payment_method_type,
            connector_payment_id,
            confirm,
            authentication_type,
            created_at,
            modified_at,
            last_synced,
            cancellation_reason,
            amount_to_capture,
            browser_info,
            error_code,
            payment_token,
            connector_metadata,
            payment_experience,
            payment_method_subtype,
            payment_method_data,
            preprocessing_step_id,
            error_reason,
            multiple_capture_count,
            connector_response_reference_id,
            amount_capturable,
            updated_by,
            merchant_connector_id,
            authentication_data,
            encoded_data,
            unified_code,
            unified_message,
            net_amount: Some(net_amount),
            external_three_ds_authentication_attempted,
            authentication_connector,
            authentication_id,
            fingerprint_id,
            payment_method_billing_address_id,
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
            connector_payment_data,
            connector_mandate_detail,
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
            let connector_payment_id = storage_model
                .get_optional_connector_transaction_id()
                .cloned();
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                payment_id: storage_model.payment_id,
                merchant_id: storage_model.merchant_id,
                id: storage_model.id,
                status: storage_model.status,
                net_amount: storage_model.net_amount.unwrap_or(MinorUnit::new(0)),
                tax_on_surcharge: storage_model.tax_on_surcharge,
                error_message: storage_model.error_message,
                surcharge_amount: storage_model.surcharge_amount,
                payment_method_id: storage_model.payment_method_id,
                payment_method_type: storage_model.payment_method_type_v2,
                connector_payment_id,
                confirm: storage_model.confirm,
                authentication_type: storage_model.authentication_type,
                created_at: storage_model.created_at,
                modified_at: storage_model.modified_at,
                last_synced: storage_model.last_synced,
                cancellation_reason: storage_model.cancellation_reason,
                amount_to_capture: storage_model.amount_to_capture,
                browser_info: storage_model.browser_info,
                error_code: storage_model.error_code,
                payment_token: storage_model.payment_token,
                connector_metadata: storage_model.connector_metadata,
                payment_experience: storage_model.payment_experience,
                payment_method_data: storage_model.payment_method_data,
                routing_result: storage_model.routing_result,
                preprocessing_step_id: storage_model.preprocessing_step_id,
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
                payment_method_subtype: storage_model.payment_method_subtype,
                authentication_applied: storage_model.authentication_applied,
                external_reference_id: storage_model.external_reference_id,
                connector: storage_model.connector,
                connector_mandate_detail: storage_model.connector_mandate_detail,
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
            status: self.status,
            error_message: self.error_message,
            surcharge_amount: self.surcharge_amount,
            tax_on_surcharge: self.tax_on_surcharge,
            payment_method_id: self.payment_method_id,
            confirm: self.confirm,
            authentication_type: self.authentication_type,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            cancellation_reason: self.cancellation_reason,
            browser_info: self.browser_info,
            payment_token: self.payment_token,
            error_code: self.error_code,
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_data: self.payment_method_data,
            preprocessing_step_id: self.preprocessing_step_id,
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
            amount_to_capture: self.amount_to_capture,
            connector_mandate_detail: self.connector_mandate_detail,
        })
    }
}

#[cfg(feature = "v2")]
impl From<PaymentAttemptUpdate> for diesel_models::PaymentAttemptUpdateInternal {
    fn from(update: PaymentAttemptUpdate) -> Self {
        todo!()
    }
}
