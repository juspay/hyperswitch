use common_enums as storage_enums;
use common_utils::{
    consts::{PAYMENTS_LIST_MAX_LIMIT_V1, PAYMENTS_LIST_MAX_LIMIT_V2},
    crypto::Encryptable,
    id_type,
    pii::{self, Email},
    types::MinorUnit,
};
use masking::{Deserialize, Secret};
use serde::Serialize;
use time::PrimitiveDateTime;

use super::{payment_attempt::PaymentAttempt, PaymentIntent};
use crate::{errors, merchant_key_store::MerchantKeyStore, RemoteStorageObject};
#[async_trait::async_trait]
pub trait PaymentIntentInterface {
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    async fn insert_payment_intent(
        &self,
        new: PaymentIntent,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        merchant_key_store: &MerchantKeyStore,
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
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        merchant_id: &str,
        time_range: &api_models::payments::TimeRange,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn get_filtered_payment_intents_attempt(
        &self,
        merchant_id: &str,
        constraints: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
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

#[derive(Clone, Debug, PartialEq, router_derive::DebugAsDisplay, Serialize, Deserialize)]
pub struct CustomerData {
    pub name: Option<Secret<String>>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaymentIntentNew {
    pub payment_id: String,
    pub merchant_id: String,
    pub status: storage_enums::IntentStatus,
    pub amount: MinorUnit,
    pub currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<MinorUnit>,
    pub customer_id: Option<id_type::CustomerId>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub frm_metadata: Option<pii::SecretSerdeValue>,
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
    pub request_incremental_authorization: Option<storage_enums::RequestIncrementalAuthorization>,
    pub incremental_authorization_allowed: Option<bool>,
    pub authorization_count: Option<i32>,
    pub fingerprint_id: Option<String>,
    pub session_expiry: Option<PrimitiveDateTime>,
    pub request_external_three_ds_authentication: Option<bool>,
    pub charges: Option<pii::SecretSerdeValue>,
    pub customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub billing_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub shipping_details: Option<Encryptable<Secret<serde_json::Value>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentIntentUpdateFields {
    pub amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub status: storage_enums::IntentStatus,
    pub customer_id: Option<id_type::CustomerId>,
    pub shipping_address_id: Option<String>,
    pub billing_address_id: Option<String>,
    pub return_url: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub description: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub metadata: Option<serde_json::Value>,
    pub frm_metadata: Option<pii::SecretSerdeValue>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,
    pub updated_by: String,
    pub fingerprint_id: Option<String>,
    pub session_expiry: Option<PrimitiveDateTime>,
    pub request_external_three_ds_authentication: Option<bool>,
    pub customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub billing_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub merchant_order_reference_id: Option<String>,
    pub shipping_details: Option<Encryptable<Secret<serde_json::Value>>>,
}

#[derive(Debug, Clone, Serialize)]
pub enum PaymentIntentUpdate {
    ResponseUpdate {
        status: storage_enums::IntentStatus,
        amount_captured: Option<MinorUnit>,
        return_url: Option<String>,
        updated_by: String,
        fingerprint_id: Option<String>,
        incremental_authorization_allowed: Option<bool>,
    },
    MetadataUpdate {
        metadata: serde_json::Value,
        updated_by: String,
    },
    Update(Box<PaymentIntentUpdateFields>),
    PaymentCreateUpdate {
        return_url: Option<String>,
        status: Option<storage_enums::IntentStatus>,
        customer_id: Option<id_type::CustomerId>,
        shipping_address_id: Option<String>,
        billing_address_id: Option<String>,
        customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
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
        incremental_authorization_allowed: Option<bool>,
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
        status: storage_enums::IntentStatus,
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
    IncrementalAuthorizationAmountUpdate {
        amount: MinorUnit,
    },
    AuthorizationCountUpdate {
        authorization_count: i32,
    },
    CompleteAuthorizeUpdate {
        shipping_address_id: Option<String>,
    },
    ManualUpdate {
        status: Option<storage_enums::IntentStatus>,
        updated_by: String,
    },
}

#[derive(Clone, Debug, Default)]
pub struct PaymentIntentUpdateInternal {
    pub amount: Option<MinorUnit>,
    pub currency: Option<storage_enums::Currency>,
    pub status: Option<storage_enums::IntentStatus>,
    pub amount_captured: Option<MinorUnit>,
    pub customer_id: Option<id_type::CustomerId>,
    pub return_url: Option<String>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub metadata: Option<serde_json::Value>,
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
    pub incremental_authorization_allowed: Option<bool>,
    pub authorization_count: Option<i32>,
    pub fingerprint_id: Option<String>,
    pub session_expiry: Option<PrimitiveDateTime>,
    pub request_external_three_ds_authentication: Option<bool>,
    pub frm_metadata: Option<pii::SecretSerdeValue>,
    pub customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub billing_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub merchant_order_reference_id: Option<String>,
    pub shipping_details: Option<Encryptable<Secret<serde_json::Value>>>,
}

impl From<PaymentIntentUpdate> for PaymentIntentUpdateInternal {
    fn from(payment_intent_update: PaymentIntentUpdate) -> Self {
        match payment_intent_update {
            PaymentIntentUpdate::MetadataUpdate {
                metadata,
                updated_by,
            } => Self {
                metadata: Some(metadata),
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::Update(value) => Self {
                amount: Some(value.amount),
                currency: Some(value.currency),
                setup_future_usage: value.setup_future_usage,
                status: Some(value.status),
                customer_id: value.customer_id,
                shipping_address_id: value.shipping_address_id,
                billing_address_id: value.billing_address_id,
                return_url: value.return_url,
                business_country: value.business_country,
                business_label: value.business_label,
                description: value.description,
                statement_descriptor_name: value.statement_descriptor_name,
                statement_descriptor_suffix: value.statement_descriptor_suffix,
                order_details: value.order_details,
                metadata: value.metadata,
                payment_confirm_source: value.payment_confirm_source,
                updated_by: value.updated_by,
                session_expiry: value.session_expiry,
                fingerprint_id: value.fingerprint_id,
                request_external_three_ds_authentication: value
                    .request_external_three_ds_authentication,
                frm_metadata: value.frm_metadata,
                customer_details: value.customer_details,
                billing_details: value.billing_details,
                merchant_order_reference_id: value.merchant_order_reference_id,
                shipping_details: value.shipping_details,
                ..Default::default()
            },
            PaymentIntentUpdate::PaymentCreateUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                customer_details,
                updated_by,
            } => Self {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                customer_details,
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
                status,
                merchant_decision,
                updated_by,
            } => Self {
                status: Some(status),
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
            PaymentIntentUpdate::CompleteAuthorizeUpdate {
                shipping_address_id,
            } => Self {
                shipping_address_id,
                ..Default::default()
            },
            PaymentIntentUpdate::ManualUpdate { status, updated_by } => Self {
                status,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                ..Default::default()
            },
        }
    }
}

use diesel_models::{
    encryption::Encryption, PaymentIntentUpdate as DieselPaymentIntentUpdate,
    PaymentIntentUpdateFields as DieselPaymentIntentUpdateFields,
};

impl From<PaymentIntentUpdate> for DieselPaymentIntentUpdate {
    fn from(value: PaymentIntentUpdate) -> Self {
        match value {
            PaymentIntentUpdate::ResponseUpdate {
                status,
                amount_captured,
                fingerprint_id,
                return_url,
                updated_by,
                incremental_authorization_allowed,
            } => Self::ResponseUpdate {
                status,
                amount_captured,
                fingerprint_id,
                return_url,
                updated_by,
                incremental_authorization_allowed,
            },
            PaymentIntentUpdate::MetadataUpdate {
                metadata,
                updated_by,
            } => Self::MetadataUpdate {
                metadata,
                updated_by,
            },
            PaymentIntentUpdate::Update(value) => {
                Self::Update(Box::new(DieselPaymentIntentUpdateFields {
                    amount: value.amount,
                    currency: value.currency,
                    setup_future_usage: value.setup_future_usage,
                    status: value.status,
                    customer_id: value.customer_id,
                    shipping_address_id: value.shipping_address_id,
                    billing_address_id: value.billing_address_id,
                    return_url: value.return_url,
                    business_country: value.business_country,
                    business_label: value.business_label,
                    description: value.description,
                    statement_descriptor_name: value.statement_descriptor_name,
                    statement_descriptor_suffix: value.statement_descriptor_suffix,
                    order_details: value.order_details,
                    metadata: value.metadata,
                    payment_confirm_source: value.payment_confirm_source,
                    updated_by: value.updated_by,
                    session_expiry: value.session_expiry,
                    fingerprint_id: value.fingerprint_id,
                    request_external_three_ds_authentication: value
                        .request_external_three_ds_authentication,
                    frm_metadata: value.frm_metadata,
                    customer_details: value.customer_details.map(Encryption::from),
                    billing_details: value.billing_details.map(Encryption::from),
                    merchant_order_reference_id: value.merchant_order_reference_id,
                    shipping_details: value.shipping_details.map(Encryption::from),
                }))
            }
            PaymentIntentUpdate::PaymentCreateUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                customer_details,
                updated_by,
            } => Self::PaymentCreateUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                customer_details: customer_details.map(Encryption::from),
                updated_by,
            },
            PaymentIntentUpdate::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
                updated_by,
            } => Self::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
                updated_by,
            },
            PaymentIntentUpdate::PGStatusUpdate {
                status,
                updated_by,
                incremental_authorization_allowed,
            } => Self::PGStatusUpdate {
                status,
                updated_by,
                incremental_authorization_allowed,
            },
            PaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
                updated_by,
            } => Self::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
                updated_by,
            },
            PaymentIntentUpdate::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
                updated_by,
            } => Self::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
                updated_by,
            },
            PaymentIntentUpdate::ApproveUpdate {
                status,
                merchant_decision,
                updated_by,
            } => Self::ApproveUpdate {
                status,
                merchant_decision,
                updated_by,
            },
            PaymentIntentUpdate::RejectUpdate {
                status,
                merchant_decision,
                updated_by,
            } => Self::RejectUpdate {
                status,
                merchant_decision,
                updated_by,
            },
            PaymentIntentUpdate::SurchargeApplicableUpdate {
                surcharge_applicable,
                updated_by,
            } => Self::SurchargeApplicableUpdate {
                surcharge_applicable: Some(surcharge_applicable),
                updated_by,
            },
            PaymentIntentUpdate::IncrementalAuthorizationAmountUpdate { amount } => {
                Self::IncrementalAuthorizationAmountUpdate { amount }
            }
            PaymentIntentUpdate::AuthorizationCountUpdate {
                authorization_count,
            } => Self::AuthorizationCountUpdate {
                authorization_count,
            },
            PaymentIntentUpdate::CompleteAuthorizeUpdate {
                shipping_address_id,
            } => Self::CompleteAuthorizeUpdate {
                shipping_address_id,
            },
            PaymentIntentUpdate::ManualUpdate { status, updated_by } => {
                Self::ManualUpdate { status, updated_by }
            }
        }
    }
}

impl From<PaymentIntentUpdateInternal> for diesel_models::PaymentIntentUpdateInternal {
    fn from(value: PaymentIntentUpdateInternal) -> Self {
        let modified_at = Some(common_utils::date_time::now());

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
            merchant_decision,
            payment_confirm_source,
            updated_by,
            surcharge_applicable,
            incremental_authorization_allowed,
            authorization_count,
            session_expiry,
            fingerprint_id,
            request_external_three_ds_authentication,
            frm_metadata,
            customer_details,
            billing_details,
            merchant_order_reference_id,
            shipping_details,
        } = value;

        Self {
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
            modified_at,
            active_attempt_id,
            business_country,
            business_label,
            description,
            statement_descriptor_name,
            statement_descriptor_suffix,
            order_details,
            attempt_count,
            merchant_decision,
            payment_confirm_source,
            updated_by,
            surcharge_applicable,
            incremental_authorization_allowed,
            authorization_count,
            session_expiry,
            fingerprint_id,
            request_external_three_ds_authentication,
            frm_metadata,
            customer_details: customer_details.map(Encryption::from),
            billing_details: billing_details.map(Encryption::from),
            merchant_order_reference_id,
            shipping_details: shipping_details.map(Encryption::from),
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
    pub amount_filter: Option<api_models::payments::AmountFilter>,
    pub connector: Option<Vec<api_models::enums::Connector>>,
    pub currency: Option<Vec<storage_enums::Currency>>,
    pub status: Option<Vec<storage_enums::IntentStatus>>,
    pub payment_method: Option<Vec<storage_enums::PaymentMethod>>,
    pub payment_method_type: Option<Vec<storage_enums::PaymentMethodType>>,
    pub authentication_type: Option<Vec<storage_enums::AuthenticationType>>,
    pub merchant_connector_id: Option<Vec<String>>,
    pub profile_id: Option<String>,
    pub customer_id: Option<id_type::CustomerId>,
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
            amount_filter: None,
            connector: None,
            currency: None,
            status: None,
            payment_method: None,
            payment_method_type: None,
            authentication_type: None,
            merchant_connector_id: None,
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
            amount_filter: None,
            connector: None,
            currency: None,
            status: None,
            payment_method: None,
            payment_method_type: None,
            authentication_type: None,
            merchant_connector_id: None,
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
                amount_filter: value.amount_filter,
                connector: value.connector,
                currency: value.currency,
                status: value.status,
                payment_method: value.payment_method,
                payment_method_type: value.payment_method_type,
                authentication_type: value.authentication_type,
                merchant_connector_id: value.merchant_connector_id,
                profile_id: value.profile_id,
                customer_id: value.customer_id,
                starting_after_id: None,
                ending_before_id: None,
                limit: Some(std::cmp::min(value.limit, PAYMENTS_LIST_MAX_LIMIT_V2)),
            }))
        }
    }
}
