use analytics::errors::AnalyticsError;
use common_utils::errors::CustomResult;
use currency_conversion::types::ExchangeRates;
use error_stack::ResultExt;
use router_env::logger;

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

// 
// pub async fn get_forex_exchange_rates(
//     state: SessionState,
// ) -> CustomResult<ExchangeRates, AnalyticsError> {
//     let forex_api = state.conf.forex_api.get_inner();
//     let rates = get_forex_rates(&state, forex_api.call_delay)
//         .await
//         .change_context(AnalyticsError::ForexFetchFailed)?;

//     Ok((*rates.data).clone())
// }

pub async fn get_forex_exchange_rates(
    state: SessionState,
) -> CustomResult<ExchangeRates, AnalyticsError> {
    let forex_api = state.conf.forex_api.get_inner();
    let max_attempts = 3;
    let mut attempt = 1;
    
    logger::info!("Starting forex exchange rates fetch");
    loop {
        logger::info!("Attempting to fetch forex rates - Attempt {attempt} of {max_attempts}");
        
        match get_forex_rates(&state, forex_api.call_delay).await {
            Ok(rates) => {
                logger::info!("Successfully fetched forex rates");
                return Ok((*rates.data).clone())
            },
            Err(e) => {
                if attempt >= max_attempts {
                    logger::error!("Failed to fetch forex rates after {max_attempts} attempts");
                    return Err(e.change_context(AnalyticsError::ForexFetchFailed));
                }
                logger::warn!("Forex rates fetch failed, retrying in {attempt} seconds");
                tokio::time::sleep(std::time::Duration::from_secs(attempt)).await;
                attempt += 1;
            }
        }
    }
}
