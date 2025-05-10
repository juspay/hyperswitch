use std::fmt::Debug;

use common_utils::id_type;
use hyperswitch_domain_models::{
    business_profile, merchant_account, merchant_connector_account, merchant_key_store,
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RevenueRecoveryWorkflowTrackingData {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub global_payment_id: id_type::GlobalPaymentId,
    pub payment_attempt_id: id_type::GlobalAttemptId,
    pub billing_mca_id: id_type::MerchantConnectorAccountId,
}

#[derive(Debug, Clone)]
pub struct RevenueRecoveryPaymentData {
    pub merchant_account: merchant_account::MerchantAccount,
    pub profile: business_profile::Profile,
    pub key_store: merchant_key_store::MerchantKeyStore,
    pub billing_mca: merchant_connector_account::MerchantConnectorAccount,
}
