use api_models::enums::PayoutConnectors;
use common_enums as storage_enums;
use common_utils::{
    id_type, payout_method_utils,
    types::{UnifiedCode, UnifiedMessage},
};
use serde::{Deserialize, Serialize};
use storage_enums::MerchantStorageScheme;
use time::PrimitiveDateTime;

use super::payouts::Payouts;
use crate::errors;

#[async_trait::async_trait]
pub trait PayoutAttemptInterface {
    async fn insert_payout_attempt(
        &self,
        _payout_attempt: PayoutAttemptNew,
        _payouts: &Payouts,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError>;

    async fn update_payout_attempt(
        &self,
        _this: &PayoutAttempt,
        _payout_attempt_update: PayoutAttemptUpdate,
        _payouts: &Payouts,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError>;

    async fn find_payout_attempt_by_merchant_id_payout_attempt_id(
        &self,
        _merchant_id: &id_type::MerchantId,
        _payout_attempt_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError>;

    async fn find_payout_attempt_by_merchant_id_connector_payout_id(
        &self,
        _merchant_id: &id_type::MerchantId,
        _connector_payout_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutAttempt, errors::StorageError>;

    async fn get_filters_for_payouts(
        &self,
        _payout: &[Payouts],
        _merchant_id: &id_type::MerchantId,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PayoutListFilters, errors::StorageError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutListFilters {
    pub connector: Vec<PayoutConnectors>,
    pub currency: Vec<storage_enums::Currency>,
    pub status: Vec<storage_enums::PayoutStatus>,
    pub payout_method: Vec<storage_enums::PayoutType>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PayoutAttempt {
    pub payout_attempt_id: String,
    pub payout_id: String,
    pub customer_id: Option<id_type::CustomerId>,
    pub merchant_id: id_type::MerchantId,
    pub address_id: Option<String>,
    pub connector: Option<String>,
    pub connector_payout_id: Option<String>,
    pub payout_token: Option<String>,
    pub status: storage_enums::PayoutStatus,
    pub is_eligible: Option<bool>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    pub profile_id: id_type::ProfileId,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    pub routing_info: Option<serde_json::Value>,
    pub unified_code: Option<UnifiedCode>,
    pub unified_message: Option<UnifiedMessage>,
    pub additional_payout_method_data: Option<payout_method_utils::AdditionalPayoutMethodData>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PayoutAttemptNew {
    pub payout_attempt_id: String,
    pub payout_id: String,
    pub customer_id: Option<id_type::CustomerId>,
    pub merchant_id: id_type::MerchantId,
    pub address_id: Option<String>,
    pub connector: Option<String>,
    pub connector_payout_id: Option<String>,
    pub payout_token: Option<String>,
    pub status: storage_enums::PayoutStatus,
    pub is_eligible: Option<bool>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub profile_id: id_type::ProfileId,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    pub routing_info: Option<serde_json::Value>,
    pub unified_code: Option<UnifiedCode>,
    pub unified_message: Option<UnifiedMessage>,
    pub additional_payout_method_data: Option<payout_method_utils::AdditionalPayoutMethodData>,
}

#[derive(Debug, Clone)]
pub enum PayoutAttemptUpdate {
    StatusUpdate {
        connector_payout_id: Option<String>,
        status: storage_enums::PayoutStatus,
        error_message: Option<String>,
        error_code: Option<String>,
        is_eligible: Option<bool>,

        unified_code: Option<UnifiedCode>,
        unified_message: Option<UnifiedMessage>,
    },
    PayoutTokenUpdate {
        payout_token: String,
    },
    BusinessUpdate {
        business_country: Option<storage_enums::CountryAlpha2>,
        business_label: Option<String>,
        address_id: Option<String>,
        customer_id: Option<id_type::CustomerId>,
    },
    UpdateRouting {
        connector: String,
        routing_info: Option<serde_json::Value>,
        merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    },
    AdditionalPayoutMethodDataUpdate {
        additional_payout_method_data: Option<payout_method_utils::AdditionalPayoutMethodData>,
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
    pub address_id: Option<String>,
    pub customer_id: Option<id_type::CustomerId>,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    pub unified_code: Option<UnifiedCode>,
    pub unified_message: Option<UnifiedMessage>,
    pub additional_payout_method_data: Option<payout_method_utils::AdditionalPayoutMethodData>,
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
                unified_code,
                unified_message,
            } => Self {
                connector_payout_id,
                status: Some(status),
                error_message,
                error_code,
                is_eligible,
                unified_code,
                unified_message,
                ..Default::default()
            },
            PayoutAttemptUpdate::BusinessUpdate {
                business_country,
                business_label,
                address_id,
                customer_id,
            } => Self {
                business_country,
                business_label,
                address_id,
                customer_id,
                ..Default::default()
            },
            PayoutAttemptUpdate::UpdateRouting {
                connector,
                routing_info,
                merchant_connector_id,
            } => Self {
                connector: Some(connector),
                routing_info,
                merchant_connector_id,
                ..Default::default()
            },
            PayoutAttemptUpdate::AdditionalPayoutMethodDataUpdate {
                additional_payout_method_data,
            } => Self {
                additional_payout_method_data,
                ..Default::default()
            },
        }
    }
}
