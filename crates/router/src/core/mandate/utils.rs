use std::marker::PhantomData;

use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use diesel_models::Mandate;
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::mandates::PaymentsMandateReferenceRecord;

use crate::{
    core::{errors, payments::helpers},
    types::{self, domain, PaymentAddress},
    SessionState,
};

const IRRELEVANT_ATTEMPT_ID_IN_MANDATE_REVOKE_FLOW: &str =
    "irrelevant_attempt_id_in_mandate_revoke_flow";

const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_MANDATE_REVOKE_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_mandate_revoke_flow";

pub async fn construct_mandate_revoke_router_data(
    state: &SessionState,
    merchant_connector_account: helpers::MerchantConnectorAccountType,
    platform: &domain::Platform,
    mandate: Mandate,
) -> CustomResult<types::MandateRevokeRouterData, errors::ApiErrorResponse> {
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
        customer_id: Some(mandate.customer_id),
        tenant_id: state.tenant.tenant_id.clone(),
        connector_customer: None,
        connector: mandate.connector,
        payment_id: mandate
            .original_payment_id
            .unwrap_or_else(|| {
                common_utils::id_type::PaymentId::get_irrelevant_id("mandate_revoke")
            })
            .get_string_repr()
            .to_owned(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_MANDATE_REVOKE_FLOW.to_string(),
        status: diesel_models::enums::AttemptStatus::default(),
        payment_method: diesel_models::enums::PaymentMethod::default(),
        payment_method_type: None,
        connector_auth_type: auth_type,
        description: None,
        address: PaymentAddress::default(),
        auth_type: diesel_models::enums::AuthenticationType::default(),
        connector_meta_data: None,
        connector_wallets_details: None,
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
        payment_method_status: None,
        request: types::MandateRevokeRequestData {
            connector_mandate_id: mandate.connector_mandate_id,
        },
        response: Err(types::ErrorResponse::get_not_implemented()),
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
        refund_id: None,
        dispute_id: None,
        payout_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        accept_amount_mismatch: None,
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
        customer_document_details: None,
        feature_data: None,
        sender_payment_instrument_id: None,
        connector_returned_payment_method_details: None,
        customer_date_of_birth: None,
    };

    Ok(router_data)
}

#[cfg(feature = "v1")]
pub async fn construct_pm_connector_mandate_revoke_router_data(
    state: &SessionState,
    merchant_connector_account: helpers::MerchantConnectorAccountType,
    platform: &domain::Platform,
    payment_method: &domain::PaymentMethod,
    mandate_reference_record: &PaymentsMandateReferenceRecord,
    connector: String,
) -> CustomResult<types::MandateRevokeRouterData, errors::ApiErrorResponse> {
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
        customer_id: payment_method.customer_id.clone(),
        tenant_id: state.tenant.tenant_id.clone(),
        connector_customer: mandate_reference_record.connector_customer_id.clone(),
        connector,
        payment_id: common_utils::id_type::PaymentId::get_irrelevant_id("connector_mandate_revoke")
            .get_string_repr()
            .to_owned(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_MANDATE_REVOKE_FLOW.to_string(),
        status: diesel_models::enums::AttemptStatus::default(),
        payment_method: payment_method.payment_method.unwrap_or_default(),
        payment_method_type: mandate_reference_record.payment_method_type,
        connector_auth_type: auth_type,
        description: None,
        address: PaymentAddress::default(),
        auth_type: diesel_models::enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: None,
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
        payment_method_status: None,
        request: types::MandateRevokeRequestData {
            connector_mandate_id: Some(mandate_reference_record.connector_mandate_id.clone()),
        },
        response: Err(types::ErrorResponse::get_not_implemented()),
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
        refund_id: None,
        dispute_id: None,
        payout_id: None,
        connector_response: None,
        integrity_check: Ok(()),
        accept_amount_mismatch: None,
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: mandate_reference_record
            .connector_mandate_request_reference_id
            .clone(),
        authentication_id: None,
        psd2_sca_exemption_type: None,
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
        customer_document_details: None,
        feature_data: None,
        sender_payment_instrument_id: None,
        connector_returned_payment_method_details: None,
        customer_date_of_birth: None,
    };

    Ok(router_data)
}
