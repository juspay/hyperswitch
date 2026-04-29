use analytics::errors::AnalyticsError;
use api_models::analytics::{AnalyticsRequest, ReportRequest};
use common_utils::errors::CustomResult;
use currency_conversion::types::ExchangeRates;
use router_env::logger;

use crate::core::currency::get_forex_exchange_rates;

pub async fn request_validator(
    req_type: AnalyticsRequest,
    state: &crate::routes::SessionState,
) -> CustomResult<Option<ExchangeRates>, AnalyticsError> {
    let forex_enabled = state.conf.analytics.get_inner().get_forex_enabled();
    let require_forex_functionality = req_type.requires_forex_functionality();

    let ex_rates = if forex_enabled && require_forex_functionality {
        logger::info!("Fetching forex exchange rates");
        Some(get_forex_exchange_rates(state.clone()).await?)
    } else {
        None
    };

    Ok(ex_rates)
}

pub fn validate_report_request(request: &ReportRequest) -> Result<(), AnalyticsError> {
    match request.return_url {
        Some(ref return_url) => return_url
            .verify_https_scheme()
            .map_err(AnalyticsError::InvalidReturnUrl),
        None => Ok(()),
    }
}
