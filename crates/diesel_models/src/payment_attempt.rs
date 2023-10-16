use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    enums::{self as storage_enums},
    schema::payment_attempt,
};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payment_attempt)]
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
    pub mandate_details: Option<storage_enums::MandateDataType>,
    pub error_reason: Option<String>,
    pub multiple_capture_count: Option<i16>,
    // reference to the payment at connector side
    pub connector_response_reference_id: Option<String>,
    pub amount_capturable: i64,
    pub surcharge_metadata: Option<serde_json::Value>,
    pub updated_by: storage_enums::MerchantStorageScheme,
}

#[derive(Clone, Debug, Eq, PartialEq, Queryable, Serialize, Deserialize)]
pub struct PaymentListFilters {
    pub connector: Vec<String>,
    pub currency: Vec<storage_enums::Currency>,
    pub status: Vec<storage_enums::IntentStatus>,
    pub payment_method: Vec<storage_enums::PaymentMethod>,
}
#[derive(
    Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize,
)]
#[diesel(table_name = payment_attempt)]
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
    pub mandate_details: Option<storage_enums::MandateDataType>,
    pub error_reason: Option<String>,
    pub connector_response_reference_id: Option<String>,
    pub multiple_capture_count: Option<i16>,
    pub amount_capturable: i64,
    pub surcharge_metadata: Option<serde_json::Value>,
    pub updated_by: storage_enums::MerchantStorageScheme,
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
    },
    UpdateTrackers {
        payment_token: Option<String>,
        connector: Option<String>,
        straight_through_algorithm: Option<serde_json::Value>,
        amount_capturable: Option<i64>,
    },
    AuthenticationTypeUpdate {
        authentication_type: storage_enums::AuthenticationType,
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
    },
    VoidUpdate {
        status: storage_enums::AttemptStatus,
        cancellation_reason: Option<String>,
    },
    RejectUpdate {
        status: storage_enums::AttemptStatus,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
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
    },
    StatusUpdate {
        status: storage_enums::AttemptStatus,
    },
    ErrorUpdate {
        connector: Option<String>,
        status: storage_enums::AttemptStatus,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
        error_reason: Option<Option<String>>,
        amount_capturable: Option<i64>,
    },
    MultipleCaptureCountUpdate {
        multiple_capture_count: i16,
    },
    AmountToCaptureUpdate {
        status: storage_enums::AttemptStatus,
        amount_capturable: i64,
    },
    SurchargeAmountUpdate {
        surcharge_amount: Option<i64>,
        tax_amount: Option<i64>,
    },
    PreprocessingUpdate {
        status: storage_enums::AttemptStatus,
        payment_method_id: Option<Option<String>>,
        connector_metadata: Option<serde_json::Value>,
        preprocessing_step_id: Option<String>,
        connector_transaction_id: Option<String>,
        connector_response_reference_id: Option<String>,
    },
    SurchargeMetadataUpdate {
        surcharge_metadata: Option<serde_json::Value>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payment_attempt)]
pub struct PaymentAttemptUpdateInternal {
    amount: Option<i64>,
    currency: Option<storage_enums::Currency>,
    status: Option<storage_enums::AttemptStatus>,
    connector_transaction_id: Option<String>,
    amount_to_capture: Option<i64>,
    connector: Option<String>,
    authentication_type: Option<storage_enums::AuthenticationType>,
    payment_method: Option<storage_enums::PaymentMethod>,
    error_message: Option<Option<String>>,
    payment_method_id: Option<Option<String>>,
    cancellation_reason: Option<String>,
    modified_at: Option<PrimitiveDateTime>,
    mandate_id: Option<String>,
    browser_info: Option<serde_json::Value>,
    payment_token: Option<String>,
    error_code: Option<Option<String>>,
    connector_metadata: Option<serde_json::Value>,
    payment_method_data: Option<serde_json::Value>,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    payment_experience: Option<storage_enums::PaymentExperience>,
    business_sub_label: Option<String>,
    straight_through_algorithm: Option<serde_json::Value>,
    preprocessing_step_id: Option<String>,
    error_reason: Option<Option<String>>,
    capture_method: Option<storage_enums::CaptureMethod>,
    connector_response_reference_id: Option<String>,
    multiple_capture_count: Option<i16>,
    surcharge_amount: Option<i64>,
    tax_amount: Option<i64>,
    amount_capturable: Option<i64>,
    surcharge_metadata: Option<serde_json::Value>,
}

impl PaymentAttemptUpdate {
    pub fn apply_changeset(self, source: PaymentAttempt) -> PaymentAttempt {
        let pa_update: PaymentAttemptUpdateInternal = self.into();
        PaymentAttempt {
            amount: pa_update.amount.unwrap_or(source.amount),
            currency: pa_update.currency.or(source.currency),
            status: pa_update.status.unwrap_or(source.status),
            connector: pa_update.connector.or(source.connector),
            connector_transaction_id: source
                .connector_transaction_id
                .or(pa_update.connector_transaction_id),
            authentication_type: pa_update.authentication_type.or(source.authentication_type),
            payment_method: pa_update.payment_method.or(source.payment_method),
            error_message: pa_update.error_message.unwrap_or(source.error_message),
            payment_method_id: pa_update
                .payment_method_id
                .unwrap_or(source.payment_method_id),
            browser_info: pa_update.browser_info.or(source.browser_info),
            modified_at: common_utils::date_time::now(),
            payment_token: pa_update.payment_token.or(source.payment_token),
            connector_metadata: pa_update.connector_metadata.or(source.connector_metadata),
            preprocessing_step_id: pa_update
                .preprocessing_step_id
                .or(source.preprocessing_step_id),
            surcharge_metadata: pa_update.surcharge_metadata.or(source.surcharge_metadata),
            ..source
        }
    }
}

impl From<PaymentAttemptUpdate> for PaymentAttemptUpdateInternal {
    fn from(payment_attempt_update: PaymentAttemptUpdate) -> Self {
        match payment_attempt_update {
            PaymentAttemptUpdate::Update {
                amount,
                currency,
                status,
                // connector_transaction_id,
                authentication_type,
                payment_method,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                amount_to_capture,
                capture_method,
            } => Self {
                amount: Some(amount),
                currency: Some(currency),
                status: Some(status),
                // connector_transaction_id,
                authentication_type,
                payment_method,
                payment_token,
                modified_at: Some(common_utils::date_time::now()),
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                amount_to_capture,
                capture_method,
                ..Default::default()
            },
            PaymentAttemptUpdate::AuthenticationTypeUpdate {
                authentication_type,
            } => Self {
                authentication_type: Some(authentication_type),
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentAttemptUpdate::ConfirmUpdate {
                amount,
                currency,
                authentication_type,
                status,
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
            } => Self {
                amount: Some(amount),
                currency: Some(currency),
                authentication_type,
                status: Some(status),
                payment_method,
                modified_at: Some(common_utils::date_time::now()),
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
                ..Default::default()
            },
            PaymentAttemptUpdate::VoidUpdate {
                status,
                cancellation_reason,
            } => Self {
                status: Some(status),
                cancellation_reason,
                ..Default::default()
            },
            PaymentAttemptUpdate::RejectUpdate {
                status,
                error_code,
                error_message,
            } => Self {
                status: Some(status),
                error_code,
                error_message,
                ..Default::default()
            },
            PaymentAttemptUpdate::ResponseUpdate {
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
            } => Self {
                status: Some(status),
                connector,
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                modified_at: Some(common_utils::date_time::now()),
                mandate_id,
                connector_metadata,
                error_code,
                error_message,
                payment_token,
                error_reason,
                connector_response_reference_id,
                amount_capturable,
                ..Default::default()
            },
            PaymentAttemptUpdate::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
                amount_capturable,
            } => Self {
                connector,
                status: Some(status),
                error_message,
                error_code,
                modified_at: Some(common_utils::date_time::now()),
                error_reason,
                amount_capturable,
                ..Default::default()
            },
            PaymentAttemptUpdate::StatusUpdate { status } => Self {
                status: Some(status),
                ..Default::default()
            },
            PaymentAttemptUpdate::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
            } => Self {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
                ..Default::default()
            },
            PaymentAttemptUpdate::UnresolvedResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                payment_method_id,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
            } => Self {
                status: Some(status),
                connector,
                connector_transaction_id,
                payment_method_id,
                modified_at: Some(common_utils::date_time::now()),
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                ..Default::default()
            },
            PaymentAttemptUpdate::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
            } => Self {
                status: Some(status),
                payment_method_id,
                modified_at: Some(common_utils::date_time::now()),
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                ..Default::default()
            },
            PaymentAttemptUpdate::MultipleCaptureCountUpdate {
                multiple_capture_count,
            } => Self {
                multiple_capture_count: Some(multiple_capture_count),
                ..Default::default()
            },
            PaymentAttemptUpdate::AmountToCaptureUpdate {
                status,
                amount_capturable,
            } => Self {
                status: Some(status),
                amount_capturable: Some(amount_capturable),
                ..Default::default()
            },
            PaymentAttemptUpdate::SurchargeMetadataUpdate { surcharge_metadata } => Self {
                surcharge_metadata,
                ..Default::default()
            },
            PaymentAttemptUpdate::SurchargeAmountUpdate {
                surcharge_amount,
                tax_amount,
            } => Self {
                surcharge_amount,
                tax_amount,
                ..Default::default()
            },
        }
    }
}
