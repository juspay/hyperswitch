use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use masking::PeekInterface;
use router_env::{instrument, tracing};

use crate::errors::api_error_response::ApiErrorResponse;

#[instrument(skip_all)]
pub fn validate_card_expiry(
    card_exp_month: &masking::Secret<String>,
    card_exp_year: &masking::Secret<String>,
) -> CustomResult<(), ApiErrorResponse> {
    let exp_month = card_exp_month
        .peek()
        .to_string()
        .parse::<u8>()
        .change_context(ApiErrorResponse::InvalidDataValue {
            field_name: "card_exp_month",
        })?;
    let month = ::cards::CardExpirationMonth::try_from(exp_month).change_context(
        ApiErrorResponse::PreconditionFailed {
            message: "Invalid Expiry Month".to_string(),
        },
    )?;

    let mut year_str = card_exp_year.peek().to_string();
    if year_str.len() == 2 {
        year_str = format!("20{year_str}");
    }
    let exp_year = year_str
        .parse::<u16>()
        .change_context(ApiErrorResponse::InvalidDataValue {
            field_name: "card_exp_year",
        })?;
    let year = ::cards::CardExpirationYear::try_from(exp_year).change_context(
        ApiErrorResponse::PreconditionFailed {
            message: "Invalid Expiry Year".to_string(),
        },
    )?;

    let card_expiration = ::cards::CardExpiration { month, year };
    let is_expired =
        card_expiration
            .is_expired()
            .change_context(ApiErrorResponse::PreconditionFailed {
                message: "Invalid card data".to_string(),
            })?;
    if is_expired {
        Err(report!(ApiErrorResponse::PreconditionFailed {
            message: "Card Expired".to_string()
        }))?
    }

    Ok(())
}
