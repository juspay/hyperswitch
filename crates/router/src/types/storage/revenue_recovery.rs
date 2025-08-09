use std::fmt::Debug;

use common_enums::enums::{self, CardNetwork};
use common_utils::{ext_traits::ValueExt, id_type};
use external_services::grpc_client::{self as external_grpc_client, GrpcHeaders};
use hyperswitch_domain_models::{
    business_profile, merchant_account, merchant_connector_account, merchant_key_store,
    payment_method_data::{Card, PaymentMethodData},
    payments::{payment_attempt::PaymentAttempt, PaymentIntent},
};
use masking::PeekInterface;
use router_env::logger;
use serde::{Deserialize, Serialize};

use crate::{db::StorageInterface, routes::SessionState, types, workflows::revenue_recovery};
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RevenueRecoveryWorkflowTrackingData {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub global_payment_id: id_type::GlobalPaymentId,
    pub payment_attempt_id: id_type::GlobalAttemptId,
    pub billing_mca_id: id_type::MerchantConnectorAccountId,
    pub revenue_recovery_retry: enums::RevenueRecoveryAlgorithmType,
    pub invoice_scheduled_time: Option<time::PrimitiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct RevenueRecoveryPaymentData {
    pub merchant_account: merchant_account::MerchantAccount,
    pub profile: business_profile::Profile,
    pub key_store: merchant_key_store::MerchantKeyStore,
    pub billing_mca: merchant_connector_account::MerchantConnectorAccount,
    pub retry_algorithm: enums::RevenueRecoveryAlgorithmType,
}
impl RevenueRecoveryPaymentData {
    pub async fn get_schedule_time_based_on_retry_type(
        &self,
        state: &SessionState,
        merchant_id: &id_type::MerchantId,
        retry_count: i32,
        payment_attempt: &PaymentAttempt,
        payment_intent: &PaymentIntent,
    ) -> Option<time::PrimitiveDateTime> {
        match self.retry_algorithm {
            enums::RevenueRecoveryAlgorithmType::Monitoring => {
                logger::error!("Monitoring type found for Revenue Recovery retry payment");
                None
            }
            enums::RevenueRecoveryAlgorithmType::Cascading => {
                revenue_recovery::get_schedule_time_to_retry_mit_payments(
                    state.store.as_ref(),
                    merchant_id,
                    retry_count,
                )
                .await
            }
            enums::RevenueRecoveryAlgorithmType::Smart => {
                revenue_recovery::get_schedule_time_for_smart_retry(
                    state,
                    payment_attempt,
                    payment_intent,
                    retry_count,
                    None,
                )
                .await
            }
        }
    }
}

#[derive(Debug, serde::Deserialize, Clone, Default)]
pub struct RevenueRecoverySettings {
    pub monitoring_threshold_in_seconds: i64,
    pub retry_algorithm_type: enums::RevenueRecoveryAlgorithmType,
    pub recovery_timestamp: RecoveryTimestamp,
    pub card_config: RetryLimitsConfig,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct RecoveryTimestamp {
    pub initial_timestamp_in_hours: i64,
}

impl Default for RecoveryTimestamp {
    fn default() -> Self {
        Self {
            initial_timestamp_in_hours: 1,
        }
    }
}

#[derive(Debug, serde::Deserialize, Clone, Default)]
pub struct RetryLimitsConfig {
    pub amex: NetworkRetryConfig,
    pub mastercard: NetworkRetryConfig,
    pub visa: NetworkRetryConfig,
    pub discover: NetworkRetryConfig,
}

#[derive(Debug, serde::Deserialize, Clone, Default)]
pub struct NetworkRetryConfig {
    pub max_daily_retry_count: i32,
    pub max_retries_last_30_days: i32,
}
impl RetryLimitsConfig {
    pub fn get_network_config(
        network: Option<CardNetwork>,
        state: &SessionState,
    ) -> &NetworkRetryConfig {
        match network {
            Some(CardNetwork::Mastercard) => &state.conf.revenue_recovery.card_config.mastercard,
            Some(CardNetwork::Visa) => &state.conf.revenue_recovery.card_config.visa,
            Some(CardNetwork::AmericanExpress) => &state.conf.revenue_recovery.card_config.amex,
            Some(CardNetwork::Discover) => &state.conf.revenue_recovery.card_config.discover,
            // All other networks (including None) default to Visa configuration
            _ => &state.conf.revenue_recovery.card_config.visa,
        }
    }
}
