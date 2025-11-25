use api_models::payments as api_payments;
use common_enums::enums;
use common_types::payments as common_payments_types;
use common_utils::errors::CustomResult;
use diesel_models::Mandate;
use error_stack::ResultExt;
use hyperswitch_domain_models::mandates::MandateData;

use crate::{
    core::{errors, payments},
    routes::SessionState,
    types::{api, domain},
};

#[cfg(feature = "v1")]
pub async fn get_profile_id_for_mandate(
    state: &SessionState,
    platform: &domain::Platform,
    mandate: Mandate,
) -> CustomResult<common_utils::id_type::ProfileId, errors::ApiErrorResponse> {
    let profile_id = if let Some(ref payment_id) = mandate.original_payment_id {
        let pi = state
            .store
            .find_payment_intent_by_payment_id_merchant_id(
                payment_id,
                platform.get_processor().get_account().get_id(),
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
        let profile_id =
            pi.profile_id
                .clone()
                .ok_or(errors::ApiErrorResponse::ProfileNotFound {
                    id: pi
                        .profile_id
                        .map(|profile_id| profile_id.get_string_repr().to_owned())
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
    customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    token: Option<String>,
    payment_method: Option<enums::PaymentMethod>,
) -> CustomResult<Option<api::MandateTransactionType>, errors::ValidationError> {
    match (
        mandate_data.clone(),
        off_session,
        setup_future_usage,
        customer_acceptance.or(mandate_data.and_then(|m_data| m_data.customer_acceptance)),
        token,
        payment_method,
    ) {
        (Some(_), Some(_), Some(enums::FutureUsage::OffSession), Some(_), Some(_), _) => {
            Err(errors::ValidationError::InvalidValue {
                message: "Expected one out of recurring_details and mandate_data but got both"
                    .to_string(),
            }
            .into())
        }
        (_, _, Some(enums::FutureUsage::OffSession), Some(_), Some(_), _)
        | (_, _, Some(enums::FutureUsage::OffSession), Some(_), _, _)
        | (Some(_), _, Some(enums::FutureUsage::OffSession), _, _, _) => {
            Ok(Some(api::MandateTransactionType::NewMandateTransaction))
        }

        (_, _, Some(enums::FutureUsage::OffSession), _, Some(_), _)
        | (_, Some(_), _, _, _, _)
        | (_, _, Some(enums::FutureUsage::OffSession), _, _, Some(enums::PaymentMethod::Wallet)) => {
            Ok(Some(
                api::MandateTransactionType::RecurringMandateTransaction,
            ))
        }

        _ => Ok(None),
    }
}
#[derive(Clone)]
pub struct MandateGenericData {
    pub token: Option<String>,
    pub payment_method: Option<enums::PaymentMethod>,
    pub payment_method_type: Option<enums::PaymentMethodType>,
    pub mandate_data: Option<MandateData>,
    pub recurring_mandate_payment_data:
        Option<hyperswitch_domain_models::router_data::RecurringMandatePaymentData>,
    pub mandate_connector: Option<payments::MandateConnectorDetails>,
    pub payment_method_info: Option<domain::PaymentMethod>,
}
