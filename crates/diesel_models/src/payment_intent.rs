use common_enums::RequestIncrementalAuthorization;
use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::payment_intent};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = payment_intent)]
pub struct PaymentIntent {
    pub id: i32,
    pub payment_id: String,
    pub merchant_id: String,
    pub status: storage_enums::IntentStatus,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<i64>,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub billing_address_id: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
    pub active_attempt_id: String,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub allowed_payment_method_types: Option<serde_json::Value>,
    pub connector_metadata: Option<serde_json::Value>,
    pub feature_metadata: Option<serde_json::Value>,
    pub attempt_count: i16,
    pub profile_id: Option<String>,
    // Denotes the action(approve or reject) taken by merchant in case of manual review.
    // Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub merchant_decision: Option<String>,
    pub payment_link_id: Option<String>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,

    pub updated_by: String,
    pub surcharge_applicable: Option<bool>,
    pub request_incremental_authorization: Option<RequestIncrementalAuthorization>,
    pub incremental_authorization_allowed: Option<bool>,
    pub authorization_count: Option<i32>,
    pub session_expiry: Option<PrimitiveDateTime>,
    pub fingerprint_id: Option<String>,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize,
)]
#[diesel(table_name = payment_intent)]
pub struct PaymentIntentNew {
    pub payment_id: String,
    pub merchant_id: String,
    pub status: storage_enums::IntentStatus,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<i64>,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub billing_address_id: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub modified_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
    pub active_attempt_id: String,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub allowed_payment_method_types: Option<serde_json::Value>,
    pub connector_metadata: Option<serde_json::Value>,
    pub feature_metadata: Option<serde_json::Value>,
    pub attempt_count: i16,
    pub profile_id: Option<String>,
    pub merchant_decision: Option<String>,
    pub payment_link_id: Option<String>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,
    pub updated_by: String,
    pub surcharge_applicable: Option<bool>,
    pub request_incremental_authorization: Option<RequestIncrementalAuthorization>,
    pub incremental_authorization_allowed: Option<bool>,
    pub authorization_count: Option<i32>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub session_expiry: Option<PrimitiveDateTime>,
    pub fingerprint_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentIntentUpdate {
    ResponseUpdate {
        status: storage_enums::IntentStatus,
        amount_captured: Option<i64>,
        fingerprint_id: Option<String>,
        return_url: Option<String>,
        updated_by: String,
        incremental_authorization_allowed: Option<bool>,
    },
    MetadataUpdate {
        metadata: pii::SecretSerdeValue,
        updated_by: String,
    },
    ReturnUrlUpdate {
        return_url: Option<String>,
        status: Option<storage_enums::IntentStatus>,
        customer_id: Option<String>,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
        updated_by: String,
    },
    MerchantStatusUpdate {
        status: storage_enums::IntentStatus,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
        updated_by: String,
    },
    PGStatusUpdate {
        status: storage_enums::IntentStatus,
        updated_by: String,
        incremental_authorization_allowed: Option<bool>,
    },
    Update {
        amount: i64,
        currency: storage_enums::Currency,
        setup_future_usage: Option<storage_enums::FutureUsage>,
        status: storage_enums::IntentStatus,
        customer_id: Option<String>,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
        return_url: Option<String>,
        business_country: Option<storage_enums::CountryAlpha2>,
        business_label: Option<String>,
        description: Option<String>,
        statement_descriptor_name: Option<String>,
        statement_descriptor_suffix: Option<String>,
        order_details: Option<Vec<pii::SecretSerdeValue>>,
        metadata: Option<pii::SecretSerdeValue>,
        payment_confirm_source: Option<storage_enums::PaymentSource>,
        updated_by: String,
        session_expiry: Option<PrimitiveDateTime>,
        fingerprint_id: Option<String>,
    },
    PaymentAttemptAndAttemptCountUpdate {
        active_attempt_id: String,
        attempt_count: i16,
        updated_by: String,
    },
    StatusAndAttemptUpdate {
        status: storage_enums::IntentStatus,
        active_attempt_id: String,
        attempt_count: i16,
        updated_by: String,
    },
    ApproveUpdate {
        merchant_decision: Option<String>,
        updated_by: String,
    },
    RejectUpdate {
        status: storage_enums::IntentStatus,
        merchant_decision: Option<String>,
        updated_by: String,
    },
    SurchargeApplicableUpdate {
        surcharge_applicable: Option<bool>,
        updated_by: String,
    },
    IncrementalAuthorizationAmountUpdate {
        amount: i64,
    },
    AuthorizationCountUpdate {
        authorization_count: i32,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payment_intent)]
pub struct PaymentIntentUpdateInternal {
    pub amount: Option<i64>,
    pub currency: Option<storage_enums::Currency>,
    pub status: Option<storage_enums::IntentStatus>,
    pub amount_captured: Option<i64>,
    pub customer_id: Option<String>,
    pub return_url: Option<String>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub billing_address_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub active_attempt_id: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub description: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub attempt_count: Option<i16>,
    pub profile_id: Option<String>,
    merchant_decision: Option<String>,
    payment_confirm_source: Option<storage_enums::PaymentSource>,

    pub updated_by: String,
    pub surcharge_applicable: Option<bool>,
    pub incremental_authorization_allowed: Option<bool>,
    pub authorization_count: Option<i32>,
    pub session_expiry: Option<PrimitiveDateTime>,
    pub fingerprint_id: Option<String>,
}

impl PaymentIntentUpdate {
    pub fn apply_changeset(self, source: PaymentIntent) -> PaymentIntent {
        let PaymentIntentUpdateInternal {
            amount,
            currency,
            status,
            amount_captured,
            customer_id,
            return_url,
            setup_future_usage,
            off_session,
            metadata,
            billing_address_id,
            shipping_address_id,
            modified_at: _,
            active_attempt_id,
            business_country,
            business_label,
            description,
            statement_descriptor_name,
            statement_descriptor_suffix,
            order_details,
            attempt_count,
            profile_id,
            merchant_decision,
            payment_confirm_source,
            updated_by,
            surcharge_applicable,
            incremental_authorization_allowed,
            authorization_count,
            session_expiry,
            fingerprint_id,
        } = self.into();
        PaymentIntent {
            amount: amount.unwrap_or(source.amount),
            currency: currency.or(source.currency),
            status: status.unwrap_or(source.status),
            amount_captured: amount_captured.or(source.amount_captured),
            customer_id: customer_id.or(source.customer_id),
            return_url: return_url.or(source.return_url),
            setup_future_usage: setup_future_usage.or(source.setup_future_usage),
            off_session: off_session.or(source.off_session),
            metadata: metadata.or(source.metadata),
            billing_address_id: billing_address_id.or(source.billing_address_id),
            shipping_address_id: shipping_address_id.or(source.shipping_address_id),
            modified_at: common_utils::date_time::now(),
            active_attempt_id: active_attempt_id.unwrap_or(source.active_attempt_id),
            business_country: business_country.or(source.business_country),
            business_label: business_label.or(source.business_label),
            description: description.or(source.description),
            statement_descriptor_name: statement_descriptor_name
                .or(source.statement_descriptor_name),
            statement_descriptor_suffix: statement_descriptor_suffix
                .or(source.statement_descriptor_suffix),
            order_details: order_details.or(source.order_details),
            attempt_count: attempt_count.unwrap_or(source.attempt_count),
            profile_id: profile_id.or(source.profile_id),
            merchant_decision: merchant_decision.or(source.merchant_decision),
            payment_confirm_source: payment_confirm_source.or(source.payment_confirm_source),
            updated_by,
            surcharge_applicable: surcharge_applicable.or(source.surcharge_applicable),

            incremental_authorization_allowed: incremental_authorization_allowed
                .or(source.incremental_authorization_allowed),
            authorization_count: authorization_count.or(source.authorization_count),
            fingerprint_id: fingerprint_id.or(source.fingerprint_id),
            session_expiry: session_expiry.or(source.session_expiry),
            ..source
        }
    }
}

impl From<PaymentIntentUpdate> for PaymentIntentUpdateInternal {
    fn from(payment_intent_update: PaymentIntentUpdate) -> Self {
        match payment_intent_update {
            PaymentIntentUpdate::Update {
                amount,
                currency,
                setup_future_usage,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                return_url,
                business_country,
                business_label,
                description,
                statement_descriptor_name,
                statement_descriptor_suffix,
                order_details,
                metadata,
                payment_confirm_source,
                updated_by,
                session_expiry,
                fingerprint_id,
            } => Self {
                amount: Some(amount),
                currency: Some(currency),
                status: Some(status),
                setup_future_usage,
                customer_id,
                shipping_address_id,
                billing_address_id,
                modified_at: Some(common_utils::date_time::now()),
                return_url,
                business_country,
                business_label,
                description,
                statement_descriptor_name,
                statement_descriptor_suffix,
                order_details,
                metadata,
                payment_confirm_source,
                updated_by,
                session_expiry,
                fingerprint_id,
                ..Default::default()
            },
            PaymentIntentUpdate::MetadataUpdate {
                metadata,
                updated_by,
            } => Self {
                metadata: Some(metadata),
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::ReturnUrlUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                updated_by,
            } => Self {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::PGStatusUpdate {
                status,
                updated_by,
                incremental_authorization_allowed,
            } => Self {
                status: Some(status),
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                incremental_authorization_allowed,
                ..Default::default()
            },
            PaymentIntentUpdate::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
                updated_by,
            } => Self {
                status: Some(status),
                shipping_address_id,
                billing_address_id,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::ResponseUpdate {
                // amount,
                // currency,
                status,
                amount_captured,
                fingerprint_id,
                // customer_id,
                return_url,
                updated_by,
                incremental_authorization_allowed,
            } => Self {
                // amount,
                // currency: Some(currency),
                status: Some(status),
                amount_captured,
                fingerprint_id,
                // customer_id,
                return_url,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                incremental_authorization_allowed,
                ..Default::default()
            },
            PaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
                updated_by,
            } => Self {
                active_attempt_id: Some(active_attempt_id),
                attempt_count: Some(attempt_count),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
                updated_by,
            } => Self {
                status: Some(status),
                active_attempt_id: Some(active_attempt_id),
                attempt_count: Some(attempt_count),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::ApproveUpdate {
                merchant_decision,
                updated_by,
            } => Self {
                merchant_decision,
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::RejectUpdate {
                status,
                merchant_decision,
                updated_by,
            } => Self {
                status: Some(status),
                merchant_decision,
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::SurchargeApplicableUpdate {
                surcharge_applicable,
                updated_by,
            } => Self {
                surcharge_applicable,
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::IncrementalAuthorizationAmountUpdate { amount } => Self {
                amount: Some(amount),
                ..Default::default()
            },
            PaymentIntentUpdate::AuthorizationCountUpdate {
                authorization_count,
            } => Self {
                authorization_count: Some(authorization_count),
                ..Default::default()
            },
        }
    }
}
