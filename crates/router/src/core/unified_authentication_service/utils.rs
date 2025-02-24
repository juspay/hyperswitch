use std::marker::PhantomData;

use common_enums::enums::PaymentMethod;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    payment_address::PaymentAddress,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_data_v2::UasFlowData,
    router_request_types::unified_authentication_service::UasAuthenticationResponseData,
};

use super::types::{
    IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW,
    IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW,
};
use crate::{
    core::{
        errors::{utils::ConnectorErrorExt, RouterResult},
        payments,
    },
    services::{self, execute_connector_processing_step},
    types::{api, transformers::ForeignFrom},
    SessionState,
};

pub async fn do_auth_connector_call<F, Req, Res>(
    state: &SessionState,
    authentication_connector_name: String,
    router_data: RouterData<F, Req, Res>,
) -> RouterResult<RouterData<F, Req, Res>>
where
    Req: std::fmt::Debug + Clone + 'static,
    Res: std::fmt::Debug + Clone + 'static,
    F: std::fmt::Debug + Clone + 'static,
    dyn api::Connector + Sync: services::api::ConnectorIntegration<F, Req, Res>,
    dyn api::ConnectorV2 + Sync: services::api::ConnectorIntegrationV2<F, UasFlowData, Req, Res>,
{
    let connector_data =
        api::AuthenticationConnectorData::get_connector_by_name(&authentication_connector_name)?;
    let connector_integration: services::BoxedUnifiedAuthenticationServiceInterface<F, Req, Res> =
        connector_data.connector.get_connector_integration();
    let router_data = execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_payment_failed_response()?;
    Ok(router_data)
}

#[allow(clippy::too_many_arguments)]
pub fn construct_uas_router_data<F: Clone, Req, Res>(
    state: &SessionState,
    authentication_connector_name: String,
    payment_method: PaymentMethod,
    merchant_id: common_utils::id_type::MerchantId,
    address: Option<PaymentAddress>,
    request_data: Req,
    merchant_connector_account: &payments::helpers::MerchantConnectorAccountType,
    authentication_id: Option<String>,
) -> RouterResult<RouterData<F, Req, Res>> {
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing ConnectorAuthType")?;
    Ok(RouterData {
        flow: PhantomData,
        merchant_id,
        customer_id: None,
        connector_customer: None,
        connector: authentication_connector_name,
        payment_id: common_utils::id_type::PaymentId::get_irrelevant_id("authentication")
            .get_string_repr()
            .to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        address: address.unwrap_or_default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata().clone(),
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
        response: Err(ErrorResponse::default()),
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
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
        authentication_id,
        psd2_sca_exemption_type: None,
    })
}

pub async fn external_authentication_update_trackers<F: Clone, Req>(
    state: &SessionState,
    router_data: RouterData<F, Req, UasAuthenticationResponseData>,
    authentication: diesel_models::authentication::Authentication,
    acquirer_details: Option<
        hyperswitch_domain_models::router_request_types::authentication::AcquirerDetails,
    >,
) -> RouterResult<diesel_models::authentication::Authentication> {
    let authentication_update = match router_data.response {
        Ok(response) => match response {
            UasAuthenticationResponseData::PreAuthentication {
                authentication_details,
            } => Ok(
                diesel_models::authentication::AuthenticationUpdate::PreAuthenticationUpdate {
                    threeds_server_transaction_id: authentication_details
                        .threeds_server_transaction_id
                        .ok_or(ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "missing threeds_server_transaction_id in PreAuthentication Details",
                        )?,
                    maximum_supported_3ds_version: authentication_details
                        .maximum_supported_3ds_version
                        .ok_or(ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "missing maximum_supported_3ds_version in PreAuthentication Details",
                        )?,
                    connector_authentication_id: authentication_details
                        .connector_authentication_id
                        .ok_or(ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "missing connector_authentication_id in PreAuthentication Details",
                        )?,
                    three_ds_method_data: authentication_details.three_ds_method_data,
                    three_ds_method_url: authentication_details.three_ds_method_url,
                    message_version: authentication_details
                        .message_version
                        .ok_or(ApiErrorResponse::InternalServerError)
                        .attach_printable("missing message_version in PreAuthentication Details")?,
                    connector_metadata: authentication_details.connector_metadata,
                    authentication_status: common_enums::AuthenticationStatus::Pending,
                    acquirer_bin: acquirer_details
                        .as_ref()
                        .map(|acquirer_details| acquirer_details.acquirer_bin.clone()),
                    acquirer_merchant_id: acquirer_details
                        .as_ref()
                        .map(|acquirer_details| acquirer_details.acquirer_merchant_id.clone()),
                    acquirer_country_code: acquirer_details
                        .and_then(|acquirer_details| acquirer_details.acquirer_country_code),
                    directory_server_id: authentication_details.directory_server_id,
                },
            ),
            UasAuthenticationResponseData::Authentication {
                authentication_details,
            } => {
                let authentication_status = common_enums::AuthenticationStatus::foreign_from(
                    authentication_details.trans_status.clone(),
                );
                Ok(
                    diesel_models::authentication::AuthenticationUpdate::AuthenticationUpdate {
                        authentication_value: authentication_details.authentication_value,
                        trans_status: authentication_details.trans_status,
                        acs_url: authentication_details.authn_flow_type.get_acs_url(),
                        challenge_request: authentication_details
                            .authn_flow_type
                            .get_challenge_request(),
                        acs_reference_number: authentication_details
                            .authn_flow_type
                            .get_acs_reference_number(),
                        acs_trans_id: authentication_details.authn_flow_type.get_acs_trans_id(),
                        acs_signed_content: authentication_details
                            .authn_flow_type
                            .get_acs_signed_content(),
                        authentication_type: authentication_details
                            .authn_flow_type
                            .get_decoupled_authentication_type(),
                        authentication_status,
                        connector_metadata: authentication_details.connector_metadata,
                        ds_trans_id: authentication_details.ds_trans_id,
                    },
                )
            }
            UasAuthenticationResponseData::PostAuthentication {
                authentication_details,
            } => {
                let trans_status = authentication_details
                    .trans_status
                    .ok_or(ApiErrorResponse::InternalServerError)
                    .attach_printable("missing trans_status in PostAuthentication Details")?;
                Ok(
                    diesel_models::authentication::AuthenticationUpdate::PostAuthenticationUpdate {
                        authentication_status: common_enums::AuthenticationStatus::foreign_from(
                            trans_status.clone(),
                        ),
                        trans_status,
                        authentication_value: authentication_details
                            .dynamic_data_details
                            .and_then(|details| details.dynamic_data_value)
                            .map(masking::ExposeInterface::expose),
                        eci: authentication_details.eci,
                    },
                )
            }

            UasAuthenticationResponseData::Confirmation { .. } => Err(
                ApiErrorResponse::InternalServerError,
            )
            .attach_printable("unexpected api confirmation in external authentication flow."),
        },
        Err(error) => Ok(
            diesel_models::authentication::AuthenticationUpdate::ErrorUpdate {
                connector_authentication_id: error.connector_transaction_id,
                authentication_status: common_enums::AuthenticationStatus::Failed,
                error_message: error
                    .reason
                    .map(|reason| format!("message: {}, reason: {}", error.message, reason))
                    .or(Some(error.message)),
                error_code: Some(error.code),
            },
        ),
    }?;

    state
        .store
        .update_authentication_by_merchant_id_authentication_id(
            authentication,
            authentication_update,
        )
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while updating authentication")
}

pub fn get_checkout_event_status_and_reason(
    attempt_status: common_enums::AttemptStatus,
) -> (Option<String>, Option<String>) {
    match attempt_status {
        common_enums::AttemptStatus::Charged | common_enums::AttemptStatus::Authorized => (
            Some("02".to_string()),
            Some("Approval Code received".to_string()),
        ),
        _ => (
            Some("03".to_string()),
            Some("No Approval Code received".to_string()),
        ),
    }
}
