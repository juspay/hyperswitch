use analytics::errors::AnalyticsError;
use common_utils::errors::CustomResult;
use currency_conversion::types::ExchangeRates;
use error_stack::ResultExt;

use crate::{
    core::errors::ApiErrorResponse,
    services::ApplicationResponse,
    utils::currency::{self, convert_currency, get_forex_rates},
    SessionState,
};

pub async fn retrieve_forex(
    state: SessionState,
) -> CustomResult<ApplicationResponse<currency::FxExchangeRatesCacheEntry>, ApiErrorResponse> {
    let forex_api = state.conf.forex_api.get_inner();
    Ok(ApplicationResponse::Json(
        get_forex_rates(&state, forex_api.call_delay)
            .await
            .change_context(ApiErrorResponse::GenericNotFoundError {
                message: "Unable to fetch forex rates".to_string(),
            })?,
    ))
}

pub async fn convert_forex(
    state: SessionState,
    amount: i64,
    to_currency: String,
    from_currency: String,
) -> CustomResult<
    ApplicationResponse<api_models::currency::CurrencyConversionResponse>,
    ApiErrorResponse,
> {
    Ok(ApplicationResponse::Json(
        Box::pin(convert_currency(
            state.clone(),
            amount,
            to_currency,
            from_currency,
        ))
        .await
        .change_context(ApiErrorResponse::InternalServerError)?,
    ))
}

pub async fn get_forex_exchange_rates(
    state: SessionState,
) -> CustomResult<ExchangeRates, AnalyticsError> {
    let forex_api = state.conf.forex_api.get_inner();
    let rates = get_forex_rates(&state, forex_api.call_delay)
        .await
        .change_context(AnalyticsError::ForexFetchFailed)?;

    Ok((*rates.data).clone())
}
