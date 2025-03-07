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
        self, domain, storage,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils::ext_traits::OptionExt,
    SessionState,
};

const IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_attempt_id_in_AUTHENTICATION_flow";
const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_AUTHENTICATION_flow";

#[allow(clippy::too_many_arguments)]
pub fn construct_authentication_router_data(
    state: &SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    authentication_connector: String,
    payment_method_data: domain::PaymentMethodData,
    payment_method: PaymentMethod,
    billing_address: hyperswitch_domain_models::address::Address,
    shipping_address: Option<hyperswitch_domain_models::address::Address>,
    browser_details: Option<types::BrowserInformation>,
    amount: Option<common_utils::types::MinorUnit>,
    currency: Option<common_enums::Currency>,
    message_category: types::api::authentication::MessageCategory,
    device_channel: payments::DeviceChannel,
    merchant_connector_account: payments_helpers::MerchantConnectorAccountType,
    authentication_data: storage::Authentication,
    return_url: Option<String>,
    sdk_information: Option<payments::SdkInformation>,
    threeds_method_comp_ind: payments::ThreeDsCompletionIndicator,
    email: Option<common_utils::pii::Email>,
    webhook_url: String,
    three_ds_requestor_url: String,
    psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    payment_id: common_utils::id_type::PaymentId,
    force_3ds_challenge: bool,
) -> RouterResult<types::authentication::ConnectorAuthenticationRouterData> {
    let router_request = types::authentication::ConnectorAuthenticationRequestData {
        payment_method_data,
        billing_address,
        shipping_address,
        browser_details,
        amount: amount.map(|amt| amt.get_amount_as_i64()),
        currency,
        message_category,
        device_channel,
        pre_authentication_data: super::types::PreAuthenticationData::foreign_try_from(
            &authentication_data,
        )?,
        return_url,
        sdk_information,
        email,
        three_ds_requestor_url,
        threeds_method_comp_ind,
        webhook_url,
        force_3ds_challenge,
    };
    construct_router_data(
        state,
        authentication_connector,
        payment_method,
        merchant_id.clone(),
        types::PaymentAddress::default(),
        router_request,
        &merchant_connector_account,
        psd2_sca_exemption_type,
        payment_id,
    )
}

pub fn construct_post_authentication_router_data(
    state: &SessionState,
    authentication_connector: String,
    business_profile: domain::Profile,
    merchant_connector_account: payments_helpers::MerchantConnectorAccountType,
    authentication_data: &storage::Authentication,
    payment_id: &common_utils::id_type::PaymentId,
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
        state,
        authentication_connector,
        PaymentMethod::default(),
        business_profile.merchant_id.clone(),
        types::PaymentAddress::default(),
        router_request,
        &merchant_connector_account,
        None,
        payment_id.clone(),
    )
}

pub fn construct_pre_authentication_router_data<F: Clone>(
    state: &SessionState,
    authentication_connector: String,
    card: hyperswitch_domain_models::payment_method_data::Card,
    merchant_connector_account: &payments_helpers::MerchantConnectorAccountType,
    merchant_id: common_utils::id_type::MerchantId,
    payment_id: common_utils::id_type::PaymentId,
) -> RouterResult<
    types::RouterData<
        F,
        types::authentication::PreAuthNRequestData,
        types::authentication::AuthenticationResponseData,
    >,
> {
    let router_request = types::authentication::PreAuthNRequestData { card };
    construct_router_data(
        state,
        authentication_connector,
        PaymentMethod::default(),
        merchant_id,
        types::PaymentAddress::default(),
        router_request,
        merchant_connector_account,
        None,
        payment_id,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn construct_router_data<F: Clone, Req, Res>(
    state: &SessionState,
    authentication_connector_name: String,
    payment_method: PaymentMethod,
    merchant_id: common_utils::id_type::MerchantId,
    address: types::PaymentAddress,
    request_data: Req,
    merchant_connector_account: &payments_helpers::MerchantConnectorAccountType,
    psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    payment_id: common_utils::id_type::PaymentId,
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
        tenant_id: state.tenant.tenant_id.clone(),
        connector_customer: None,
        connector: authentication_connector_name,
        payment_id: payment_id.get_string_repr().to_owned(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        address,
        auth_type: common_enums::AuthenticationType::NoThreeDs,
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: None,
        minor_amount_captured: None,
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
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type,
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
