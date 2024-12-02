use analytics::errors::AnalyticsError;
use api_models::analytics::AnalyticsRequest;
use common_utils::errors::CustomResult;
use currency_conversion::types::ExchangeRates;

use crate::core::currency::get_forex_exchange_rates;

pub async fn request_validator(
    _req_type: AnalyticsRequest,
    state: &crate::routes::SessionState,
) -> CustomResult<(bool, Option<ExchangeRates>), AnalyticsError> {
    // other validation logic based on `req_type` goes here

    let ex_rates = if state.conf.analytics.get_inner().get_forex_enabled() {
        Some(get_forex_exchange_rates(state.clone()).await?)
    } else {
        None
    };

    Ok((ex_rates.is_some(), ex_rates))
}
