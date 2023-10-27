use api_models::enums::Connector;
use common_enums as storage_enums;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::PaymentIntent;
use crate::{errors, mandates::MandateDataType, ForeignIDRef};

#[async_trait::async_trait]
pub trait PaymentAttemptInterface {
    async fn insert_payment_attempt(
        &self,
        payment_attempt: PaymentAttemptNew,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn update_payment_attempt_with_attempt_id(
        &self,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        connector_transaction_id: &str,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &str,
        connector_txn_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        attempt_id: &str,
        merchant_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        preprocessing_id: &str,
        merchant_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentAttempt>, errors::StorageError>;

    async fn get_filters_for_payments(
        &self,
        pi: &[PaymentIntent],
        merchant_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentListFilters, errors::StorageError>;

    #[allow(clippy::too_many_arguments)]
    async fn get_total_count_of_filtered_payment_attempts(
        &self,
        merchant_id: &str,
        active_attempt_ids: &[String],
        connector: Option<Vec<Connector>>,
        payment_method: Option<Vec<storage_enums::PaymentMethod>>,
        payment_method_type: Option<Vec<storage_enums::PaymentMethodType>>,
        authentication_type: Option<Vec<storage_enums::AuthenticationType>>,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<i64, errors::StorageError>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PaymentAttempt {
    pub id: i32,
    pub payment_id: String,
    pub merchant_id: String,
    pub attempt_id: String,
    pub status: storage_enums::AttemptStatus,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub save_to_locker: Option<bool>,
    pub connector: Option<String>,
    pub error_message: Option<String>,
    pub offer_amount: Option<i64>,
    pub surcharge_amount: Option<i64>,
    pub tax_amount: Option<i64>,
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
    pub amount_to_capture: Option<i64>,
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
    pub amount_capturable: i64,
    pub updated_by: String,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
    pub merchant_connector_id: Option<String>,
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PaymentAttemptNew {
    pub payment_id: String,
    pub merchant_id: String,
    pub attempt_id: String,
    pub status: storage_enums::AttemptStatus,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    // pub auto_capture: Option<bool>,
    pub save_to_locker: Option<bool>,
    pub connector: Option<String>,
    pub error_message: Option<String>,
    pub offer_amount: Option<i64>,
    pub surcharge_amount: Option<i64>,
    pub tax_amount: Option<i64>,
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
    pub amount_to_capture: Option<i64>,
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
    pub amount_capturable: i64,
    pub updated_by: String,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
    pub merchant_connector_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentAttemptUpdate {
    Update {
        amount: i64,
        currency: storage_enums::Currency,
        status: storage_enums::AttemptStatus,
        authentication_type: Option<storage_enums::AuthenticationType>,
        payment_method: Option<storage_enums::PaymentMethod>,
        payment_token: Option<String>,
        payment_method_data: Option<serde_json::Value>,
        payment_method_type: Option<storage_enums::PaymentMethodType>,
        payment_experience: Option<storage_enums::PaymentExperience>,
        business_sub_label: Option<String>,
        amount_to_capture: Option<i64>,
        capture_method: Option<storage_enums::CaptureMethod>,
        updated_by: String,
    },
    UpdateTrackers {
        payment_token: Option<String>,
        connector: Option<String>,
        straight_through_algorithm: Option<serde_json::Value>,
        amount_capturable: Option<i64>,
        updated_by: String,
        merchant_connector_id: Option<String>,
    },
    AuthenticationTypeUpdate {
        authentication_type: storage_enums::AuthenticationType,
        updated_by: String,
    },
    ConfirmUpdate {
        amount: i64,
        currency: storage_enums::Currency,
        status: storage_enums::AttemptStatus,
        authentication_type: Option<storage_enums::AuthenticationType>,
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
        amount_capturable: Option<i64>,
        surcharge_amount: Option<i64>,
        tax_amount: Option<i64>,
        updated_by: String,
        merchant_connector_id: Option<String>,
    },
    RejectUpdate {
        status: storage_enums::AttemptStatus,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
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
        payment_method_id: Option<Option<String>>,
        mandate_id: Option<String>,
        connector_metadata: Option<serde_json::Value>,
        payment_token: Option<String>,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
        error_reason: Option<Option<String>>,
        connector_response_reference_id: Option<String>,
        amount_capturable: Option<i64>,
        updated_by: String,
    },
    UnresolvedResponseUpdate {
        status: storage_enums::AttemptStatus,
        connector: Option<String>,
        connector_transaction_id: Option<String>,
        payment_method_id: Option<Option<String>>,
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
        amount_capturable: Option<i64>,
        updated_by: String,
    },
    MultipleCaptureCountUpdate {
        multiple_capture_count: i16,
        updated_by: String,
    },
    AmountToCaptureUpdate {
        status: storage_enums::AttemptStatus,
        amount_capturable: i64,
        updated_by: String,
    },
    PreprocessingUpdate {
        status: storage_enums::AttemptStatus,
        payment_method_id: Option<Option<String>>,
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
        updated_by: String,
    },
}

impl ForeignIDRef for PaymentAttempt {
    fn foreign_id(&self) -> String {
        self.attempt_id.clone()
    }
}
