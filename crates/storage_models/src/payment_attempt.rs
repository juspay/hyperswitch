use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::payment_attempt};

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
    pub connector_transaction_id: Option<String>,
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
    },
    UpdateTrackers {
        payment_token: Option<String>,
        connector: Option<String>,
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
    },
    VoidUpdate {
        status: storage_enums::AttemptStatus,
        cancellation_reason: Option<String>,
    },
    ResponseUpdate {
        status: storage_enums::AttemptStatus,
        connector: Option<String>,
        connector_transaction_id: Option<String>,
        authentication_type: Option<storage_enums::AuthenticationType>,
        payment_method_id: Option<Option<String>>,
        mandate_id: Option<String>,
        connector_metadata: Option<serde_json::Value>,
    },
    StatusUpdate {
        status: storage_enums::AttemptStatus,
    },
    ErrorUpdate {
        connector: Option<String>,
        status: storage_enums::AttemptStatus,
        error_code: Option<String>,
        error_message: Option<String>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payment_attempt)]
pub struct PaymentAttemptUpdateInternal {
    amount: Option<i64>,
    currency: Option<storage_enums::Currency>,
    status: Option<storage_enums::AttemptStatus>,
    connector_transaction_id: Option<String>,
    connector: Option<String>,
    authentication_type: Option<storage_enums::AuthenticationType>,
    payment_method: Option<storage_enums::PaymentMethod>,
    error_message: Option<String>,
    payment_method_id: Option<Option<String>>,
    cancellation_reason: Option<String>,
    modified_at: Option<PrimitiveDateTime>,
    mandate_id: Option<String>,
    browser_info: Option<serde_json::Value>,
    payment_token: Option<String>,
    error_code: Option<String>,
    connector_metadata: Option<serde_json::Value>,
    payment_method_data: Option<serde_json::Value>,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    payment_experience: Option<storage_enums::PaymentExperience>,
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
            error_message: pa_update.error_message.or(source.error_message),
            payment_method_id: pa_update
                .payment_method_id
                .unwrap_or(source.payment_method_id),
            browser_info: pa_update.browser_info.or(source.browser_info),
            modified_at: common_utils::date_time::now(),
            payment_token: pa_update.payment_token.or(source.payment_token),
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
            PaymentAttemptUpdate::ResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                mandate_id,
                connector_metadata,
            } => Self {
                status: Some(status),
                connector,
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                modified_at: Some(common_utils::date_time::now()),
                mandate_id,
                connector_metadata,
                ..Default::default()
            },
            PaymentAttemptUpdate::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
            } => Self {
                connector,
                status: Some(status),
                error_message,
                error_code,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentAttemptUpdate::StatusUpdate { status } => Self {
                status: Some(status),
                ..Default::default()
            },
            PaymentAttemptUpdate::UpdateTrackers {
                payment_token,
                connector,
            } => Self {
                payment_token,
                connector,
                ..Default::default()
            },
        }
    }
}
