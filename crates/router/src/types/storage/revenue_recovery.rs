use std::fmt::Debug;

use common_enums::enums;
use common_utils::id_type;
use hyperswitch_domain_models::{
    business_profile, merchant_account, merchant_connector_account, merchant_key_store,
};
use router_env::logger;

use crate::{db::StorageInterface, workflows::revenue_recovery};
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RevenueRecoveryWorkflowTrackingData {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub global_payment_id: id_type::GlobalPaymentId,
    pub payment_attempt_id: id_type::GlobalAttemptId,
    pub billing_mca_id: id_type::MerchantConnectorAccountId,
    pub revenue_recovery_retry: enums::RevenueRecoveryAlgorithmType,
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
        db: &dyn StorageInterface,
        merchant_id: &id_type::MerchantId,
        retry_count: i32,
    ) -> Option<time::PrimitiveDateTime> {
        match self.retry_algorithm {
            enums::RevenueRecoveryAlgorithmType::Monitoring => {
                logger::error!("Monitoring type found for Revenue Recovery retry payment");
                None
            }
            enums::RevenueRecoveryAlgorithmType::Cascading => {
                revenue_recovery::get_schedule_time_to_retry_mit_payments(
                    db,
                    merchant_id,
                    retry_count,
                )
                .await
            }
            enums::RevenueRecoveryAlgorithmType::Smart => {
                // TODO: Integrate the smart retry call to return back a schedule time
                None
            }
        }
    }
}
#[derive(Debug, serde::Deserialize, Clone, Default)]
pub struct RevenueRecoverySettings {
    pub monitoring_threshold_in_seconds: i64,
    pub retry_algorithm_type: enums::RevenueRecoveryAlgorithmType,
}
