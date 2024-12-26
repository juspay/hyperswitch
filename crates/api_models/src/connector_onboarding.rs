use common_utils::id_type;

use super::{admin, enums};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct ActionUrlRequest {
    pub connector: enums::Connector,
    pub connector_id: id_type::MerchantConnectorAccountId,
    pub return_url: String,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ActionUrlResponse {
    PayPal(PayPalActionUrlResponse),
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct OnboardingSyncRequest {
    pub profile_id: id_type::ProfileId,
    pub connector_id: id_type::MerchantConnectorAccountId,
    pub connector: enums::Connector,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct PayPalActionUrlResponse {
    pub action_url: String,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OnboardingStatus {
    PayPal(PayPalOnboardingStatus),
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PayPalOnboardingStatus {
    AccountNotFound,
    PaymentsNotReceivable,
    PpcpCustomDenied,
    MorePermissionsNeeded,
    EmailNotVerified,
    Success(PayPalOnboardingDone),
    ConnectorIntegrated(Box<admin::MerchantConnectorResponse>),
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct PayPalOnboardingDone {
    pub payer_id: id_type::MerchantId,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct PayPalIntegrationDone {
    pub connector_id: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct ResetTrackingIdRequest {
    pub connector_id: id_type::MerchantConnectorAccountId,
    pub connector: enums::Connector,
}
