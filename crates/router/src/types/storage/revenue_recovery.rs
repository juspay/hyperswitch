use std::{collections::HashMap, fmt::Debug};

use common_enums::enums::{self, CardNetwork};
use common_utils::{date_time, ext_traits::ValueExt, id_type};
use error_stack::ResultExt;
use external_services::grpc_client::{self as external_grpc_client, GrpcHeaders};
use hyperswitch_domain_models::{
    business_profile, merchant_account, merchant_connector_account, merchant_key_store,
    payment_method_data::{Card, PaymentMethodData},
    payments::{payment_attempt::PaymentAttempt, PaymentIntent, PaymentStatusData},
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
    pub psync_data: Option<PaymentStatusData<types::api::PSync>>,
}
impl RevenueRecoveryPaymentData {
    pub async fn get_schedule_time_based_on_retry_type(
        &self,
        state: &SessionState,
        merchant_id: &id_type::MerchantId,
        retry_count: i32,
        payment_attempt: &PaymentAttempt,
        payment_intent: &PaymentIntent,
        is_hard_decline: bool,
    ) -> Option<time::PrimitiveDateTime> {
        if is_hard_decline {
            logger::info!("Hard Decline encountered");
            return None;
        }
        match self.retry_algorithm {
            enums::RevenueRecoveryAlgorithmType::Monitoring => {
                logger::error!("Monitoring type found for Revenue Recovery retry payment");
                None
            }
            enums::RevenueRecoveryAlgorithmType::Cascading => {
                logger::info!("Cascading type found for Revenue Recovery retry payment");
                revenue_recovery::get_schedule_time_to_retry_mit_payments(
                    state.store.as_ref(),
                    merchant_id,
                    retry_count,
                )
                .await
            }
            enums::RevenueRecoveryAlgorithmType::Smart => None,
        }
    }
}

#[derive(Debug, serde::Deserialize, Clone, Default)]
pub struct RevenueRecoverySettings {
    pub monitoring_threshold_in_seconds: i64,
    pub retry_algorithm_type: enums::RevenueRecoveryAlgorithmType,
    pub recovery_timestamp: RecoveryTimestamp,
    pub card_config: RetryLimitsConfig,
    pub redis_ttl_in_seconds: i64,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct RecoveryTimestamp {
    pub initial_timestamp_in_seconds: i64,
    pub job_schedule_buffer_time_in_seconds: i64,
    pub reopen_workflow_buffer_time_in_seconds: i64,
    pub max_random_schedule_delay_in_seconds: i64,
    pub redis_ttl_buffer_in_seconds: i64,
    pub unretried_invoice_schedule_time_offset_seconds: i64,
}

impl Default for RecoveryTimestamp {
    fn default() -> Self {
        Self {
            initial_timestamp_in_seconds: 1,
            job_schedule_buffer_time_in_seconds: 15,
            reopen_workflow_buffer_time_in_seconds: 60,
            max_random_schedule_delay_in_seconds: 300,
            redis_ttl_buffer_in_seconds: 300,
            unretried_invoice_schedule_time_offset_seconds: 300,
        }
    }
}

#[derive(Debug, serde::Deserialize, Clone, Default)]
pub struct RetryLimitsConfig(pub HashMap<CardNetwork, NetworkRetryConfig>);

#[derive(Debug, serde::Deserialize, Clone, Default)]
pub struct NetworkRetryConfig {
    pub max_retries_per_day: i32,
    pub max_retry_count_for_thirty_day: i32,
}

impl RetryLimitsConfig {
    pub fn get_network_config(&self, network: Option<CardNetwork>) -> &NetworkRetryConfig {
        // Hardcoded fallback default config
        static DEFAULT_CONFIG: NetworkRetryConfig = NetworkRetryConfig {
            max_retries_per_day: 20,
            max_retry_count_for_thirty_day: 20,
        };

        if let Some(net) = network {
            self.0.get(&net).unwrap_or(&DEFAULT_CONFIG)
        } else {
            self.0.get(&CardNetwork::Visa).unwrap_or(&DEFAULT_CONFIG)
        }
    }
}
