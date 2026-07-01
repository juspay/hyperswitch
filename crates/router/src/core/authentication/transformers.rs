use std::marker::PhantomData;

use api_models::payments;
use common_enums::PaymentMethod;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use hyperswitch_domain_models::authentication;
use hyperswitch_masking::ExposeInterface;

use crate::{
    core::{
        errors::{self, RouterResult},
        payments::helpers as payments_helpers,
    },
    types::{
        self, domain,
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
    authentication_data: authentication::Authentication,
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
    authentication_data: &authentication::Authentication,
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
        status: common_enums::AttemptStatus::default().into(),
        payment_method,
        payment_method_type: None,
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
        payout_id: None,
        payment_method_status: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type,
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
        customer_document_details: None,
        feature_data: None,
        sender_payment_instrument_id: None,
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

pub fn construct_authentication_domain_model<T>(input: T) -> authentication::Authentication
where
    authentication::Authentication: ForeignFrom<T>,
{
    authentication::Authentication::foreign_from(input)
}

#[cfg(feature = "v1")]
impl
    ForeignFrom<(
        api_models::authentication::AuthenticationResponse,
        api_models::authentication::AuthenticationEligibilityResponse,
        Option<api_models::authentication::ThreeDsData>,
        common_utils::id_type::ProfileId,
        common_utils::id_type::OrganizationId,
    )> for authentication::Authentication
{
    fn foreign_from(
        (auth_create_response, elig_response, three_ds_data, profile_id, organization_id): (
            api_models::authentication::AuthenticationResponse,
            api_models::authentication::AuthenticationEligibilityResponse,
            Option<api_models::authentication::ThreeDsData>,
            common_utils::id_type::ProfileId,
            common_utils::id_type::OrganizationId,
        ),
    ) -> Self {
        Self {
            authentication_id: auth_create_response.authentication_id,
            merchant_id: auth_create_response.merchant_id,
            authentication_connector: auth_create_response
                .authentication_connector
                .map(|c| c.to_string()),
            connector_authentication_id: three_ds_data
                .as_ref()
                .and_then(|d| d.connector_authentication_id.clone()),
            authentication_data: None,
            payment_method_id: "".to_string(),
            authentication_type: None,
            authentication_status: auth_create_response.status,
            authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus::Unused,
            created_at: auth_create_response
                .created_at
                .unwrap_or_else(common_utils::date_time::now),
            modified_at: auth_create_response
                .created_at
                .unwrap_or_else(common_utils::date_time::now),
            error_message: auth_create_response.error_message,
            error_code: auth_create_response.error_code,
            connector_metadata: elig_response.connector_metadata,
            maximum_supported_version: three_ds_data
                .as_ref()
                .and_then(|d| d.maximum_supported_3ds_version.clone()),
            threeds_server_transaction_id: three_ds_data
                .as_ref()
                .and_then(|d| d.three_ds_server_transaction_id.clone()),
            cavv: None,
            authentication_flow_type: None,
            message_version: three_ds_data
                .as_ref()
                .and_then(|d| d.message_version.clone()),
            eci: None,
            trans_status: None,
            acquirer_bin: auth_create_response
                .acquirer_details
                .as_ref()
                .and_then(|a| a.acquirer_bin.clone()),
            acquirer_merchant_id: auth_create_response
                .acquirer_details
                .as_ref()
                .and_then(|a| a.acquirer_merchant_id.clone()),
            three_ds_method_data: three_ds_data
                .as_ref()
                .and_then(|d| d.three_ds_method_data.clone()),
            three_ds_method_url: three_ds_data
                .as_ref()
                .and_then(|d| d.three_ds_method_url.clone().map(|u| u.to_string())),
            acs_url: None,
            challenge_request: None,
            acs_reference_number: None,
            acs_trans_id: None,
            acs_signed_content: None,
            profile_id,
            payment_id: None,
            merchant_connector_id: None,
            ds_trans_id: None,
            directory_server_id: three_ds_data
                .as_ref()
                .and_then(|d| d.directory_server_id.clone()),
            acquirer_country_code: auth_create_response
                .acquirer_details
                .as_ref()
                .and_then(|a| a.merchant_country_code.clone()),
            organization_id,
            mcc: None,
            currency: Some(auth_create_response.currency),
            billing_country: None,
            shipping_country: None,
            issuer_country: None,
            earliest_supported_version: None,
            latest_supported_version: None,
            platform: None,
            device_type: None,
            device_brand: None,
            device_os: None,
            device_display: None,
            browser_name: None,
            browser_version: None,
            issuer_id: None,
            scheme_name: None,
            exemption_requested: None,
            exemption_accepted: None,
            service_details: None,
            authentication_client_secret: auth_create_response.client_secret.map(|s| s.expose()),
            force_3ds_challenge: auth_create_response.force_3ds_challenge,
            psd2_sca_exemption_type: auth_create_response.psd2_sca_exemption_type,
            return_url: auth_create_response.return_url,
            billing_address: None,
            shipping_address: None,
            browser_info: None,
            email: None,
            profile_acquirer_id: auth_create_response.profile_acquirer_id,
            challenge_code: None,
            challenge_cancel: None,
            challenge_code_reason: None,
            message_extension: None,
            challenge_request_key: None,
            customer_details: None,
            amount: Some(auth_create_response.amount),
            merchant_country_code: None,
            processor_merchant_id: None,
            created_by: None,
            updated_by: None,
        }
    }
}

#[cfg(feature = "v1")]
impl
    ForeignFrom<(
        api_models::authentication::AuthenticationSyncResponse,
        common_utils::id_type::OrganizationId,
    )> for authentication::Authentication
{
    fn foreign_from(
        (sync_response, organization_id): (
            api_models::authentication::AuthenticationSyncResponse,
            common_utils::id_type::OrganizationId,
        ),
    ) -> Self {
        Self {
            authentication_id: sync_response.authentication_id,
            merchant_id: sync_response.merchant_id,
            authentication_connector: sync_response
                .authentication_connector
                .map(|c| c.to_string()),
            connector_authentication_id: sync_response.connector_authentication_id,
            authentication_data: None,
            payment_method_id: "".to_string(),
            authentication_type: None,
            authentication_status: sync_response.status,
            authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus::Unused,
            created_at: sync_response.created_at,
            modified_at: sync_response.created_at,
            error_message: None,
            error_code: None,
            connector_metadata: sync_response.connector_metadata,
            maximum_supported_version: sync_response.maximum_supported_3ds_version,
            threeds_server_transaction_id: sync_response.threeds_server_transaction_id,
            cavv: sync_response.authentication_details.as_ref().and_then(|d| {
                match &d.three_ds_data {
                    Some(three_ds_data) => match &three_ds_data.authentication_cryptogram {
                        Some(api_models::authentication::Cryptogram::Cavv {
                            authentication_cryptogram,
                        }) => Some(authentication_cryptogram.clone().expose()),
                        _ => None,
                    },
                    None => None,
                }
            }),
            authentication_flow_type: None,
            message_version: sync_response.message_version,
            eci: sync_response
                .authentication_details
                .as_ref()
                .and_then(|d| d.three_ds_data.as_ref().and_then(|t| t.eci.clone())),
            trans_status: sync_response.authentication_details.as_ref().and_then(|d| {
                d.three_ds_data
                    .as_ref()
                    .map(|t| t.transaction_status.clone())
            }),
            acquirer_bin: sync_response
                .acquirer_details
                .as_ref()
                .and_then(|a| a.acquirer_bin.clone()),
            acquirer_merchant_id: sync_response
                .acquirer_details
                .as_ref()
                .and_then(|a| a.acquirer_merchant_id.clone()),
            three_ds_method_data: sync_response.three_ds_method_data,
            three_ds_method_url: sync_response.three_ds_method_url,
            acs_url: None,
            challenge_request: None,
            acs_reference_number: None,
            acs_trans_id: None,
            acs_signed_content: None,
            profile_id: sync_response.profile_id,
            payment_id: None,
            merchant_connector_id: None,
            ds_trans_id: sync_response
                .authentication_details
                .as_ref()
                .and_then(|d| d.three_ds_data.as_ref().and_then(|t| t.ds_trans_id.clone())),
            directory_server_id: sync_response.directory_server_id,
            acquirer_country_code: sync_response
                .acquirer_details
                .as_ref()
                .and_then(|a| a.merchant_country_code.clone()),
            organization_id,
            mcc: None,
            currency: Some(sync_response.currency),
            billing_country: None,
            shipping_country: None,
            issuer_country: None,
            earliest_supported_version: None,
            latest_supported_version: None,
            platform: None,
            device_type: None,
            device_brand: None,
            device_os: None,
            device_display: None,
            browser_name: None,
            browser_version: None,
            issuer_id: None,
            scheme_name: None,
            exemption_requested: None,
            exemption_accepted: None,
            service_details: None,
            authentication_client_secret: sync_response.client_secret.map(|s| s.expose()),
            force_3ds_challenge: sync_response.force_3ds_challenge,
            psd2_sca_exemption_type: sync_response.psd2_sca_exemption_type,
            return_url: sync_response.return_url,
            billing_address: None,
            shipping_address: None,
            browser_info: None,
            email: None,
            profile_acquirer_id: None,
            challenge_code: None,
            challenge_cancel: None,
            challenge_code_reason: None,
            message_extension: None,
            challenge_request_key: None,
            customer_details: None,
            amount: Some(sync_response.amount),
            merchant_country_code: None,
            processor_merchant_id: None,
            created_by: None,
            updated_by: None,
        }
    }
}
