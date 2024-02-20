use std::marker::PhantomData;

use api_models::payments;
use common_enums::PaymentMethod;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;

use crate::{
    core::{
        errors::{self, RouterResult},
        payments::helpers as payments_helpers,
    },
    types::{self, domain, storage},
};

const IRRELEVANT_PAYMENT_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_payment_id_in_AUTHENTICATION_flow";
const IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_attempt_id_in_AUTHENTICATION_flow";
const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_AUTHENTICATION_flow";

#[allow(clippy::too_many_arguments)]
pub fn construct_authentication_router_data(
    authentication_connector: String,
    payment_method_data: payments::PaymentMethodData,
    payment_method: PaymentMethod,
    billing_address: api_models::payments::Address,
    shipping_address: Option<api_models::payments::Address>,
    browser_details: Option<types::BrowserInformation>,
    amount: Option<i64>,
    currency: Option<common_enums::Currency>,
    message_category: types::api::authentication::MessageCategory,
    device_channel: payments::DeviceChannel,
    merchant_account: domain::MerchantAccount,
    merchant_connector_account: payments_helpers::MerchantConnectorAccountType,
    authentication_data: (super::types::AuthenticationData, storage::Authentication),
    return_url: Option<String>,
    sdk_information: Option<api_models::payments::SDKInformation>,
    email: Option<common_utils::pii::Email>,
) -> RouterResult<types::ConnectorAuthenticationRouterData> {
    let router_request = types::ConnectorAuthenticationRequestData {
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
        email,
    };
    construct_router_data(
        authentication_connector,
        payment_method,
        merchant_account.merchant_id.clone(),
        types::PaymentAddress::default(),
        router_request,
        &merchant_connector_account,
    )
}

pub fn construct_post_authentication_router_data(
    authentication_connector: String,
    merchant_account: domain::MerchantAccount,
    merchant_connector_account: payments_helpers::MerchantConnectorAccountType,
    authentication_data: super::types::AuthenticationData,
) -> RouterResult<types::ConnectorPostAuthenticationRouterData> {
    let router_request = types::ConnectorPostAuthenticationRequestData {
        authentication_data,
    };
    construct_router_data(
        authentication_connector,
        PaymentMethod::default(),
        merchant_account.merchant_id.clone(),
        types::PaymentAddress::default(),
        router_request,
        &merchant_connector_account,
    )
}

pub fn construct_pre_authentication_router_data(
    authentication_connector: String,
    card_holder_account_number: cards::CardNumber,
    merchant_connector_account: &payments_helpers::MerchantConnectorAccountType,
    merchant_id: String,
) -> RouterResult<types::authentication::PreAuthNRouterData> {
    let router_request = types::authentication::PreAuthNRequestData {
        card_holder_account_number,
    };
    construct_router_data(
        authentication_connector,
        PaymentMethod::default(),
        merchant_id,
        types::PaymentAddress::default(),
        router_request,
        merchant_connector_account,
    )
}

// pub fn construct_router_data_data ()

pub fn construct_router_data<F: Clone, Req, Res>(
    authentication_connector_name: String,
    payment_method: PaymentMethod,
    merchant_id: String,
    address: types::PaymentAddress,
    request_data: Req,
    merchant_connector_account: &payments_helpers::MerchantConnectorAccountType,
) -> RouterResult<types::RouterData<F, Req, Res>> {
    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(types::RouterData {
        flow: PhantomData,
        merchant_id,
        customer_id: None,
        connector_customer: None,
        connector: authentication_connector_name,
        payment_id: IRRELEVANT_PAYMENT_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        return_url: None,
        address,
        auth_type: common_enums::AuthenticationType::NoThreeDs,
        connector_meta_data: merchant_connector_account.get_metadata(),
        amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request: request_data,
        response: Err(types::ErrorResponse::default()),
        payment_method_id: None,
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        payout_method_data: None,
        quote_id: None,
        test_mode,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
    })
}
