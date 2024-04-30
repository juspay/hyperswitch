use api_models::payments as api_payments;
use common_enums::enums;
use common_utils::errors::CustomResult;
use data_models::mandates::MandateData;
use diesel_models::Mandate;
use error_stack::ResultExt;

use crate::{
    core::{errors, payments},
    routes::SessionState,
    types::{api, domain},
};

pub async fn get_profile_id_for_mandate(
    state: &SessionState,
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

pub fn get_mandate_type(
    mandate_data: Option<api_payments::MandateData>,
    off_session: Option<bool>,
    setup_future_usage: Option<enums::FutureUsage>,
    customer_acceptance: Option<api_payments::CustomerAcceptance>,
    token: Option<String>,
) -> CustomResult<Option<api::MandateTransactionType>, errors::ValidationError> {
    match (
        mandate_data.clone(),
        off_session,
        setup_future_usage,
        customer_acceptance.or(mandate_data.and_then(|m_data| m_data.customer_acceptance)),
        token,
    ) {
        (Some(_), Some(_), Some(enums::FutureUsage::OffSession), Some(_), Some(_)) => {
            Err(errors::ValidationError::InvalidValue {
                message: "Expected one out of recurring_details and mandate_data but got both"
                    .to_string(),
            }
            .into())
        }
        (_, _, Some(enums::FutureUsage::OffSession), Some(_), Some(_))
        | (_, _, Some(enums::FutureUsage::OffSession), Some(_), _)
        | (Some(_), _, Some(enums::FutureUsage::OffSession), _, _) => {
            Ok(Some(api::MandateTransactionType::NewMandateTransaction))
        }

        (_, _, Some(enums::FutureUsage::OffSession), _, Some(_)) | (_, Some(_), _, _, _) => Ok(
            Some(api::MandateTransactionType::RecurringMandateTransaction),
        ),

        _ => Ok(None),
    }
}
#[derive(Clone)]
pub struct MandateGenericData {
    pub token: Option<String>,
    pub payment_method: Option<enums::PaymentMethod>,
    pub payment_method_type: Option<enums::PaymentMethodType>,
    pub mandate_data: Option<MandateData>,
    pub recurring_mandate_payment_data: Option<payments::RecurringMandatePaymentData>,
    pub mandate_connector: Option<payments::MandateConnectorDetails>,
    pub payment_method_info: Option<diesel_models::PaymentMethod>,
}
