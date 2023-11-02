use common_enums as storage_enums;
use common_utils::{
    consts::{PAYMENTS_LIST_MAX_LIMIT_V1, PAYMENTS_LIST_MAX_LIMIT_V2},
    pii,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::{payment_attempt::PaymentAttempt, PaymentIntent};
use crate::{errors, RemoteStorageObject};
#[async_trait::async_trait]
pub trait PaymentIntentInterface {
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    async fn insert_payment_intent(
        &self,
        new: PaymentIntentNew,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    async fn get_active_payment_attempt(
        &self,
        payment: &mut PaymentIntent,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_payment_intent_by_constraints(
        &self,
        merchant_id: &str,
        filters: &PaymentIntentFetchConstraints,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        merchant_id: &str,
        time_range: &api_models::payments::TimeRange,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn get_filtered_payment_intents_attempt(
        &self,
        merchant_id: &str,
        constraints: &PaymentIntentFetchConstraints,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &str,
        constraints: &PaymentIntentFetchConstraints,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, errors::StorageError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub created_at: Option<PrimitiveDateTime>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub last_synced: Option<PrimitiveDateTime>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
    pub active_attempt: RemoteStorageObject<PaymentAttempt>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentIntentUpdate {
    ResponseUpdate {
        status: storage_enums::IntentStatus,
        amount_captured: Option<i64>,
        return_url: Option<String>,
        updated_by: String,
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
        surcharge_applicable: bool,
        updated_by: String,
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

    pub updated_by: String,
    pub surcharge_applicable: Option<bool>,
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
            PaymentIntentUpdate::PGStatusUpdate { status, updated_by } => Self {
                status: Some(status),
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
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
                // customer_id,
                return_url,
                updated_by,
            } => Self {
                // amount,
                // currency: Some(currency),
                status: Some(status),
                amount_captured,
                // customer_id,
                return_url,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
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
                surcharge_applicable: Some(surcharge_applicable),
                updated_by,
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
