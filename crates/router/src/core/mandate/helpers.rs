use common_utils::errors::CustomResult;
use diesel_models::Mandate;
use error_stack::ResultExt;

use crate::{core::errors, routes::AppState, types::domain};

pub async fn get_profile_id_for_mandate(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    mandate: Mandate,
) -> CustomResult<String, errors::ApiErrorResponse> {
    let profile_id = if let Some(ref payment_id) = mandate.original_payment_id {
        let pi = state
            .store
            .find_payment_intent_by_payment_id_merchant_id(
                payment_id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
        let profile_id =
            pi.profile_id
                .clone()
                .ok_or(errors::ApiErrorResponse::BusinessProfileNotFound {
                    id: pi
                        .profile_id
                        .unwrap_or_else(|| "Profile id is Null".to_string()),
                })?;
        Ok(profile_id)
    } else {
        Err(errors::ApiErrorResponse::PaymentNotFound)
    }?;
    Ok(profile_id)
}
