use std::marker::PhantomData;

use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use diesel_models::Mandate;
use error_stack::ResultExt;

use crate::{
    core::{errors, payments::helpers},
    types::{self, domain, PaymentAddress},
};
const IRRELEVANT_PAYMENT_ID_IN_MANDATE_REVOKE_FLOW: &str =
    "irrelevant_payment_id_in_mandate_revoke_flow";

const IRRELEVANT_ATTEMPT_ID_IN_MANDATE_REVOKE_FLOW: &str =
    "irrelevant_attempt_id_in_mandate_revoke_flow";

const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_MANDATE_REVOKE_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_mandate_revoke_flow";

pub async fn construct_mandate_revoke_router_data(
    merchant_connector_account: helpers::MerchantConnectorAccountType,
    merchant_account: &domain::MerchantAccount,
    mandate: Mandate,
) -> CustomResult<types::MandateRevokeRouterData, errors::ApiErrorResponse> {
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.merchant_id.clone(),
        customer_id: Some(mandate.customer_id),
        connector_customer: None,
        connector: mandate.connector,
        payment_id: mandate
            .original_payment_id
            .unwrap_or_else(|| IRRELEVANT_PAYMENT_ID_IN_MANDATE_REVOKE_FLOW.to_string()),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_MANDATE_REVOKE_FLOW.to_string(),
        status: diesel_models::enums::AttemptStatus::default(),
        payment_method: diesel_models::enums::PaymentMethod::default(),
        connector_auth_type: auth_type,
        description: None,
        return_url: None,
        address: PaymentAddress::default(),
        auth_type: diesel_models::enums::AuthenticationType::default(),
        connector_meta_data: None,
        amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request: types::MandateRevokeRequestData {
            mandate_id: mandate.mandate_id,
            connector_mandate_id: mandate.connector_mandate_id,
        },
        response: Err(types::ErrorResponse::get_not_implemented()),
        payment_method_id: None,
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_MANDATE_REVOKE_FLOW.to_string(),
        test_mode: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
    };

    Ok(router_data)
}
