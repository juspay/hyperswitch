use std::marker::PhantomData;

use common_enums::enums::PaymentMethod;
use common_utils::ext_traits::ValueExt;
use diesel_models::authentication::{Authentication, AuthenticationUpdate};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    payment_address::PaymentAddress,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_data_v2::UasFlowData,
    router_request_types::unified_authentication_service::UasAuthenticationResponseData,
};
use masking::ExposeOptionInterface;

use super::types::{
    IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW,
    IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW,
};
use crate::{
    core::{
        errors::{utils::ConnectorErrorExt, RouterResult},
        payments,
        unified_authentication_service::MerchantConnectorAccountType,
    },
    services::{self, execute_connector_processing_step},
    types::api,
    SessionState,
};

pub async fn update_trackers<F: Clone, Req>(
    state: &SessionState,
    router_data: RouterData<F, Req, UasAuthenticationResponseData>,
    authentication: Authentication,
) -> RouterResult<Authentication> {
    let authentication_update = match router_data.response {
        Ok(response) => match response {
            UasAuthenticationResponseData::PreAuthentication {} => {
                AuthenticationUpdate::AuthenticationStatusUpdate {
                    trans_status: common_enums::TransactionStatus::InformationOnly,
                    authentication_status: common_enums::AuthenticationStatus::Pending,
                }
            }
            UasAuthenticationResponseData::PostAuthentication {
                authentication_details,
            } => AuthenticationUpdate::PostAuthenticationUpdate {
                authentication_status: common_enums::AuthenticationStatus::Success,
                trans_status: common_enums::TransactionStatus::Success,
                authentication_value: authentication_details
                    .dynamic_data_details
                    .and_then(|data| data.dynamic_data_value.expose_option()),
                eci: authentication_details.eci,
            },
        },
        Err(error) => AuthenticationUpdate::ErrorUpdate {
            connector_authentication_id: error.connector_transaction_id,
            authentication_status: common_enums::AuthenticationStatus::Failed,
            error_message: error
                .reason
                .map(|reason| format!("message: {}, reason: {}", error.message, reason))
                .or(Some(error.message)),
            error_code: Some(error.code),
        },
    };

    state
        .store
        .update_authentication_by_merchant_id_authentication_id(
            authentication,
            authentication_update,
        )
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while updating authentication for uas")
}

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

pub fn construct_uas_router_data<F: Clone, Req, Res>(
    state: &SessionState,
    authentication_connector_name: String,
    payment_method: PaymentMethod,
    merchant_id: common_utils::id_type::MerchantId,
    address: Option<PaymentAddress>,
    request_data: Req,
    merchant_connector_account: &MerchantConnectorAccountType,
    authentication_id: Option<String>,
) -> RouterResult<RouterData<F, Req, Res>> {
    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(ApiErrorResponse::InternalServerError)?;
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
        response: Err(ErrorResponse::default()),
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
        authentication_id,
        psd2_sca_exemption_type: None,
    })
}
