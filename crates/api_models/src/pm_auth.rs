use common_enums::{PaymentMethod, PaymentMethodType};
use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    id_type, impl_api_event_type,
};

use crate::enums as api_enums;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LinkTokenCreateRequest {
    pub language: Option<String>, // optional language field to be passed
    pub client_secret: Option<String>, // client secret to be passed in req body
    pub payment_id: id_type::PaymentId, // payment_id to be passed in req body for redis pm_auth connector name fetch
    pub payment_method: PaymentMethod,  // payment_method to be used for filtering pm_auth connector
    pub payment_method_type: PaymentMethodType, // payment_method_type to be used for filtering pm_auth connector
    pub client_platform: api_enums::ClientPlatform, // Client Platform to perform platform based processing
    pub android_package_name: Option<String>, // Android Package name to be sent for Android platform
    pub redirect_uri: Option<String>, // Merchant redirect_uri to be sent in case of IOS platform
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LinkTokenCreateResponse {
    pub link_token: String, // link_token received in response
    pub connector: String,  // pm_auth connector name in response
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]

pub struct ExchangeTokenCreateRequest {
    pub public_token: String,
    pub client_secret: Option<String>,
    pub payment_id: id_type::PaymentId,
    pub payment_method: PaymentMethod,
    pub payment_method_type: PaymentMethodType,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExchangeTokenCreateResponse {
    pub access_token: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodAuthConfig {
    pub enabled_payment_methods: Vec<PaymentMethodAuthConnectorChoice>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodAuthConnectorChoice {
    pub payment_method: PaymentMethod,
    pub payment_method_type: PaymentMethodType,
    pub connector_name: String,
    pub mca_id: id_type::MerchantConnectorAccountId,
}

impl_api_event_type!(
    Miscellaneous,
    (
        LinkTokenCreateRequest,
        LinkTokenCreateResponse,
        ExchangeTokenCreateRequest,
        ExchangeTokenCreateResponse
    )
);
