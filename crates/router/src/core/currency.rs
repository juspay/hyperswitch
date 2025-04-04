use analytics::errors::AnalyticsError;
use common_utils::errors::CustomResult;
use currency_conversion::types::ExchangeRates;
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    consts::DEFAULT_ANALYTICS_FOREX_RETRY_ATTEMPTS,
    core::errors::ApiErrorResponse,
    services::ApplicationResponse,
    utils::currency::{self, convert_currency, get_forex_rates, ForexError as ForexCacheError},
    SessionState,
};

pub async fn retrieve_forex(
    state: SessionState,
) -> CustomResult<ApplicationResponse<currency::FxExchangeRatesCacheEntry>, ApiErrorResponse> {
    let forex_api = state.conf.forex_api.get_inner();
    Ok(ApplicationResponse::Json(
        get_forex_rates(&state, forex_api.data_expiration_delay_in_seconds)
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
    let mut attempt = 1;

    logger::info!("Starting forex exchange rates fetch");
    loop {
        logger::info!("Attempting to fetch forex rates - Attempt {attempt} of {DEFAULT_ANALYTICS_FOREX_RETRY_ATTEMPTS}");

        match get_forex_rates(&state, forex_api.data_expiration_delay_in_seconds).await {
            Ok(rates) => {
                logger::info!("Successfully fetched forex rates");
                return Ok((*rates.data).clone());
            }
            Err(error) => {
                let is_retryable = matches!(
                    error.current_context(),
                    ForexCacheError::CouldNotAcquireLock
                        | ForexCacheError::EntryNotFound
                        | ForexCacheError::ForexDataUnavailable
                        | ForexCacheError::LocalReadError
                        | ForexCacheError::LocalWriteError
                        | ForexCacheError::RedisConnectionError
                        | ForexCacheError::RedisLockReleaseFailed
                        | ForexCacheError::RedisWriteError
                        | ForexCacheError::WriteLockNotAcquired
                );

                if !is_retryable {
                    return Err(error.change_context(AnalyticsError::ForexFetchFailed));
                }

                if attempt >= DEFAULT_ANALYTICS_FOREX_RETRY_ATTEMPTS {
                    logger::error!("Failed to fetch forex rates after {DEFAULT_ANALYTICS_FOREX_RETRY_ATTEMPTS} attempts");
                    return Err(error.change_context(AnalyticsError::ForexFetchFailed));
                }
                logger::warn!(
                    "Forex rates fetch failed with retryable error, retrying in {attempt} seconds"
                );
                tokio::time::sleep(std::time::Duration::from_secs(attempt * 2)).await;
                attempt += 1;
            }
        }
    }
}
