use common_utils::errors::CustomResult;
use error_stack::ResultExt;

use crate::{
    core::errors::ApiErrorResponse,
    services::ApplicationResponse,
    utils::currency::{self, convert_currency, get_forex_rates},
    AppState,
};

pub async fn retrieve_forex(
    state: AppState,
) -> CustomResult<ApplicationResponse<currency::FxExchangeRatesCacheEntry>, ApiErrorResponse> {
    Ok(ApplicationResponse::Json(
        get_forex_rates(
            &state,
            state.conf.forex_api.call_delay,
            state.conf.forex_api.local_fetch_retry_delay,
            state.conf.forex_api.local_fetch_retry_count,
            #[cfg(feature = "kms")]
            &state.conf.kms,
            #[cfg(feature = "hashicorp-vault")]
            &state.conf.hc_vault,
        )
        .await
        .change_context(ApiErrorResponse::GenericNotFoundError {
            message: "Unable to fetch forex rates".to_string(),
        })?,
    ))
}

pub async fn convert_forex(
    state: AppState,
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
            #[cfg(feature = "kms")]
            &state.conf.kms,
            #[cfg(feature = "hashicorp-vault")]
            &state.conf.hc_vault,
        ))
        .await
        .change_context(ApiErrorResponse::InternalServerError)?,
    ))
}
