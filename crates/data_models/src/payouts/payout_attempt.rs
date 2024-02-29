use common_enums as storage_enums;
use storage_enums::MerchantStorageScheme;
use time::PrimitiveDateTime;

use crate::errors;

#[async_trait::async_trait]
pub trait PayoutAttemptInterface {
    async fn insert_payout_attempt(
        &self,
        _payout: PayoutAttemptNew,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError>;

    async fn find_payout_attempt_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError>;

    async fn update_payout_attempt(
        &self,
        _this: &PayoutAttempt,
        _payout_attempt_update: PayoutAttemptUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError>;

    async fn find_payout_attempt_by_merchant_id_payout_attempt_id(
        &self,
        _merchant_id: &str,
        _payout_attempt_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutAttempt {
    pub payout_attempt_id: String,
    pub payout_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub address_id: String,
    pub connector: Option<String>,
    pub connector_payout_id: String,
    pub payout_token: Option<String>,
    pub status: storage_enums::PayoutStatus,
    pub is_eligible: Option<bool>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub profile_id: String,
    pub merchant_connector_id: Option<String>,
    pub routing_info: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PayoutAttemptNew {
    pub payout_attempt_id: String,
    pub payout_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub address_id: String,
    pub connector: Option<String>,
    pub connector_payout_id: String,
    pub payout_token: Option<String>,
    pub status: storage_enums::PayoutStatus,
    pub is_eligible: Option<bool>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub created_at: Option<PrimitiveDateTime>,
    pub last_modified_at: Option<PrimitiveDateTime>,
    pub profile_id: String,
    pub merchant_connector_id: Option<String>,
    pub routing_info: Option<serde_json::Value>,
}

impl Default for PayoutAttemptNew {
    fn default() -> Self {
        let now = common_utils::date_time::now();

        Self {
            payout_attempt_id: String::default(),
            payout_id: String::default(),
            customer_id: String::default(),
            merchant_id: String::default(),
            address_id: String::default(),
            connector: None,
            connector_payout_id: String::default(),
            payout_token: None,
            status: storage_enums::PayoutStatus::default(),
            is_eligible: None,
            error_message: None,
            error_code: None,
            business_country: Some(storage_enums::CountryAlpha2::default()),
            business_label: None,
            created_at: Some(now),
            last_modified_at: Some(now),
            profile_id: String::default(),
            merchant_connector_id: None,
            routing_info: None,
        }
    }
}

#[derive(Debug)]
pub enum PayoutAttemptUpdate {
    StatusUpdate {
        connector_payout_id: String,
        status: storage_enums::PayoutStatus,
        error_message: Option<String>,
        error_code: Option<String>,
        is_eligible: Option<bool>,
    },
    PayoutTokenUpdate {
        payout_token: String,
    },
    BusinessUpdate {
        business_country: Option<storage_enums::CountryAlpha2>,
        business_label: Option<String>,
    },
    UpdateRouting {
        connector: String,
        routing_info: Option<serde_json::Value>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct PayoutAttemptUpdateInternal {
    pub payout_token: Option<String>,
    pub connector_payout_id: Option<String>,
    pub status: Option<storage_enums::PayoutStatus>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub is_eligible: Option<bool>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub connector: Option<String>,
    pub routing_info: Option<serde_json::Value>,
}

impl From<PayoutAttemptUpdate> for PayoutAttemptUpdateInternal {
    fn from(payout_update: PayoutAttemptUpdate) -> Self {
        match payout_update {
            PayoutAttemptUpdate::PayoutTokenUpdate { payout_token } => Self {
                payout_token: Some(payout_token),
                ..Default::default()
            },
            PayoutAttemptUpdate::StatusUpdate {
                connector_payout_id,
                status,
                error_message,
                error_code,
                is_eligible,
            } => Self {
                connector_payout_id: Some(connector_payout_id),
                status: Some(status),
                error_message,
                error_code,
                is_eligible,
                ..Default::default()
            },
            PayoutAttemptUpdate::BusinessUpdate {
                business_country,
                business_label,
            } => Self {
                business_country,
                business_label,
                ..Default::default()
            },
            PayoutAttemptUpdate::UpdateRouting {
                connector,
                routing_info,
            } => Self {
                connector: Some(connector),
                routing_info,
                ..Default::default()
            },
        }
    }
}
