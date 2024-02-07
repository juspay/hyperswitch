use std::marker::PhantomData;

use api_models::payments;
use common_enums::{AuthenticationType, PaymentMethod};
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;

use crate::{
    core::{
        errors::{self, RouterResult},
        payments::helpers as payments_helpers,
    },
    types::{self, domain},
};

const IRRELEVANT_PAYMENT_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_payment_id_in_AUTHENTICATION_flow";
const IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_attempt_id_in_AUTHENTICATION_flow";
const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_AUTHENTICATION_flow";

pub fn construct_authentication_router_data(
    authentication_provider: String,
    payment_method_data: payments::PaymentMethodData,
    payment_method: PaymentMethod,
    billing_address: api_models::payments::Address,
    shipping_address: api_models::payments::Address,
    browser_details: types::BrowserInformation,
    amount: Option<i64>,
    currency: Option<common_enums::Currency>,
    message_category: types::api::authentication::MessageCategory,
    device_channel: String,
    merchant_account: domain::MerchantAccount,
    merchant_connector_account: payments_helpers::MerchantConnectorAccountType,
    authentication_data: super::types::AuthenticationData,
    return_url: Option<String>,
    sdk_information: Option<api_models::payments::SDKInformation>,
) -> RouterResult<types::ConnectorAuthenticationRouterData> {
    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.merchant_id.clone(),
        connector: authentication_provider,
        payment_id: IRRELEVANT_PAYMENT_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        return_url: Some("".to_string()),
        payment_method_id: Some("".to_string()),
        address: types::PaymentAddress::default(),
        auth_type: AuthenticationType::ThreeDs,
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: Some(0),
        request: types::ConnectorAuthenticationRequestData {
            payment_method_data,
            billing_address,
            shipping_address,
            browser_details,
            amount,
            currency,
            message_category,
            device_channel,
            authentication_data,
            return_url,
            sdk_information,
        },
        response: Err(types::ErrorResponse::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        customer_id: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
    };
    Ok(router_data)
}

pub fn construct_post_authentication_router_data(
    authentication_provider: String,
    merchant_account: domain::MerchantAccount,
    merchant_connector_account: payments_helpers::MerchantConnectorAccountType,
    authentication_data: super::types::AuthenticationData,
) -> RouterResult<types::ConnectorPostAuthenticationRouterData> {
    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.merchant_id.clone(),
        connector: authentication_provider,
        payment_id: IRRELEVANT_PAYMENT_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method: PaymentMethod::default(),
        connector_auth_type: auth_type,
        description: None,
        return_url: Some("".to_string()),
        payment_method_id: Some("".to_string()),
        address: types::PaymentAddress::default(),
        auth_type: AuthenticationType::ThreeDs,
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: Some(0),
        request: types::ConnectorPostAuthenticationRequestData {
            authentication_data,
        },
        response: Err(types::ErrorResponse::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        customer_id: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        payment_method_balance: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
    };
    Ok(router_data)
}
