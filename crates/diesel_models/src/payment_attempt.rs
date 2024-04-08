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
    pub updated_by: String,
    pub merchant_connector_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub net_amount: Option<i64>,
    pub external_three_ds_authentication_attempted: Option<bool>,
    pub authentication_connector: Option<String>,
    pub authentication_id: Option<String>,
    pub mandate_data: Option<storage_enums::MandateDetails>,
    pub fingerprint_id: Option<String>,
    pub payment_method_billing_address_id: Option<String>,
}

impl PaymentAttempt {
    pub fn get_or_calculate_net_amount(&self) -> i64 {
        self.net_amount.unwrap_or(
            self.amount + self.surcharge_amount.unwrap_or(0) + self.tax_amount.unwrap_or(0),
        )
    }
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
    pub updated_by: String,
    pub merchant_connector_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub encoded_data: Option<String>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub net_amount: Option<i64>,
    pub external_three_ds_authentication_attempted: Option<bool>,
    pub authentication_connector: Option<String>,
    pub authentication_id: Option<String>,
    pub mandate_data: Option<storage_enums::MandateDetails>,
    pub fingerprint_id: Option<String>,
    pub payment_method_billing_address_id: Option<String>,
}

impl PaymentAttemptNew {
    /// returns amount + surcharge_amount + tax_amount
    pub fn calculate_net_amount(&self) -> i64 {
        self.amount + self.surcharge_amount.unwrap_or(0) + self.tax_amount.unwrap_or(0)
    }

    pub fn get_or_calculate_net_amount(&self) -> i64 {
        self.net_amount
            .unwrap_or_else(|| self.calculate_net_amount())
    }

    pub fn populate_derived_fields(self) -> Self {
        let mut payment_attempt_new = self;
        payment_attempt_new.net_amount = Some(payment_attempt_new.calculate_net_amount());
        payment_attempt_new
    }
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
        surcharge_amount: Option<i64>,
        tax_amount: Option<i64>,
        fingerprint_id: Option<String>,
        updated_by: String,
    },
    UpdateTrackers {
        payment_token: Option<String>,
        connector: Option<String>,
        straight_through_algorithm: Option<serde_json::Value>,
        amount_capturable: Option<i64>,
        surcharge_amount: Option<i64>,
        tax_amount: Option<i64>,
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
        fingerprint_id: Option<String>,
        updated_by: String,
        merchant_connector_id: Option<String>,
        payment_method_id: Option<String>,
        external_three_ds_authentication_attempted: Option<bool>,
        authentication_connector: Option<String>,
        authentication_id: Option<String>,
        payment_method_billing_address_id: Option<String>,
    },
    VoidUpdate {
        status: storage_enums::AttemptStatus,
        cancellation_reason: Option<String>,
        updated_by: String,
    },
    PaymentMethodDetailsUpdate{
        payment_method_id: Option<String>,
        updated_by: String,
    },
    BlocklistUpdate {
        status: storage_enums::AttemptStatus,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
        updated_by: String,
    },
    RejectUpdate {
        status: storage_enums::AttemptStatus,
        error_code: Option<Option<String>>,
        error_message: Option<Option<String>>,
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
        amount_capturable: Option<i64>,
        updated_by: String,
        authentication_data: Option<serde_json::Value>,
        encoded_data: Option<String>,
        unified_code: Option<Option<String>>,
        unified_message: Option<Option<String>>,
        payment_method_data: Option<serde_json::Value>,
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
        amount_capturable: Option<i64>,
        updated_by: String,
        unified_code: Option<Option<String>>,
        unified_message: Option<Option<String>>,
        connector_transaction_id: Option<String>,
        payment_method_data: Option<serde_json::Value>,
    },
    CaptureUpdate {
        amount_to_capture: Option<i64>,
        multiple_capture_count: Option<i16>,
        updated_by: String,
    },
    AmountToCaptureUpdate {
        status: storage_enums::AttemptStatus,
        amount_capturable: i64,
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
        updated_by: String,
    },
    IncrementalAuthorizationAmountUpdate {
        amount: i64,
        amount_capturable: i64,
    },
    AuthenticationUpdate {
        status: storage_enums::AttemptStatus,
        external_three_ds_authentication_attempted: Option<bool>,
        authentication_connector: Option<String>,
        authentication_id: Option<String>,
        updated_by: String,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payment_attempt)]
pub struct PaymentAttemptUpdateInternal {
    amount: Option<i64>,
    net_amount: Option<i64>,
    currency: Option<storage_enums::Currency>,
    status: Option<storage_enums::AttemptStatus>,
    connector_transaction_id: Option<String>,
    amount_to_capture: Option<i64>,
    connector: Option<Option<String>>,
    authentication_type: Option<storage_enums::AuthenticationType>,
    payment_method: Option<storage_enums::PaymentMethod>,
    error_message: Option<Option<String>>,
    payment_method_id: Option<String>,
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
    updated_by: String,
    merchant_connector_id: Option<Option<String>>,
    authentication_data: Option<serde_json::Value>,
    encoded_data: Option<String>,
    unified_code: Option<Option<String>>,
    unified_message: Option<Option<String>>,
    external_three_ds_authentication_attempted: Option<bool>,
    authentication_connector: Option<String>,
    authentication_id: Option<String>,
    fingerprint_id: Option<String>,
    payment_method_billing_address_id: Option<String>,
}

impl PaymentAttemptUpdateInternal {
    pub fn populate_derived_fields(self, source: &PaymentAttempt) -> Self {
        let mut update_internal = self;
        update_internal.net_amount = Some(
            update_internal.amount.unwrap_or(source.amount)
                + update_internal
                    .surcharge_amount
                    .or(source.surcharge_amount)
                    .unwrap_or(0)
                + update_internal
                    .tax_amount
                    .or(source.tax_amount)
                    .unwrap_or(0),
        );
        update_internal
    }
}

impl PaymentAttemptUpdate {
    pub fn apply_changeset(self, source: PaymentAttempt) -> PaymentAttempt {
        let PaymentAttemptUpdateInternal {
            amount,
            net_amount,
            currency,
            status,
            connector_transaction_id,
            amount_to_capture,
            connector,
            authentication_type,
            payment_method,
            error_message,
            payment_method_id,
            cancellation_reason,
            modified_at: _,
            mandate_id,
            browser_info,
            payment_token,
            error_code,
            connector_metadata,
            payment_method_data,
            payment_method_type,
            payment_experience,
            business_sub_label,
            straight_through_algorithm,
            preprocessing_step_id,
            error_reason,
            capture_method,
            connector_response_reference_id,
            multiple_capture_count,
            surcharge_amount,
            tax_amount,
            amount_capturable,
            updated_by,
            merchant_connector_id,
            authentication_data,
            encoded_data,
            unified_code,
            unified_message,
            external_three_ds_authentication_attempted,
            authentication_connector,
            authentication_id,
            payment_method_billing_address_id,
            fingerprint_id,
        } = PaymentAttemptUpdateInternal::from(self).populate_derived_fields(&source);
        PaymentAttempt {
            amount: amount.unwrap_or(source.amount),
            net_amount: net_amount.or(source.net_amount),
            currency: currency.or(source.currency),
            status: status.unwrap_or(source.status),
            connector_transaction_id: connector_transaction_id.or(source.connector_transaction_id),
            amount_to_capture: amount_to_capture.or(source.amount_to_capture),
            connector: connector.unwrap_or(source.connector),
            authentication_type: authentication_type.or(source.authentication_type),
            payment_method: payment_method.or(source.payment_method),
            error_message: error_message.unwrap_or(source.error_message),
            payment_method_id: payment_method_id.or(source.payment_method_id),
            cancellation_reason: cancellation_reason.or(source.cancellation_reason),
            modified_at: common_utils::date_time::now(),
            mandate_id: mandate_id.or(source.mandate_id),
            browser_info: browser_info.or(source.browser_info),
            payment_token: payment_token.or(source.payment_token),
            error_code: error_code.unwrap_or(source.error_code),
            connector_metadata: connector_metadata.or(source.connector_metadata),
            payment_method_data: payment_method_data.or(source.payment_method_data),
            payment_method_type: payment_method_type.or(source.payment_method_type),
            payment_experience: payment_experience.or(source.payment_experience),
            business_sub_label: business_sub_label.or(source.business_sub_label),
            straight_through_algorithm: straight_through_algorithm
                .or(source.straight_through_algorithm),
            preprocessing_step_id: preprocessing_step_id.or(source.preprocessing_step_id),
            error_reason: error_reason.unwrap_or(source.error_reason),
            capture_method: capture_method.or(source.capture_method),
            connector_response_reference_id: connector_response_reference_id
                .or(source.connector_response_reference_id),
            multiple_capture_count: multiple_capture_count.or(source.multiple_capture_count),
            surcharge_amount: surcharge_amount.or(source.surcharge_amount),
            tax_amount: tax_amount.or(source.tax_amount),
            amount_capturable: amount_capturable.unwrap_or(source.amount_capturable),
            updated_by,
            merchant_connector_id: merchant_connector_id.unwrap_or(source.merchant_connector_id),
            authentication_data: authentication_data.or(source.authentication_data),
            encoded_data: encoded_data.or(source.encoded_data),
            unified_code: unified_code.unwrap_or(source.unified_code),
            unified_message: unified_message.unwrap_or(source.unified_message),
            external_three_ds_authentication_attempted: external_three_ds_authentication_attempted
                .or(source.external_three_ds_authentication_attempted),
            authentication_connector: authentication_connector.or(source.authentication_connector),
            authentication_id: authentication_id.or(source.authentication_id),
            payment_method_billing_address_id: payment_method_billing_address_id
                .or(source.payment_method_billing_address_id),
            fingerprint_id: fingerprint_id.or(source.fingerprint_id),
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
                surcharge_amount,
                tax_amount,
                fingerprint_id,
                updated_by,
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
                surcharge_amount,
                tax_amount,
                fingerprint_id,
                updated_by,
                ..Default::default()
            },
            PaymentAttemptUpdate::AuthenticationTypeUpdate {
                authentication_type,
                updated_by,
            } => Self {
                authentication_type: Some(authentication_type),
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
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
                updated_by,
                merchant_connector_id,
                surcharge_amount,
                tax_amount,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                payment_method_billing_address_id,
                fingerprint_id,
                payment_method_id,
            } => Self {
                amount: Some(amount),
                currency: Some(currency),
                authentication_type,
                status: Some(status),
                payment_method,
                modified_at: Some(common_utils::date_time::now()),
                browser_info,
                connector: connector.map(Some),
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                straight_through_algorithm,
                error_code,
                error_message,
                amount_capturable,
                updated_by,
                merchant_connector_id: merchant_connector_id.map(Some),
                surcharge_amount,
                tax_amount,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                payment_method_billing_address_id,
                fingerprint_id,
                payment_method_id,
                ..Default::default()
            },
            PaymentAttemptUpdate::VoidUpdate {
                status,
                cancellation_reason,
                updated_by,
            } => Self {
                status: Some(status),
                cancellation_reason,
                updated_by,
                ..Default::default()
            },
            PaymentAttemptUpdate::RejectUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            } => Self {
                status: Some(status),
                error_code,
                error_message,
                updated_by,
                ..Default::default()
            },
            PaymentAttemptUpdate::BlocklistUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            } => Self {
                status: Some(status),
                error_code,
                connector: Some(None),
                error_message,
                updated_by,
                merchant_connector_id: Some(None),
                ..Default::default()
            },
            PaymentAttemptUpdate::PaymentMethodDetailsUpdate {
                payment_method_id,
                updated_by,
            } => Self {
                payment_method_id,
                updated_by,
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
                updated_by,
                authentication_data,
                encoded_data,
                unified_code,
                unified_message,
                payment_method_data,
            } => Self {
                status: Some(status),
                connector: connector.map(Some),
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
                updated_by,
                authentication_data,
                encoded_data,
                unified_code,
                unified_message,
                payment_method_data,
                ..Default::default()
            },
            PaymentAttemptUpdate::ErrorUpdate {
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
            } => Self {
                connector: connector.map(Some),
                status: Some(status),
                error_message,
                error_code,
                modified_at: Some(common_utils::date_time::now()),
                error_reason,
                amount_capturable,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
                payment_method_data,
                ..Default::default()
            },
            PaymentAttemptUpdate::StatusUpdate { status, updated_by } => Self {
                status: Some(status),
                updated_by,
                ..Default::default()
            },
            PaymentAttemptUpdate::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
                surcharge_amount,
                tax_amount,
                updated_by,
                merchant_connector_id,
            } => Self {
                payment_token,
                connector: connector.map(Some),
                straight_through_algorithm,
                amount_capturable,
                surcharge_amount,
                tax_amount,
                updated_by,
                merchant_connector_id: merchant_connector_id.map(Some),
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
                updated_by,
            } => Self {
                status: Some(status),
                connector: connector.map(Some),
                connector_transaction_id,
                payment_method_id,
                modified_at: Some(common_utils::date_time::now()),
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                updated_by,
                ..Default::default()
            },
            PaymentAttemptUpdate::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                updated_by,
            } => Self {
                status: Some(status),
                payment_method_id,
                modified_at: Some(common_utils::date_time::now()),
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                updated_by,
                ..Default::default()
            },
            PaymentAttemptUpdate::CaptureUpdate {
                multiple_capture_count,
                updated_by,
                amount_to_capture,
            } => Self {
                multiple_capture_count,
                updated_by,
                amount_to_capture,
                ..Default::default()
            },
            PaymentAttemptUpdate::AmountToCaptureUpdate {
                status,
                amount_capturable,
                updated_by,
            } => Self {
                status: Some(status),
                amount_capturable: Some(amount_capturable),
                updated_by,
                ..Default::default()
            },
            PaymentAttemptUpdate::ConnectorResponse {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                connector,
                updated_by,
            } => Self {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                connector: connector.map(Some),
                updated_by,
                ..Default::default()
            },
            PaymentAttemptUpdate::IncrementalAuthorizationAmountUpdate {
                amount,
                amount_capturable,
            } => Self {
                amount: Some(amount),
                amount_capturable: Some(amount_capturable),
                ..Default::default()
            },
            PaymentAttemptUpdate::AuthenticationUpdate {
                status,
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                updated_by,
            } => Self {
                status: Some(status),
                external_three_ds_authentication_attempted,
                authentication_connector,
                authentication_id,
                updated_by,
                ..Default::default()
            },
        }
    }
}

mod tests {

    #[test]
    fn test_backwards_compatibility() {
        let serialized_payment_attempt = r#"{
    "id": 1,
    "payment_id": "PMT123456789",
    "merchant_id": "M123456789",
    "attempt_id": "ATMPT123456789",
    "status": "pending",
    "amount": 10000,
    "currency": "USD",
    "save_to_locker": true,
    "connector": "stripe",
    "error_message": null,
    "offer_amount": 9500,
    "surcharge_amount": 500,
    "tax_amount": 800,
    "payment_method_id": "CRD123456789",
    "payment_method": "card",
    "connector_transaction_id": "CNTR123456789",
    "capture_method": "automatic",
    "capture_on": "2022-09-10T10:11:12Z",
    "confirm": false,
    "authentication_type": "no_three_ds",
    "created_at": "2024-02-26T12:00:00Z",
    "modified_at": "2024-02-26T12:00:00Z",
    "last_synced": null,
    "cancellation_reason": null,
    "amount_to_capture": 10000,
    "mandate_id": null,
    "browser_info": {
        "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
        "accept_header": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
        "language": "nl-NL",
        "color_depth": 24,
        "screen_height": 723,
        "screen_width": 1536,
        "time_zone": 0,
        "java_enabled": true,
        "java_script_enabled": true,
        "ip_address": "127.0.0.1"
    },
    "error_code": null,
    "payment_token": "TOKEN123456789",
    "connector_metadata": null,
    "payment_experience": "redirect_to_url",
    "payment_method_type": "credit",
    "payment_method_data": {
        "card": {
            "card_number": "4242424242424242",
            "card_exp_month": "10",
            "card_cvc": "123",
            "card_exp_year": "2024",
            "card_holder_name": "John Doe"
        }
    },
    "business_sub_label": "Premium",
    "straight_through_algorithm": null,
    "preprocessing_step_id": null,
    "mandate_details": null,
    "error_reason": null,
    "multiple_capture_count": 0,
    "connector_response_reference_id": null,
    "amount_capturable": 10000,
    "updated_by": "redis_kv",
    "merchant_connector_id": "MCN123456789",
    "authentication_data": null,
    "encoded_data": null,
    "unified_code": null,
    "unified_message": null,
    "net_amount": 10200,
    "mandate_data": {
    "customer_acceptance": {
        "acceptance_type": "offline",
        "accepted_at": "1963-05-03T04:07:52.723Z",
        "online": {
            "ip_address": "127.0.0.1",
            "user_agent": "amet irure esse"
        }
    },
    "mandate_type": {
        "single_use": {
            "amount": 6540,
            "currency": "USD"
        }
    }
},
    "fingerprint_id": null
}"#;
        let deserialized =
            serde_json::from_str::<super::PaymentAttempt>(serialized_payment_attempt);

        assert!(deserialized.is_ok());
    }
}
