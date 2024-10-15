use common_enums as storage_enums;
#[cfg(feature = "v2")]
use common_utils::ext_traits::{Encode, ValueExt};
use common_utils::{
    consts::{PAYMENTS_LIST_MAX_LIMIT_V1, PAYMENTS_LIST_MAX_LIMIT_V2},
    crypto::{self, Encryptable},
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    id_type,
    pii::{self, Email},
    type_name,
    types::{
        keymanager::{self, KeyManagerState, ToEncryptable},
        MinorUnit,
    },
};
use diesel_models::{
    PaymentIntent as DieselPaymentIntent, PaymentIntentNew as DieselPaymentIntentNew,
};
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use masking::ExposeInterface;
use masking::{Deserialize, PeekInterface, Secret};
use rustc_hash::FxHashMap;
use serde::Serialize;
use time::PrimitiveDateTime;

#[cfg(all(feature = "v1", feature = "olap"))]
use super::payment_attempt::PaymentAttempt;
use super::PaymentIntent;
use crate::{
    behaviour, errors,
    merchant_key_store::MerchantKeyStore,
    type_encryption::{crypto_operation, CryptoOperation},
    RemoteStorageObject,
};
#[async_trait::async_trait]
pub trait PaymentIntentInterface {
    async fn update_payment_intent(
        &self,
        state: &KeyManagerState,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    async fn insert_payment_intent(
        &self,
        state: &KeyManagerState,
        new: PaymentIntent,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        state: &KeyManagerState,
        payment_id: &id_type::PaymentId,
        merchant_id: &id_type::MerchantId,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    #[cfg(feature = "v2")]
    async fn find_payment_intent_by_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalPaymentId,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, errors::StorageError>;

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn filter_payment_intent_by_constraints(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        filters: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, errors::StorageError>;

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        time_range: &common_utils::types::TimeRange,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, errors::StorageError>;

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_intent_status_with_count(
        &self,
        merchant_id: &id_type::MerchantId,
        profile_id_list: Option<Vec<id_type::ProfileId>>,
        constraints: &common_utils::types::TimeRange,
    ) -> error_stack::Result<Vec<(common_enums::IntentStatus, i64)>, errors::StorageError>;

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_filtered_payment_intents_attempt(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, errors::StorageError>;

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &id_type::MerchantId,
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

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Serialize)]
pub struct PaymentIntentUpdateFields {
    pub amount: Option<MinorUnit>,
    pub currency: Option<storage_enums::Currency>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub status: storage_enums::IntentStatus,
    pub customer_id: Option<id_type::CustomerId>,
    pub shipping_address: Option<Encryptable<Secret<serde_json::Value>>>,
    pub billing_address: Option<Encryptable<Secret<serde_json::Value>>>,
    pub return_url: Option<String>,
    pub description: Option<String>,
    pub statement_descriptor: Option<String>,
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,
    pub updated_by: String,
    pub session_expiry: Option<PrimitiveDateTime>,
    pub request_external_three_ds_authentication: Option<bool>,
    pub frm_metadata: Option<pii::SecretSerdeValue>,
    pub customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub merchant_order_reference_id: Option<String>,
    pub is_payment_processor_token_flow: Option<bool>,
}

#[cfg(feature = "v1")]
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
    pub is_payment_processor_token_flow: Option<bool>,
    pub tax_details: Option<diesel_models::TaxDetails>,
}

#[cfg(feature = "v1")]
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
    SessionResponseUpdate {
        tax_details: diesel_models::TaxDetails,
        shipping_address_id: Option<String>,
        updated_by: String,
        shipping_details: Option<Encryptable<Secret<serde_json::Value>>>,
    },
}

// TODO: remove all enum variants and create new variants that should be used for v2
#[cfg(feature = "v2")]
#[derive(Debug, Clone, Serialize)]
pub enum PaymentIntentUpdate {
    ResponseUpdate {
        status: storage_enums::IntentStatus,
        amount_captured: Option<MinorUnit>,
        return_url: Option<String>,
        updated_by: String,
    },
    MetadataUpdate {
        metadata: pii::SecretSerdeValue,
        updated_by: String,
    },
    Update(Box<PaymentIntentUpdateFields>),
    PaymentCreateUpdate {
        return_url: Option<String>,
        status: Option<storage_enums::IntentStatus>,
        customer_id: Option<id_type::CustomerId>,
        shipping_address: Option<Encryptable<Secret<serde_json::Value>>>,
        billing_address: Option<Encryptable<Secret<serde_json::Value>>>,
        customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
        updated_by: String,
    },
    MerchantStatusUpdate {
        status: storage_enums::IntentStatus,
        shipping_address: Option<Encryptable<Secret<serde_json::Value>>>,
        billing_address: Option<Encryptable<Secret<serde_json::Value>>>,
        updated_by: String,
    },
    PGStatusUpdate {
        status: storage_enums::IntentStatus,
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
        frm_merchant_decision: Option<String>,
        updated_by: String,
    },
    RejectUpdate {
        status: storage_enums::IntentStatus,
        frm_merchant_decision: Option<String>,
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
        shipping_address: Option<Encryptable<Secret<serde_json::Value>>>,
    },
    ManualUpdate {
        status: Option<storage_enums::IntentStatus>,
        updated_by: String,
    },
}

#[cfg(feature = "v2")]
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
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub active_attempt_id: Option<String>,
    pub description: Option<String>,
    pub statement_descriptor: Option<String>,
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub attempt_count: Option<i16>,
    pub frm_merchant_decision: Option<common_enums::MerchantDecision>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,
    pub updated_by: String,
    pub surcharge_applicable: Option<bool>,
    pub authorization_count: Option<i32>,
    pub session_expiry: Option<PrimitiveDateTime>,
    pub request_external_three_ds_authentication: Option<bool>,
    pub frm_metadata: Option<pii::SecretSerdeValue>,
    pub customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub billing_address: Option<Encryptable<Secret<serde_json::Value>>>,
    pub shipping_address: Option<Encryptable<Secret<serde_json::Value>>>,
    pub merchant_order_reference_id: Option<String>,
    pub is_payment_processor_token_flow: Option<bool>,
}

#[cfg(feature = "v1")]
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
    pub is_payment_processor_token_flow: Option<bool>,
    pub tax_details: Option<diesel_models::TaxDetails>,
}

#[cfg(feature = "v2")]
impl From<PaymentIntentUpdate> for PaymentIntentUpdateInternal {
    fn from(payment_intent_update: PaymentIntentUpdate) -> Self {
        todo!()
        // match payment_intent_update {
        //     PaymentIntentUpdate::MetadataUpdate {
        //         metadata,
        //         updated_by,
        //     } => Self {
        //         metadata: Some(metadata),
        //         modified_at: Some(common_utils::date_time::now()),
        //         updated_by,
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::Update(value) => Self {
        //         amount: Some(value.amount),
        //         currency: Some(value.currency),
        //         setup_future_usage: value.setup_future_usage,
        //         status: Some(value.status),
        //         customer_id: value.customer_id,
        //         return_url: value.return_url,
        //         description: value.description,
        //         statement_descriptor: value.statement_descriptor,
        //         order_details: value.order_details,
        //         metadata: value.metadata,
        //         payment_confirm_source: value.payment_confirm_source,
        //         updated_by: value.updated_by,
        //         session_expiry: value.session_expiry,
        //         request_external_three_ds_authentication: value
        //             .request_external_three_ds_authentication,
        //         frm_metadata: value.frm_metadata,
        //         customer_details: value.customer_details,
        //         billing_address: value.billing_address,
        //         merchant_order_reference_id: value.merchant_order_reference_id,
        //         shipping_address: value.shipping_address,
        //         is_payment_processor_token_flow: value.is_payment_processor_token_flow,
        //         modified_at: Some(common_utils::date_time::now()),
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::PaymentCreateUpdate {
        //         return_url,
        //         status,
        //         customer_id,
        //         shipping_address,
        //         billing_address,
        //         customer_details,
        //         updated_by,
        //     } => Self {
        //         return_url,
        //         status,
        //         customer_id,
        //         shipping_address,
        //         billing_address,
        //         customer_details,
        //         modified_at: Some(common_utils::date_time::now()),
        //         updated_by,
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::PGStatusUpdate { status, updated_by } => Self {
        //         status: Some(status),
        //         modified_at: Some(common_utils::date_time::now()),
        //         updated_by,
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::MerchantStatusUpdate {
        //         status,
        //         shipping_address,
        //         billing_address,
        //         updated_by,
        //     } => Self {
        //         status: Some(status),
        //         shipping_address,
        //         billing_address,
        //         modified_at: Some(common_utils::date_time::now()),
        //         updated_by,
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::ResponseUpdate {
        //         // amount,
        //         // currency,
        //         status,
        //         amount_captured,
        //         // customer_id,
        //         return_url,
        //         updated_by,
        //     } => Self {
        //         // amount,
        //         // currency: Some(currency),
        //         status: Some(status),
        //         amount_captured,
        //         // customer_id,
        //         return_url,
        //         modified_at: Some(common_utils::date_time::now()),
        //         updated_by,
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate {
        //         active_attempt_id,
        //         attempt_count,
        //         updated_by,
        //     } => Self {
        //         active_attempt_id: Some(active_attempt_id),
        //         attempt_count: Some(attempt_count),
        //         updated_by,
        //         modified_at: Some(common_utils::date_time::now()),
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::StatusAndAttemptUpdate {
        //         status,
        //         active_attempt_id,
        //         attempt_count,
        //         updated_by,
        //     } => Self {
        //         status: Some(status),
        //         active_attempt_id: Some(active_attempt_id),
        //         attempt_count: Some(attempt_count),
        //         updated_by,
        //         modified_at: Some(common_utils::date_time::now()),
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::ApproveUpdate {
        //         status,
        //         frm_merchant_decision,
        //         updated_by,
        //     } => Self {
        //         status: Some(status),
        //         frm_merchant_decision,
        //         updated_by,
        //         modified_at: Some(common_utils::date_time::now()),
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::RejectUpdate {
        //         status,
        //         frm_merchant_decision,
        //         updated_by,
        //     } => Self {
        //         status: Some(status),
        //         frm_merchant_decision,
        //         updated_by,
        //         modified_at: Some(common_utils::date_time::now()),
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::SurchargeApplicableUpdate {
        //         surcharge_applicable,
        //         updated_by,
        //     } => Self {
        //         surcharge_applicable: Some(surcharge_applicable),
        //         modified_at: Some(common_utils::date_time::now()),
        //         updated_by,
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::IncrementalAuthorizationAmountUpdate { amount } => Self {
        //         amount: Some(amount),
        //         modified_at: Some(common_utils::date_time::now()),
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::AuthorizationCountUpdate {
        //         authorization_count,
        //     } => Self {
        //         authorization_count: Some(authorization_count),
        //         modified_at: Some(common_utils::date_time::now()),
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::CompleteAuthorizeUpdate { shipping_address } => Self {
        //         shipping_address,
        //         modified_at: Some(common_utils::date_time::now()),
        //         ..Default::default()
        //     },
        //     PaymentIntentUpdate::ManualUpdate { status, updated_by } => Self {
        //         status,
        //         modified_at: Some(common_utils::date_time::now()),
        //         updated_by,
        //         ..Default::default()
        //     },
        // }
    }
}

#[cfg(feature = "v1")]
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
                is_payment_processor_token_flow: value.is_payment_processor_token_flow,
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
            PaymentIntentUpdate::SessionResponseUpdate {
                tax_details,
                shipping_address_id,
                updated_by,
                shipping_details,
            } => Self {
                tax_details: Some(tax_details),
                shipping_address_id,
                updated_by,
                shipping_details,
                ..Default::default()
            },
        }
    }
}

use diesel_models::{
    PaymentIntentUpdate as DieselPaymentIntentUpdate,
    PaymentIntentUpdateFields as DieselPaymentIntentUpdateFields,
};
#[cfg(feature = "v2")]
impl From<PaymentIntentUpdate> for DieselPaymentIntentUpdate {
    fn from(value: PaymentIntentUpdate) -> Self {
        todo!()
        // match value {
        //     PaymentIntentUpdate::ResponseUpdate {
        //         status,
        //         amount_captured,
        //         return_url,
        //         updated_by,
        //     } => Self::ResponseUpdate {
        //         status,
        //         amount_captured,
        //         return_url,
        //         updated_by,
        //     },
        //     PaymentIntentUpdate::MetadataUpdate {
        //         metadata,
        //         updated_by,
        //     } => Self::MetadataUpdate {
        //         metadata,
        //         updated_by,
        //     },
        //     PaymentIntentUpdate::Update(value) => {
        //         Self::Update(Box::new(DieselPaymentIntentUpdateFields {
        //             amount: value.amount,
        //             currency: value.currency,
        //             setup_future_usage: value.setup_future_usage,
        //             status: value.status,
        //             customer_id: value.customer_id,
        //             return_url: value.return_url,
        //             description: value.description,
        //             statement_descriptor: value.statement_descriptor,
        //             order_details: value.order_details,
        //             metadata: value.metadata,
        //             payment_confirm_source: value.payment_confirm_source,
        //             updated_by: value.updated_by,
        //             session_expiry: value.session_expiry,
        //             request_external_three_ds_authentication: value
        //                 .request_external_three_ds_authentication,
        //             frm_metadata: value.frm_metadata,
        //             customer_details: value.customer_details.map(Encryption::from),
        //             billing_address: value.billing_address.map(Encryption::from),
        //             shipping_address: value.shipping_address.map(Encryption::from),
        //             merchant_order_reference_id: value.merchant_order_reference_id,
        //             is_payment_processor_token_flow: value.is_payment_processor_token_flow,
        //         }))
        //     }
        //     PaymentIntentUpdate::PaymentCreateUpdate {
        //         return_url,
        //         status,
        //         customer_id,
        //         shipping_address,
        //         billing_address,
        //         customer_details,
        //         updated_by,
        //     } => Self::PaymentCreateUpdate {
        //         return_url,
        //         status,
        //         customer_id,
        //         shipping_address: shipping_address.map(Encryption::from),
        //         billing_address: billing_address.map(Encryption::from),
        //         customer_details: customer_details.map(Encryption::from),
        //         updated_by,
        //     },
        //     PaymentIntentUpdate::MerchantStatusUpdate {
        //         status,
        //         shipping_address,
        //         billing_address,
        //         updated_by,
        //     } => Self::MerchantStatusUpdate {
        //         status,
        //         shipping_address: shipping_address.map(Encryption::from),
        //         billing_address: billing_address.map(Encryption::from),
        //         updated_by,
        //     },
        //     PaymentIntentUpdate::PGStatusUpdate { status, updated_by } => {
        //         Self::PGStatusUpdate { status, updated_by }
        //     }
        //     PaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate {
        //         active_attempt_id,
        //         attempt_count,
        //         updated_by,
        //     } => Self::PaymentAttemptAndAttemptCountUpdate {
        //         active_attempt_id,
        //         attempt_count,
        //         updated_by,
        //     },
        //     PaymentIntentUpdate::StatusAndAttemptUpdate {
        //         status,
        //         active_attempt_id,
        //         attempt_count,
        //         updated_by,
        //     } => Self::StatusAndAttemptUpdate {
        //         status,
        //         active_attempt_id,
        //         attempt_count,
        //         updated_by,
        //     },
        //     PaymentIntentUpdate::ApproveUpdate {
        //         status,
        //         frm_merchant_decision,
        //         updated_by,
        //     } => Self::ApproveUpdate {
        //         status,
        //         frm_merchant_decision,
        //         updated_by,
        //     },
        //     PaymentIntentUpdate::RejectUpdate {
        //         status,
        //         frm_merchant_decision,
        //         updated_by,
        //     } => Self::RejectUpdate {
        //         status,
        //         frm_merchant_decision,
        //         updated_by,
        //     },
        //     PaymentIntentUpdate::SurchargeApplicableUpdate {
        //         surcharge_applicable,
        //         updated_by,
        //     } => Self::SurchargeApplicableUpdate {
        //         surcharge_applicable: Some(surcharge_applicable),
        //         updated_by,
        //     },
        //     PaymentIntentUpdate::IncrementalAuthorizationAmountUpdate { amount } => {
        //         Self::IncrementalAuthorizationAmountUpdate { amount }
        //     }
        //     PaymentIntentUpdate::AuthorizationCountUpdate {
        //         authorization_count,
        //     } => Self::AuthorizationCountUpdate {
        //         authorization_count,
        //     },
        //     PaymentIntentUpdate::CompleteAuthorizeUpdate { shipping_address } => {
        //         Self::CompleteAuthorizeUpdate {
        //             shipping_address: shipping_address.map(Encryption::from),
        //         }
        //     }
        //     PaymentIntentUpdate::ManualUpdate { status, updated_by } => {
        //         Self::ManualUpdate { status, updated_by }
        //     }
        // }
    }
}

#[cfg(feature = "v1")]
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
                    is_payment_processor_token_flow: value.is_payment_processor_token_flow,
                    tax_details: value.tax_details,
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
            PaymentIntentUpdate::SessionResponseUpdate {
                tax_details,
                shipping_address_id,
                updated_by,
                shipping_details,
            } => Self::SessionResponseUpdate {
                tax_details,
                shipping_address_id,
                updated_by,
                shipping_details: shipping_details.map(Encryption::from),
            },
        }
    }
}

// TODO: evaluate if we will be using the same update struct for v2 as well, uncomment this and make necessary changes if necessary
#[cfg(feature = "v2")]
impl From<PaymentIntentUpdateInternal> for diesel_models::PaymentIntentUpdateInternal {
    fn from(value: PaymentIntentUpdateInternal) -> Self {
        todo!()
        // let modified_at = common_utils::date_time::now();
        // let PaymentIntentUpdateInternal {
        //     amount,
        //     currency,
        //     status,
        //     amount_captured,
        //     customer_id,
        //     return_url,
        //     setup_future_usage,
        //     off_session,
        //     metadata,
        //     modified_at: _,
        //     active_attempt_id,
        //     description,
        //     statement_descriptor,
        //     order_details,
        //     attempt_count,
        //     frm_merchant_decision,
        //     payment_confirm_source,
        //     updated_by,
        //     surcharge_applicable,
        //     authorization_count,
        //     session_expiry,
        //     request_external_three_ds_authentication,
        //     frm_metadata,
        //     customer_details,
        //     billing_address,
        //     merchant_order_reference_id,
        //     shipping_address,
        //     is_payment_processor_token_flow,
        // } = value;
        // Self {
        //     amount,
        //     currency,
        //     status,
        //     amount_captured,
        //     customer_id,
        //     return_url,
        //     setup_future_usage,
        //     off_session,
        //     metadata,
        //     modified_at,
        //     active_attempt_id,
        //     description,
        //     statement_descriptor,
        //     order_details,
        //     attempt_count,
        //     frm_merchant_decision,
        //     payment_confirm_source,
        //     updated_by,
        //     surcharge_applicable,
        //     authorization_count,
        //     session_expiry,
        //     request_external_three_ds_authentication,
        //     frm_metadata,
        //     customer_details: customer_details.map(Encryption::from),
        //     billing_address: billing_address.map(Encryption::from),
        //     merchant_order_reference_id,
        //     shipping_address: shipping_address.map(Encryption::from),
        //     is_payment_processor_token_flow,
        // }
    }
}

#[cfg(feature = "v1")]
impl From<PaymentIntentUpdateInternal> for diesel_models::PaymentIntentUpdateInternal {
    fn from(value: PaymentIntentUpdateInternal) -> Self {
        let modified_at = common_utils::date_time::now();
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
            is_payment_processor_token_flow,
            tax_details,
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
            is_payment_processor_token_flow,
            tax_details,
        }
    }
}

pub enum PaymentIntentFetchConstraints {
    Single {
        payment_intent_id: id_type::PaymentId,
    },
    List(Box<PaymentIntentListParams>),
}

impl PaymentIntentFetchConstraints {
    pub fn get_profile_id_list(&self) -> Option<Vec<id_type::ProfileId>> {
        if let Self::List(pi_list_params) = self {
            pi_list_params.profile_id.clone()
        } else {
            None
        }
    }
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
    pub merchant_connector_id: Option<Vec<id_type::MerchantConnectorAccountId>>,
    pub profile_id: Option<Vec<id_type::ProfileId>>,
    pub customer_id: Option<id_type::CustomerId>,
    pub starting_after_id: Option<id_type::PaymentId>,
    pub ending_before_id: Option<id_type::PaymentId>,
    pub limit: Option<u32>,
    pub order: api_models::payments::Order,
    pub card_network: Option<Vec<storage_enums::CardNetwork>>,
}

impl From<api_models::payments::PaymentListConstraints> for PaymentIntentFetchConstraints {
    fn from(value: api_models::payments::PaymentListConstraints) -> Self {
        let api_models::payments::PaymentListConstraints {
            customer_id,
            starting_after,
            ending_before,
            limit,
            created,
            created_lt,
            created_gt,
            created_lte,
            created_gte,
        } = value;
        Self::List(Box::new(PaymentIntentListParams {
            offset: 0,
            starting_at: created_gte.or(created_gt).or(created),
            ending_at: created_lte.or(created_lt).or(created),
            amount_filter: None,
            connector: None,
            currency: None,
            status: None,
            payment_method: None,
            payment_method_type: None,
            authentication_type: None,
            merchant_connector_id: None,
            profile_id: None,
            customer_id,
            starting_after_id: starting_after,
            ending_before_id: ending_before,
            limit: Some(std::cmp::min(limit, PAYMENTS_LIST_MAX_LIMIT_V1)),
            order: Default::default(),
            card_network: None,
        }))
    }
}

impl From<common_utils::types::TimeRange> for PaymentIntentFetchConstraints {
    fn from(value: common_utils::types::TimeRange) -> Self {
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
            order: Default::default(),
            card_network: None,
        }))
    }
}

impl From<api_models::payments::PaymentListFilterConstraints> for PaymentIntentFetchConstraints {
    fn from(value: api_models::payments::PaymentListFilterConstraints) -> Self {
        let api_models::payments::PaymentListFilterConstraints {
            payment_id,
            profile_id,
            customer_id,
            limit,
            offset,
            amount_filter,
            time_range,
            connector,
            currency,
            status,
            payment_method,
            payment_method_type,
            authentication_type,
            merchant_connector_id,
            order,
            card_network,
        } = value;
        if let Some(payment_intent_id) = payment_id {
            Self::Single { payment_intent_id }
        } else {
            Self::List(Box::new(PaymentIntentListParams {
                offset: offset.unwrap_or_default(),
                starting_at: time_range.map(|t| t.start_time),
                ending_at: time_range.and_then(|t| t.end_time),
                amount_filter,
                connector,
                currency,
                status,
                payment_method,
                payment_method_type,
                authentication_type,
                merchant_connector_id,
                profile_id: profile_id.map(|profile_id| vec![profile_id]),
                customer_id,
                starting_after_id: None,
                ending_before_id: None,
                limit: Some(std::cmp::min(limit, PAYMENTS_LIST_MAX_LIMIT_V2)),
                order,
                card_network,
            }))
        }
    }
}

impl<T> TryFrom<(T, Option<Vec<id_type::ProfileId>>)> for PaymentIntentFetchConstraints
where
    Self: From<T>,
{
    type Error = error_stack::Report<errors::api_error_response::ApiErrorResponse>;
    fn try_from(
        (constraints, auth_profile_id_list): (T, Option<Vec<id_type::ProfileId>>),
    ) -> Result<Self, Self::Error> {
        let payment_intent_constraints = Self::from(constraints);
        if let Self::List(mut pi_list_params) = payment_intent_constraints {
            let profile_id_from_request_body = pi_list_params.profile_id;
            match (profile_id_from_request_body, auth_profile_id_list) {
                (None, None) => pi_list_params.profile_id = None,
                (None, Some(auth_profile_id_list)) => {
                    pi_list_params.profile_id = Some(auth_profile_id_list)
                }
                (Some(profile_id_from_request_body), None) => {
                    pi_list_params.profile_id = Some(profile_id_from_request_body)
                }
                (Some(profile_id_from_request_body), Some(auth_profile_id_list)) => {
                    let profile_id_from_request_body_is_available_in_auth_profile_id_list =
                        profile_id_from_request_body
                            .iter()
                            .all(|profile_id| auth_profile_id_list.contains(profile_id));

                    if profile_id_from_request_body_is_available_in_auth_profile_id_list {
                        pi_list_params.profile_id = Some(profile_id_from_request_body)
                    } else {
                        // This scenario is very unlikely to happen
                        let inaccessible_profile_ids: Vec<_> = profile_id_from_request_body
                            .iter()
                            .filter(|profile_id| !auth_profile_id_list.contains(profile_id))
                            .collect();
                        return Err(error_stack::Report::new(
                            errors::api_error_response::ApiErrorResponse::PreconditionFailed {
                                message: format!(
                                    "Access not available for the given profile_id {:?}",
                                    inaccessible_profile_ids
                                ),
                            },
                        ));
                    }
                }
            }
            Ok(Self::List(pi_list_params))
        } else {
            Ok(payment_intent_constraints)
        }
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl behaviour::Conversion for PaymentIntent {
    type DstType = DieselPaymentIntent;
    type NewDstType = DieselPaymentIntentNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let Self {
            merchant_id,
            amount_details,
            status,
            amount_captured,
            customer_id,
            description,
            return_url,
            metadata,
            statement_descriptor,
            created_at,
            modified_at,
            last_synced,
            setup_future_usage,
            client_secret,
            active_attempt,
            order_details,
            allowed_payment_method_types,
            connector_metadata,
            feature_metadata,
            attempt_count,
            profile_id,
            payment_link_id,
            frm_merchant_decision,
            updated_by,
            request_incremental_authorization,
            authorization_count,
            session_expiry,
            request_external_three_ds_authentication,
            frm_metadata,
            customer_details,
            merchant_reference_id,
            billing_address,
            shipping_address,
            capture_method,
            id,
            authentication_type,
            prerouting_algorithm,
            organization_id,
            enable_payment_link,
            apply_mit_exemption,
            customer_present,
            routing_algorithm_id,
            payment_link_config,
        } = self;
        Ok(DieselPaymentIntent {
            skip_external_tax_calculation: Some(amount_details.get_external_tax_action_as_bool()),
            surcharge_applicable: Some(amount_details.get_surcharge_action_as_bool()),
            merchant_id,
            status,
            amount: amount_details.order_amount,
            currency: amount_details.currency,
            amount_captured,
            customer_id,
            description,
            return_url,
            metadata,
            statement_descriptor,
            created_at,
            modified_at,
            last_synced,
            setup_future_usage: Some(setup_future_usage),
            client_secret,
            active_attempt_id: active_attempt.map(|attempt| attempt.get_id()),
            order_details: order_details
                .map(|order_details| {
                    order_details
                        .into_iter()
                        .map(|order_detail| order_detail.encode_to_value().map(Secret::new))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "invalid value found for order_details".to_string(),
                })?,
            allowed_payment_method_types,
            connector_metadata,
            feature_metadata,
            attempt_count,
            profile_id,
            frm_merchant_decision,
            payment_link_id,
            updated_by,

            request_incremental_authorization: Some(request_incremental_authorization),
            authorization_count,
            session_expiry,
            request_external_three_ds_authentication: Some(
                request_external_three_ds_authentication.as_bool(),
            ),
            frm_metadata,
            customer_details: customer_details.map(Encryption::from),
            billing_address: billing_address.map(Encryption::from),
            shipping_address: shipping_address.map(Encryption::from),
            capture_method: Some(capture_method),
            id,
            authentication_type: Some(authentication_type),
            prerouting_algorithm,
            merchant_reference_id,
            surcharge_amount: amount_details.surcharge_amount,
            tax_on_surcharge: amount_details.tax_on_surcharge,
            organization_id,
            shipping_cost: amount_details.shipping_cost,
            tax_details: amount_details.tax_details,
            enable_payment_link: Some(enable_payment_link.as_bool()),
            apply_mit_exemption: Some(apply_mit_exemption.as_bool()),
            customer_present: Some(customer_present.as_bool()),
            payment_link_config,
            routing_algorithm_id,
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
            let decrypted_data = crypto_operation(
                state,
                type_name!(Self::DstType),
                CryptoOperation::BatchDecrypt(EncryptedPaymentIntentAddress::to_encryptable(
                    EncryptedPaymentIntentAddress {
                        billing: storage_model.billing_address,
                        shipping: storage_model.shipping_address,
                        customer_details: storage_model.customer_details,
                    },
                )),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let data = EncryptedPaymentIntentAddress::from_encryptable(decrypted_data)
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Invalid batch operation data")?;

            let amount_details = super::AmountDetails {
                order_amount: storage_model.amount,
                currency: storage_model.currency,
                surcharge_amount: storage_model.surcharge_amount,
                tax_on_surcharge: storage_model.tax_on_surcharge,
                shipping_cost: storage_model.shipping_cost,
                tax_details: storage_model.tax_details,
                skip_external_tax_calculation: super::TaxCalculationOverride::from(
                    storage_model.skip_external_tax_calculation,
                ),
                skip_surcharge_calculation: super::SurchargeCalculationOverride::from(
                    storage_model.surcharge_applicable,
                ),
            };
            let billing_address = data
                .billing
                .map(|billing| {
                    billing.deserialize_inner_value(|value| value.parse_value("Address"))
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Address")?;

            let shipping_address = data
                .shipping
                .map(|shipping| {
                    shipping.deserialize_inner_value(|value| value.parse_value("Address"))
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Address")?;

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                merchant_id: storage_model.merchant_id,
                status: storage_model.status,
                amount_details,
                amount_captured: storage_model.amount_captured,
                customer_id: storage_model.customer_id,
                description: storage_model.description,
                return_url: storage_model.return_url,
                metadata: storage_model.metadata,
                statement_descriptor: storage_model.statement_descriptor,
                created_at: storage_model.created_at,
                modified_at: storage_model.modified_at,
                last_synced: storage_model.last_synced,
                setup_future_usage: storage_model.setup_future_usage.unwrap_or_default(),
                client_secret: storage_model.client_secret,
                active_attempt: storage_model
                    .active_attempt_id
                    .map(RemoteStorageObject::ForeignID),
                order_details: storage_model
                    .order_details
                    .map(|order_details| {
                        order_details
                            .into_iter()
                            .map(|order_detail| {
                                order_detail.expose().parse_value("OrderDetailsWithAmount")
                            })
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()
                    .change_context(common_utils::errors::CryptoError::DecodingFailed)?,
                allowed_payment_method_types: storage_model.allowed_payment_method_types,
                connector_metadata: storage_model.connector_metadata,
                feature_metadata: storage_model.feature_metadata,
                attempt_count: storage_model.attempt_count,
                profile_id: storage_model.profile_id,
                frm_merchant_decision: storage_model.frm_merchant_decision,
                payment_link_id: storage_model.payment_link_id,
                updated_by: storage_model.updated_by,
                request_incremental_authorization: storage_model
                    .request_incremental_authorization
                    .unwrap_or_default(),
                authorization_count: storage_model.authorization_count,
                session_expiry: storage_model.session_expiry,
                request_external_three_ds_authentication:
                    common_enums::External3dsAuthenticationRequest::from(
                        storage_model.request_external_three_ds_authentication,
                    ),
                frm_metadata: storage_model.frm_metadata,
                customer_details: data.customer_details,
                billing_address,
                shipping_address,
                capture_method: storage_model.capture_method.unwrap_or_default(),
                id: storage_model.id,
                merchant_reference_id: storage_model.merchant_reference_id,
                organization_id: storage_model.organization_id,
                authentication_type: storage_model.authentication_type.unwrap_or_default(),
                prerouting_algorithm: storage_model.prerouting_algorithm,
                enable_payment_link: common_enums::EnablePaymentLinkRequest::from(
                    storage_model.enable_payment_link,
                ),
                apply_mit_exemption: common_enums::MitExemptionRequest::from(
                    storage_model.apply_mit_exemption,
                ),
                customer_present: common_enums::PresenceOfCustomerDuringPayment::from(
                    storage_model.customer_present,
                ),
                payment_link_config: storage_model.payment_link_config,
                routing_algorithm_id: storage_model.routing_algorithm_id,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment intent".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let amount_details = self.amount_details;

        Ok(DieselPaymentIntentNew {
            surcharge_applicable: Some(amount_details.get_surcharge_action_as_bool()),
            skip_external_tax_calculation: Some(amount_details.get_external_tax_action_as_bool()),
            merchant_id: self.merchant_id,
            status: self.status,
            amount: amount_details.order_amount,
            currency: amount_details.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: self.return_url,
            metadata: self.metadata,
            statement_descriptor: self.statement_descriptor,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            setup_future_usage: Some(self.setup_future_usage),
            client_secret: self.client_secret,
            active_attempt_id: self.active_attempt.map(|attempt| attempt.get_id()),
            order_details: self
                .order_details
                .map(|order_details| {
                    order_details
                        .into_iter()
                        .map(|order_detail| order_detail.encode_to_value().map(Secret::new))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Invalid value found for ".to_string(),
                })?,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            frm_merchant_decision: self.frm_merchant_decision,
            payment_link_id: self.payment_link_id,
            updated_by: self.updated_by,

            request_incremental_authorization: Some(self.request_incremental_authorization),
            authorization_count: self.authorization_count,
            session_expiry: self.session_expiry,
            request_external_three_ds_authentication: Some(
                self.request_external_three_ds_authentication.as_bool(),
            ),
            frm_metadata: self.frm_metadata,
            customer_details: self.customer_details.map(Encryption::from),
            billing_address: self.billing_address.map(Encryption::from),
            shipping_address: self.shipping_address.map(Encryption::from),
            capture_method: Some(self.capture_method),
            id: self.id,
            merchant_reference_id: self.merchant_reference_id,
            authentication_type: Some(self.authentication_type),
            prerouting_algorithm: self.prerouting_algorithm,
            surcharge_amount: amount_details.surcharge_amount,
            tax_on_surcharge: amount_details.tax_on_surcharge,
            organization_id: self.organization_id,
            shipping_cost: amount_details.shipping_cost,
            tax_details: amount_details.tax_details,
            enable_payment_link: Some(self.enable_payment_link.as_bool()),
            apply_mit_exemption: Some(self.apply_mit_exemption.as_bool()),
        })
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl behaviour::Conversion for PaymentIntent {
    type DstType = DieselPaymentIntent;
    type NewDstType = DieselPaymentIntentNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(DieselPaymentIntent {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: self.return_url,
            metadata: self.metadata,
            connector_id: self.connector_id,
            shipping_address_id: self.shipping_address_id,
            billing_address_id: self.billing_address_id,
            statement_descriptor_name: self.statement_descriptor_name,
            statement_descriptor_suffix: self.statement_descriptor_suffix,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            setup_future_usage: self.setup_future_usage,
            off_session: self.off_session,
            client_secret: self.client_secret,
            active_attempt_id: self.active_attempt.get_id(),
            business_country: self.business_country,
            business_label: self.business_label,
            order_details: self.order_details,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            merchant_decision: self.merchant_decision,
            payment_link_id: self.payment_link_id,
            payment_confirm_source: self.payment_confirm_source,
            updated_by: self.updated_by,
            surcharge_applicable: self.surcharge_applicable,
            request_incremental_authorization: self.request_incremental_authorization,
            incremental_authorization_allowed: self.incremental_authorization_allowed,
            authorization_count: self.authorization_count,
            fingerprint_id: self.fingerprint_id,
            session_expiry: self.session_expiry,
            request_external_three_ds_authentication: self.request_external_three_ds_authentication,
            charges: self.charges,
            frm_metadata: self.frm_metadata,
            customer_details: self.customer_details.map(Encryption::from),
            billing_details: self.billing_details.map(Encryption::from),
            merchant_order_reference_id: self.merchant_order_reference_id,
            shipping_details: self.shipping_details.map(Encryption::from),
            is_payment_processor_token_flow: self.is_payment_processor_token_flow,
            organization_id: self.organization_id,
            shipping_cost: self.shipping_cost,
            tax_details: self.tax_details,
            skip_external_tax_calculation: self.skip_external_tax_calculation,
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
            let decrypted_data = crypto_operation(
                state,
                type_name!(Self::DstType),
                CryptoOperation::BatchDecrypt(EncryptedPaymentIntentAddress::to_encryptable(
                    EncryptedPaymentIntentAddress {
                        billing: storage_model.billing_details,
                        shipping: storage_model.shipping_details,
                        customer_details: storage_model.customer_details,
                    },
                )),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let data = EncryptedPaymentIntentAddress::from_encryptable(decrypted_data)
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Invalid batch operation data")?;

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                payment_id: storage_model.payment_id,
                merchant_id: storage_model.merchant_id,
                status: storage_model.status,
                amount: storage_model.amount,
                currency: storage_model.currency,
                amount_captured: storage_model.amount_captured,
                customer_id: storage_model.customer_id,
                description: storage_model.description,
                return_url: storage_model.return_url,
                metadata: storage_model.metadata,
                connector_id: storage_model.connector_id,
                shipping_address_id: storage_model.shipping_address_id,
                billing_address_id: storage_model.billing_address_id,
                statement_descriptor_name: storage_model.statement_descriptor_name,
                statement_descriptor_suffix: storage_model.statement_descriptor_suffix,
                created_at: storage_model.created_at,
                modified_at: storage_model.modified_at,
                last_synced: storage_model.last_synced,
                setup_future_usage: storage_model.setup_future_usage,
                off_session: storage_model.off_session,
                client_secret: storage_model.client_secret,
                active_attempt: RemoteStorageObject::ForeignID(storage_model.active_attempt_id),
                business_country: storage_model.business_country,
                business_label: storage_model.business_label,
                order_details: storage_model.order_details,
                allowed_payment_method_types: storage_model.allowed_payment_method_types,
                connector_metadata: storage_model.connector_metadata,
                feature_metadata: storage_model.feature_metadata,
                attempt_count: storage_model.attempt_count,
                profile_id: storage_model.profile_id,
                merchant_decision: storage_model.merchant_decision,
                payment_link_id: storage_model.payment_link_id,
                payment_confirm_source: storage_model.payment_confirm_source,
                updated_by: storage_model.updated_by,
                surcharge_applicable: storage_model.surcharge_applicable,
                request_incremental_authorization: storage_model.request_incremental_authorization,
                incremental_authorization_allowed: storage_model.incremental_authorization_allowed,
                authorization_count: storage_model.authorization_count,
                fingerprint_id: storage_model.fingerprint_id,
                session_expiry: storage_model.session_expiry,
                request_external_three_ds_authentication: storage_model
                    .request_external_three_ds_authentication,
                charges: storage_model.charges,
                frm_metadata: storage_model.frm_metadata,
                shipping_cost: storage_model.shipping_cost,
                tax_details: storage_model.tax_details,
                customer_details: data.customer_details,
                billing_details: data.billing,
                merchant_order_reference_id: storage_model.merchant_order_reference_id,
                shipping_details: data.shipping,
                is_payment_processor_token_flow: storage_model.is_payment_processor_token_flow,
                organization_id: storage_model.organization_id,
                skip_external_tax_calculation: storage_model.skip_external_tax_calculation,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment intent".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(DieselPaymentIntentNew {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: self.return_url,
            metadata: self.metadata,
            connector_id: self.connector_id,
            shipping_address_id: self.shipping_address_id,
            billing_address_id: self.billing_address_id,
            statement_descriptor_name: self.statement_descriptor_name,
            statement_descriptor_suffix: self.statement_descriptor_suffix,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            setup_future_usage: self.setup_future_usage,
            off_session: self.off_session,
            client_secret: self.client_secret,
            active_attempt_id: self.active_attempt.get_id(),
            business_country: self.business_country,
            business_label: self.business_label,
            order_details: self.order_details,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            merchant_decision: self.merchant_decision,
            payment_link_id: self.payment_link_id,
            payment_confirm_source: self.payment_confirm_source,
            updated_by: self.updated_by,
            surcharge_applicable: self.surcharge_applicable,
            request_incremental_authorization: self.request_incremental_authorization,
            incremental_authorization_allowed: self.incremental_authorization_allowed,
            authorization_count: self.authorization_count,
            fingerprint_id: self.fingerprint_id,
            session_expiry: self.session_expiry,
            request_external_three_ds_authentication: self.request_external_three_ds_authentication,
            charges: self.charges,
            frm_metadata: self.frm_metadata,
            customer_details: self.customer_details.map(Encryption::from),
            billing_details: self.billing_details.map(Encryption::from),
            merchant_order_reference_id: self.merchant_order_reference_id,
            shipping_details: self.shipping_details.map(Encryption::from),
            is_payment_processor_token_flow: self.is_payment_processor_token_flow,
            organization_id: self.organization_id,
            shipping_cost: self.shipping_cost,
            tax_details: self.tax_details,
            skip_external_tax_calculation: self.skip_external_tax_calculation,
        })
    }
}

pub struct EncryptedPaymentIntentAddress {
    pub shipping: Option<Encryption>,
    pub billing: Option<Encryption>,
    pub customer_details: Option<Encryption>,
}

pub struct PaymentAddressFromRequest {
    pub shipping: Option<Secret<serde_json::Value>>,
    pub billing: Option<Secret<serde_json::Value>>,
    pub customer_details: Option<Secret<serde_json::Value>>,
}

pub struct DecryptedPaymentIntentAddress {
    pub shipping: crypto::OptionalEncryptableValue,
    pub billing: crypto::OptionalEncryptableValue,
    pub customer_details: crypto::OptionalEncryptableValue,
}

impl ToEncryptable<DecryptedPaymentIntentAddress, Secret<serde_json::Value>, Encryption>
    for EncryptedPaymentIntentAddress
{
    fn from_encryptable(
        mut hashmap: FxHashMap<String, Encryptable<Secret<serde_json::Value>>>,
    ) -> CustomResult<DecryptedPaymentIntentAddress, common_utils::errors::ParsingError> {
        Ok(DecryptedPaymentIntentAddress {
            shipping: hashmap.remove("shipping"),
            billing: hashmap.remove("billing"),
            customer_details: hashmap.remove("customer_details"),
        })
    }

    fn to_encryptable(self) -> FxHashMap<String, Encryption> {
        let mut map = FxHashMap::with_capacity_and_hasher(9, Default::default());

        self.shipping.map(|s| map.insert("shipping".to_string(), s));
        self.billing.map(|s| map.insert("billing".to_string(), s));
        self.customer_details
            .map(|s| map.insert("customer_details".to_string(), s));
        map
    }
}

impl
    ToEncryptable<
        DecryptedPaymentIntentAddress,
        Secret<serde_json::Value>,
        Secret<serde_json::Value>,
    > for PaymentAddressFromRequest
{
    fn from_encryptable(
        mut hashmap: FxHashMap<String, Encryptable<Secret<serde_json::Value>>>,
    ) -> CustomResult<DecryptedPaymentIntentAddress, common_utils::errors::ParsingError> {
        Ok(DecryptedPaymentIntentAddress {
            shipping: hashmap.remove("shipping"),
            billing: hashmap.remove("billing"),
            customer_details: hashmap.remove("customer_details"),
        })
    }

    fn to_encryptable(self) -> FxHashMap<String, Secret<serde_json::Value>> {
        let mut map = FxHashMap::with_capacity_and_hasher(9, Default::default());

        self.shipping.map(|s| map.insert("shipping".to_string(), s));
        self.billing.map(|s| map.insert("billing".to_string(), s));
        self.customer_details
            .map(|s| map.insert("customer_details".to_string(), s));
        map
    }
}
