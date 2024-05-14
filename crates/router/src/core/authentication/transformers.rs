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
    types::{
        self, storage,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils::ext_traits::OptionExt,
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
    business_profile: storage::BusinessProfile,
    merchant_connector_account: payments_helpers::MerchantConnectorAccountType,
    authentication_data: storage::Authentication,
    return_url: Option<String>,
    sdk_information: Option<api_models::payments::SdkInformation>,
    threeds_method_comp_ind: api_models::payments::ThreeDsCompletionIndicator,
    email: Option<common_utils::pii::Email>,
    webhook_url: String,
) -> RouterResult<types::authentication::ConnectorAuthenticationRouterData> {
    let authentication_details: api_models::admin::AuthenticationConnectorDetails =
        business_profile
            .authentication_connector_details
            .clone()
            .get_required_value("authentication_details")
            .attach_printable("authentication_details not configured by the merchant")?
            .parse_value("AuthenticationDetails")
            .change_context(errors::ApiErrorResponse::UnprocessableEntity {
                message: "Invalid data format found for authentication_details".into(),
            })
            .attach_printable("Error while parsing authentication_details from merchant_account")?;
    let router_request = types::authentication::ConnectorAuthenticationRequestData {
        payment_method_data: From::from(payment_method_data),
        billing_address,
        shipping_address,
        browser_details,
        amount,
        currency,
        message_category,
        device_channel,
        pre_authentication_data: super::types::PreAuthenticationData::foreign_try_from(
            &authentication_data,
        )?,
        return_url,
        sdk_information,
        email,
        three_ds_requestor_url: authentication_details.three_ds_requestor_url,
        threeds_method_comp_ind,
        webhook_url,
    };
    construct_router_data(
        authentication_connector,
        payment_method,
        business_profile.merchant_id.clone(),
        types::PaymentAddress::default(),
        router_request,
        &merchant_connector_account,
    )
}

pub fn construct_post_authentication_router_data(
    authentication_connector: String,
    business_profile: storage::BusinessProfile,
    merchant_connector_account: payments_helpers::MerchantConnectorAccountType,
    authentication_data: &storage::Authentication,
) -> RouterResult<types::authentication::ConnectorPostAuthenticationRouterData> {
    let threeds_server_transaction_id = authentication_data
        .threeds_server_transaction_id
        .clone()
        .get_required_value("threeds_server_transaction_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let router_request = types::authentication::ConnectorPostAuthenticationRequestData {
        threeds_server_transaction_id,
    };
    construct_router_data(
        authentication_connector,
        PaymentMethod::default(),
        business_profile.merchant_id.clone(),
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
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        dispute_id: None,
        refund_id: None,
        payment_method_status: None,
        connector_response: None,
    })
}

impl ForeignFrom<common_enums::TransactionStatus> for common_enums::AuthenticationStatus {
    fn foreign_from(trans_status: common_enums::TransactionStatus) -> Self {
        match trans_status {
            common_enums::TransactionStatus::Success => Self::Success,
            common_enums::TransactionStatus::Failure
            | common_enums::TransactionStatus::Rejected
            | common_enums::TransactionStatus::VerificationNotPerformed
            | common_enums::TransactionStatus::NotVerified => Self::Failed,
            common_enums::TransactionStatus::ChallengeRequired
            | common_enums::TransactionStatus::ChallengeRequiredDecoupledAuthentication
            | common_enums::TransactionStatus::InformationOnly => Self::Pending,
        }
    }
}
