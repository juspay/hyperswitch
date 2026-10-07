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
use hyperswitch_masking::PeekInterface;
use router_env::logger;
use serde::{Deserialize, Serialize};

use crate::{
    core::revenue_recovery::schedule::StaticLadderProgress, db::StorageInterface,
    routes::SessionState, types, workflows::revenue_recovery,
};
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RevenueRecoveryWorkflowTrackingData {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub global_payment_id: id_type::GlobalPaymentId,
    pub payment_attempt_id: id_type::GlobalAttemptId,
    pub billing_mca_id: id_type::MerchantConnectorAccountId,
    pub revenue_recovery_retry: enums::RevenueRecoveryAlgorithmType,
    pub invoice_scheduled_time: Option<time::PrimitiveDateTime>,
    /// Standardised error code for the failed attempt that motivated this retry chain,
    #[serde(default)]
    pub prev_attempt_error_code: Option<enums::StandardisedCode>,
    /// Ladder position carried across an invoice's recovery. Inert since the static ladder was
    /// removed from the decision — nothing advances it and nothing reads it to pick a time — but
    /// still seeded and still persisted, so it keeps round-tripping until it is removed properly.
    /// Only meaningful on the CALCULATE row, which is reopened rather than recreated and so
    /// survives for the whole recovery lifecycle of an invoice.
    #[serde(default)]
    pub static_ladder_progress: Option<StaticLadderProgress>,
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
                let connector = payment_attempt.connector.as_ref().and_then(|c| {
                    c.parse::<common_enums::connector_enums::Connector>()
                        .map_err(|e| {
                            logger::error!(
                                "Failed to parse connector {:?} for payment_attempt {:?}: {:?}",
                                c,
                                payment_attempt.payment_id,
                                e
                            )
                        })
                        .ok()
                })?;
                let dimensions = crate::core::configs::dimension_state::Dimensions::new()
                    .with_processor_merchant_id(merchant_id.clone().into())
                    .with_connector(connector);
                revenue_recovery::get_schedule_time_to_retry_mit_payments(
                    state.store.as_ref(),
                    state.superposition_service.as_ref(),
                    &dimensions,
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
    #[serde(default)]
    pub retry_stats_lock: RetryStatsLockSettings,
    /// Fallback hour-of-day (UTC, 0–23) the retry model schedules at when a cluster has no
    /// usable hour-of-day history. Overridable per environment via the config file's
    /// `[revenue_recovery]` section or the `ROUTER__REVENUE_RECOVERY__DEFAULT_RETRY_HOUR_UTC` env var;
    /// when omitted it defaults to noon UTC (see [`DefaultRetryHour`]).
    #[serde(default)]
    pub default_retry_hour_utc: DefaultRetryHour,
    /// Minimum probability the systematic-k sampler gives every candidate day, so no day is ruled
    /// out on a model that has never tried it. `ROUTER__REVENUE_RECOVERY__EXPLORATION_FLOOR`.
    #[serde(default)]
    pub exploration_floor: ExplorationFloor,
    /// Distance within which two day values count as tied and split their share of the budget
    /// equally. `ROUTER__REVENUE_RECOVERY__TIE_TOLERANCE`. See [`TieTolerance`] before raising it.
    #[serde(default)]
    pub tie_tolerance: TieTolerance,
}

/// Redis distributed-lock settings for revenue-recovery retry-stats recording
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryStatsLockSettings {
    /// TTL on the per-cluster-key lock
    pub redis_lock_expiry_seconds: u32,
    /// Delay between successive attempts to acquire a contended lock.
    pub delay_between_retries_in_milliseconds: u32,
}

impl Default for RetryStatsLockSettings {
    fn default() -> Self {
        Self {
            redis_lock_expiry_seconds: 10,
            delay_between_retries_in_milliseconds: 100,
        }
    }
}

impl RetryStatsLockSettings {
    pub fn lock_retries(&self) -> u32 {
        self.redis_lock_expiry_seconds
            .saturating_mul(1000)
            .checked_div(self.delay_between_retries_in_milliseconds)
            .unwrap_or(1)
    }
}

/// Fallback hour-of-day (UTC, 0–23) for the retry model. A newtype so its `Default` (noon UTC = 12) is
/// carried automatically by both `#[derive(Default)]` on the settings and serde's `#[serde(default)]`
/// — the value lives in exactly one place, with no field-list repetition.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct DefaultRetryHour(pub u8);

impl Default for DefaultRetryHour {
    fn default() -> Self {
        Self(12)
    }
}

impl DefaultRetryHour {
    /// The hour to schedule at: the configured one when it names a real hour, the default
    /// otherwise. Out of range it would degrade to midnight downstream, which is a legal-looking
    /// time nobody would question — so it warns instead of being quietly clamped.
    pub fn resolve(self) -> u8 {
        if self.0 <= 23 {
            return self.0;
        }
        let fallback = Self::default().0;
        logger::warn!(
            configured = self.0,
            fallback,
            "revenue_recovery.default_retry_hour_utc is outside 0-23; using the default"
        );
        fallback
    }
}

/// Exploration floor for the systematic-k sampler: the minimum inclusion probability every
/// candidate day is guaranteed. A business dial — higher explores more and uses the model less.
/// Values above `budget / window_length` are infeasible and get clamped at the point of use.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct ExplorationFloor(pub f64);

impl Default for ExplorationFloor {
    fn default() -> Self {
        Self(0.10)
    }
}

impl ExplorationFloor {
    /// The floor to sample with: the configured one when it is a probability, the default
    /// otherwise. Outside `[0, 1)` it is not one, and the sampler clamps it at the point of use —
    /// so without this the misconfiguration would never surface anywhere.
    pub fn resolve(self) -> f64 {
        if (0.0..1.0).contains(&self.0) {
            return self.0;
        }
        let fallback = Self::default().0;
        logger::warn!(
            configured = self.0,
            fallback,
            "revenue_recovery.exploration_floor is not in [0, 1); using the default"
        );
        fallback
    }
}

/// Distance within which two candidate-day values are treated as tied.
///
/// Raising this is not safe by inspection. A merge only changes anything where the group spans the
/// rank the budget ran out at, and there it averages a near-certain day with a floored one — a
/// swing of up to `1 − exploration_floor`. On production stats the smallest gap at that rank was
/// 4.2e-05, so above that this starts reallocating probability between days the model ranks apart.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct TieTolerance(pub f64);

impl Default for TieTolerance {
    fn default() -> Self {
        Self(1e-4)
    }
}

impl TieTolerance {
    /// The tolerance to group by: the configured one when it is a usable distance, the default
    /// otherwise. Negative merges nothing and non-finite merges everything, since every `<=`
    /// against `NaN` is false and every one against infinity is true.
    pub fn resolve(self) -> f64 {
        if self.0 >= 0.0 && self.0.is_finite() {
            return self.0;
        }
        let fallback = Self::default().0;
        logger::warn!(
            configured = self.0,
            fallback,
            "revenue_recovery.tie_tolerance is negative or not finite; using the default"
        );
        fallback
    }
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

#[cfg(test)]
mod tunable_tests {
    use super::*;

    #[test]
    fn a_usable_value_is_returned_as_configured() {
        assert_eq!(DefaultRetryHour(0).resolve(), 0);
        assert_eq!(DefaultRetryHour(23).resolve(), 23);
        assert!((ExplorationFloor(0.0).resolve() - 0.0).abs() < f64::EPSILON);
        assert!((ExplorationFloor(0.25).resolve() - 0.25).abs() < f64::EPSILON);
        assert!((TieTolerance(0.0).resolve() - 0.0).abs() < f64::EPSILON);
        assert!((TieTolerance(1e-3).resolve() - 1e-3).abs() < f64::EPSILON);
    }

    #[test]
    fn an_unusable_value_falls_back_to_the_default() {
        // Each guard's far side, including the boundary that is deliberately excluded: a floor of
        // exactly 1.0 means "always", which is not a floor.
        assert_eq!(
            DefaultRetryHour(24).resolve(),
            DefaultRetryHour::default().0
        );
        for floor in [1.0, -0.1, f64::NAN, f64::INFINITY] {
            assert!(
                (ExplorationFloor(floor).resolve() - ExplorationFloor::default().0).abs()
                    < f64::EPSILON
            );
        }
        for tolerance in [-1e-9, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                (TieTolerance(tolerance).resolve() - TieTolerance::default().0).abs()
                    < f64::EPSILON
            );
        }
    }
}
