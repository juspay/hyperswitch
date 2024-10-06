use common_utils::{events::ApiEventMetric, types::MinorUnit};

/// QueryParams to be send to convert the amount -> from_currency -> to_currency
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CurrencyConversionParams {
    pub amount: MinorUnit,
    pub to_currency: String,
    pub from_currency: String,
}

/// Response to be send for convert currency route
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CurrencyConversionResponse {
    pub converted_amount: String,
    pub currency: String,
}

impl ApiEventMetric for CurrencyConversionResponse {}
impl ApiEventMetric for CurrencyConversionParams {}
