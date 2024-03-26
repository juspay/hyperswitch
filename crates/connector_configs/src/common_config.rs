use api_models::{payment_methods, payments};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ZenApplePay {
    pub terminal_uuid: Option<String>,
    pub pay_wall_secret: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
#[serde(untagged)]
pub enum ApplePayData {
    ApplePay(payments::ApplePayMetadata),
    ApplePayCombined(payments::ApplePayCombinedMetadata),
    Zen(ZenApplePay),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpayDashboardPayLoad {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_merchant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "stripe:version")]
    pub stripe_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(
        serialize = "stripe_publishable_key",
        deserialize = "stripe:publishable_key"
    ))]
    #[serde(alias = "stripe:publishable_key")]
    #[serde(alias = "stripe_publishable_key")]
    pub stripe_publishable_key: Option<String>,
    pub merchant_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_id: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ZenGooglePay {
    pub terminal_uuid: Option<String>,
    pub pay_wall_secret: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
#[serde(untagged)]
pub enum GooglePayData {
    Standard(GpayDashboardPayLoad),
    Zen(ZenGooglePay),
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
#[serde(untagged)]
pub enum GoogleApiModelData {
    Standard(payments::GpayMetaData),
    Zen(ZenGooglePay),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct PaymentMethodsEnabled {
    pub payment_method: api_models::enums::PaymentMethod,
    pub payment_method_types: Option<Vec<payment_methods::RequestPaymentMethodTypes>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ApiModelMetaData {
    pub merchant_config_currency: Option<api_models::enums::Currency>,
    pub merchant_account_id: Option<String>,
    pub account_name: Option<String>,
    pub terminal_id: Option<String>,
    pub merchant_id: Option<String>,
    pub google_pay: Option<GoogleApiModelData>,
    pub apple_pay: Option<ApplePayData>,
    pub apple_pay_combined: Option<ApplePayData>,
    pub endpoint_prefix: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, ToSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CardProvider {
    pub payment_method_type: api_models::enums::CardNetwork,
    /// List of currencies accepted or has the processing capabilities of the processor
    #[schema(example = json!(
            {
                "type": "specific_accepted",
                "list": ["USD", "INR"]
            }
        ), value_type = Option<AcceptedCurrencies>)]
    pub accepted_currencies: Option<api_models::admin::AcceptedCurrencies>,
    #[schema(example = json!(
        {
            "type": "specific_accepted",
            "list": ["UK", "AU"]
        }
    ), value_type = Option<AcceptedCountries>)]
    pub accepted_countries: Option<api_models::admin::AcceptedCountries>,
}
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, ToSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Provider {
    pub payment_method_type: api_models::enums::PaymentMethodType,
    /// List of currencies accepted or has the processing capabilities of the processor
    #[schema(example = json!(
            {
                "type": "specific_accepted",
                "list": ["USD", "INR"]
            }
        ), value_type = Option<AcceptedCurrencies>)]
    pub accepted_currencies: Option<api_models::admin::AcceptedCurrencies>,
    #[schema(example = json!(
        {
            "type": "specific_accepted",
            "list": ["UK", "AU"]
        }
    ), value_type = Option<AcceptedCountries>)]
    pub accepted_countries: Option<api_models::admin::AcceptedCountries>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConnectorApiIntegrationPayload {
    pub connector_type: String,
    pub profile_id: String,
    pub connector_name: api_models::enums::Connector,
    #[serde(skip_deserializing)]
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,
    pub merchant_connector_id: Option<String>,
    pub disabled: bool,
    pub test_mode: bool,
    pub payment_methods_enabled: Option<Vec<PaymentMethodsEnabled>>,
    pub metadata: Option<ApiModelMetaData>,
    pub connector_webhook_details: Option<api_models::admin::MerchantConnectorWebhookDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DashboardPaymentMethodPayload {
    pub payment_method: api_models::enums::PaymentMethod,
    pub payment_method_type: String,
    pub provider: Option<Vec<Provider>>,
    pub card_provider: Option<Vec<CardProvider>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case")]
pub struct DashboardRequestPayload {
    pub connector: api_models::enums::Connector,
    pub payment_methods_enabled: Option<Vec<DashboardPaymentMethodPayload>>,
    pub metadata: Option<DashboardMetaData>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct DashboardMetaData {
    pub merchant_config_currency: Option<api_models::enums::Currency>,
    pub merchant_account_id: Option<String>,
    pub account_name: Option<String>,
    pub terminal_id: Option<String>,
    pub merchant_id: Option<String>,
    pub google_pay: Option<GooglePayData>,
    pub apple_pay: Option<ApplePayData>,
    pub apple_pay_combined: Option<ApplePayData>,
    pub endpoint_prefix: Option<String>,
}
