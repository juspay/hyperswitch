use common_enums as storage_enums;
use common_utils::{
    consts::{PAYMENTS_LIST_MAX_LIMIT_V1, PAYMENTS_LIST_MAX_LIMIT_V2},
    pii,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{errors, MerchantStorageScheme};
#[async_trait::async_trait]
pub trait PaymentIntentInterface {
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    async fn insert_payment_intent(
        &self,
        new: PaymentIntentNew,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_payment_intent_by_constraints(
        &self,
        merchant_id: &str,
        filters: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        merchant_id: &str,
        time_range: &api_models::payments::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn get_filtered_payment_intents_attempt(
        &self,
        merchant_id: &str,
        constraints: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<
        Vec<(PaymentIntent, super::payment_attempt::PaymentAttempt)>,
        errors::StorageError,
    >;

    #[cfg(feature = "olap")]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &str,
        constraints: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, errors::StorageError>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub allowed_payment_method_types: Option<serde_json::Value>,
    pub connector_metadata: Option<serde_json::Value>,
    pub feature_metadata: Option<serde_json::Value>,
    pub attempt_count: i16,
    pub profile_id: Option<String>,
    // Denotes the action(approve or reject) taken by merchant in case of manual review.
    // Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub merchant_decision: Option<String>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
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
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub allowed_payment_method_types: Option<serde_json::Value>,
    pub connector_metadata: Option<serde_json::Value>,
    pub feature_metadata: Option<serde_json::Value>,
    pub attempt_count: i16,
    pub profile_id: Option<String>,
    pub merchant_decision: Option<String>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentIntentUpdate {
    ResponseUpdate {
        status: storage_enums::IntentStatus,
        amount_captured: Option<i64>,
        return_url: Option<String>,
    },
    MetadataUpdate {
        metadata: pii::SecretSerdeValue,
    },
    ReturnUrlUpdate {
        return_url: Option<String>,
        status: Option<storage_enums::IntentStatus>,
        customer_id: Option<String>,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
    },
    MerchantStatusUpdate {
        status: storage_enums::IntentStatus,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
    },
    PGStatusUpdate {
        status: storage_enums::IntentStatus,
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
    },
    PaymentAttemptAndAttemptCountUpdate {
        active_attempt_id: String,
        attempt_count: i16,
    },
    StatusAndAttemptUpdate {
        status: storage_enums::IntentStatus,
        active_attempt_id: String,
        attempt_count: i16,
    },
    ApproveUpdate {
        merchant_decision: Option<String>,
    },
    RejectUpdate {
        status: storage_enums::IntentStatus,
        merchant_decision: Option<String>,
    },
}

#[derive(Clone, Debug, Default)]
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
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub attempt_count: Option<i16>,
    // Denotes the action(approve or reject) taken by merchant in case of manual review.
    // Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub merchant_decision: Option<String>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,
}

impl PaymentIntentUpdate {
    pub fn apply_changeset(self, source: PaymentIntent) -> PaymentIntent {
        let internal_update: PaymentIntentUpdateInternal = self.into();
        PaymentIntent {
            amount: internal_update.amount.unwrap_or(source.amount),
            currency: internal_update.currency.or(source.currency),
            status: internal_update.status.unwrap_or(source.status),
            amount_captured: internal_update.amount_captured.or(source.amount_captured),
            customer_id: internal_update.customer_id.or(source.customer_id),
            return_url: internal_update.return_url.or(source.return_url),
            setup_future_usage: internal_update
                .setup_future_usage
                .or(source.setup_future_usage),
            off_session: internal_update.off_session.or(source.off_session),
            metadata: internal_update.metadata.or(source.metadata),
            billing_address_id: internal_update
                .billing_address_id
                .or(source.billing_address_id),
            shipping_address_id: internal_update
                .shipping_address_id
                .or(source.shipping_address_id),
            modified_at: common_utils::date_time::now(),
            order_details: internal_update.order_details.or(source.order_details),
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
                ..Default::default()
            },
            PaymentIntentUpdate::MetadataUpdate { metadata } => Self {
                metadata: Some(metadata),
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::ReturnUrlUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
            } => Self {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::PGStatusUpdate { status } => Self {
                status: Some(status),
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
            } => Self {
                status: Some(status),
                shipping_address_id,
                billing_address_id,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::ResponseUpdate {
                // amount,
                // currency,
                status,
                amount_captured,
                // customer_id,
                return_url,
            } => Self {
                // amount,
                // currency: Some(currency),
                status: Some(status),
                amount_captured,
                // customer_id,
                return_url,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            PaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
            } => Self {
                active_attempt_id: Some(active_attempt_id),
                attempt_count: Some(attempt_count),
                ..Default::default()
            },
            PaymentIntentUpdate::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
            } => Self {
                status: Some(status),
                active_attempt_id: Some(active_attempt_id),
                attempt_count: Some(attempt_count),
                ..Default::default()
            },
            PaymentIntentUpdate::ApproveUpdate { merchant_decision } => Self {
                merchant_decision,
                ..Default::default()
            },
            PaymentIntentUpdate::RejectUpdate {
                status,
                merchant_decision,
            } => Self {
                status: Some(status),
                merchant_decision,
                ..Default::default()
            },
        }
    }
}

pub enum PaymentIntentFetchConstraints {
    Single { payment_intent_id: String },
    List(Box<PaymentIntentListParams>),
}

pub struct PaymentIntentListParams {
    pub offset: u32,
    pub starting_at: Option<PrimitiveDateTime>,
    pub ending_at: Option<PrimitiveDateTime>,
    pub connector: Option<Vec<api_models::enums::Connector>>,
    pub currency: Option<Vec<storage_enums::Currency>>,
    pub status: Option<Vec<storage_enums::IntentStatus>>,
    pub payment_method: Option<Vec<storage_enums::PaymentMethod>>,
    pub payment_method_type: Option<Vec<storage_enums::PaymentMethodType>>,
    pub authentication_type: Option<Vec<storage_enums::AuthenticationType>>,
    pub profile_id: Option<String>,
    pub customer_id: Option<String>,
    pub starting_after_id: Option<String>,
    pub ending_before_id: Option<String>,
    pub limit: Option<u32>,
}

impl From<api_models::payments::PaymentListConstraints> for PaymentIntentFetchConstraints {
    fn from(value: api_models::payments::PaymentListConstraints) -> Self {
        Self::List(Box::new(PaymentIntentListParams {
            offset: 0,
            starting_at: value.created_gte.or(value.created_gt).or(value.created),
            ending_at: value.created_lte.or(value.created_lt).or(value.created),
            connector: None,
            currency: None,
            status: None,
            payment_method: None,
            payment_method_type: None,
            authentication_type: None,
            profile_id: None,
            customer_id: value.customer_id,
            starting_after_id: value.starting_after,
            ending_before_id: value.ending_before,
            limit: Some(std::cmp::min(value.limit, PAYMENTS_LIST_MAX_LIMIT_V1)),
        }))
    }
}

impl From<api_models::payments::TimeRange> for PaymentIntentFetchConstraints {
    fn from(value: api_models::payments::TimeRange) -> Self {
        Self::List(Box::new(PaymentIntentListParams {
            offset: 0,
            starting_at: Some(value.start_time),
            ending_at: value.end_time,
            connector: None,
            currency: None,
            status: None,
            payment_method: None,
            payment_method_type: None,
            authentication_type: None,
            profile_id: None,
            customer_id: None,
            starting_after_id: None,
            ending_before_id: None,
            limit: None,
        }))
    }
}

impl From<api_models::payments::PaymentListFilterConstraints> for PaymentIntentFetchConstraints {
    fn from(value: api_models::payments::PaymentListFilterConstraints) -> Self {
        if let Some(payment_intent_id) = value.payment_id {
            Self::Single { payment_intent_id }
        } else {
            Self::List(Box::new(PaymentIntentListParams {
                offset: value.offset.unwrap_or_default(),
                starting_at: value.time_range.map(|t| t.start_time),
                ending_at: value.time_range.and_then(|t| t.end_time),
                connector: value.connector,
                currency: value.currency,
                status: value.status,
                payment_method: value.payment_method,
                payment_method_type: value.payment_method_type,
                authentication_type: value.authentication_type,
                profile_id: value.profile_id,
                customer_id: value.customer_id,
                starting_after_id: None,
                ending_before_id: None,
                limit: Some(std::cmp::min(value.limit, PAYMENTS_LIST_MAX_LIMIT_V2)),
            }))
        }
    }
}
