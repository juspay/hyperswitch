use std::marker::PhantomData;

use common_utils::types::MinorUnit;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::types::{PayoutsData, PayoutsResponseData};
use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
    router_flow_types::{
        Accept, AccessTokenAuth, Authorize, Capture, Defend, Evidence, PSync, PreProcessing,
        Session, SetupMandate, Upload, Void,
    },
    router_request_types::{
        AcceptDisputeRequestData, AccessTokenRequestData, DefendDisputeRequestData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsPreProcessingData,
        PaymentsSessionData, PaymentsSyncData, RefundsData, SetupMandateRequestData,
        SubmitEvidenceRequestData, UploadFileRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, DefendDisputeResponse, PaymentsResponseData, RefundsResponseData,
        SubmitEvidenceResponse, UploadFileResponse,
    },
};

pub(crate) type PaymentsSyncResponseRouterData<R> =
    ResponseRouterData<PSync, R, PaymentsSyncData, PaymentsResponseData>;
pub(crate) type PaymentsResponseRouterData<R> =
    ResponseRouterData<Authorize, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub(crate) type PaymentsCaptureResponseRouterData<R> =
    ResponseRouterData<Capture, R, PaymentsCaptureData, PaymentsResponseData>;
pub(crate) type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;
pub(crate) type RefreshTokenRouterData =
    RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>;

pub(crate) type PaymentsCancelResponseRouterData<R> =
    ResponseRouterData<Void, R, PaymentsCancelData, PaymentsResponseData>;
pub(crate) type PaymentsPreprocessingResponseRouterData<R> =
    ResponseRouterData<PreProcessing, R, PaymentsPreProcessingData, PaymentsResponseData>;
pub(crate) type PaymentsSessionResponseRouterData<R> =
    ResponseRouterData<Session, R, PaymentsSessionData, PaymentsResponseData>;

pub(crate) type AcceptDisputeRouterData =
    RouterData<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>;
pub(crate) type SubmitEvidenceRouterData =
    RouterData<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>;
pub(crate) type UploadFileRouterData =
    RouterData<Upload, UploadFileRequestData, UploadFileResponse>;
pub(crate) type DefendDisputeRouterData =
    RouterData<Defend, DefendDisputeRequestData, DefendDisputeResponse>;

pub(crate) type SetupMandateRouterData =
    RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;

#[cfg(feature = "payouts")]
pub type PayoutsResponseRouterData<F, R> =
    ResponseRouterData<F, R, PayoutsData, PayoutsResponseData>;

// TODO: Remove `ResponseRouterData` from router crate after all the related type aliases are moved to this crate.
pub struct ResponseRouterData<Flow, R, Request, Response> {
    pub response: R,
    pub data: RouterData<Flow, Request, Response>,
    pub http_code: u16,
}

pub fn convert_setup_mandate_router_data_to_authorize_router_data(
    data: &SetupMandateRouterData,
) -> PaymentsAuthorizeData {
    PaymentsAuthorizeData {
        currency: data.request.currency,
        payment_method_data: data.request.payment_method_data.clone(),
        confirm: data.request.confirm,
        statement_descriptor_suffix: data.request.statement_descriptor_suffix.clone(),
        mandate_id: data.request.mandate_id.clone(),
        setup_future_usage: data.request.setup_future_usage,
        off_session: data.request.off_session,
        setup_mandate_details: data.request.setup_mandate_details.clone(),
        router_return_url: data.request.router_return_url.clone(),
        email: data.request.email.clone(),
        customer_name: data.request.customer_name.clone(),
        amount: 0,
        order_tax_amount: Some(MinorUnit::zero()),
        minor_amount: MinorUnit::new(0),
        statement_descriptor: None,
        capture_method: None,
        webhook_url: None,
        complete_authorize_url: None,
        browser_info: data.request.browser_info.clone(),
        order_details: None,
        order_category: None,
        session_token: None,
        enrolled_for_3ds: true,
        related_transaction_id: None,
        payment_experience: None,
        payment_method_type: None,
        customer_id: None,
        surcharge_details: None,
        request_incremental_authorization: data.request.request_incremental_authorization,
        metadata: None,
        authentication_data: None,
        customer_acceptance: data.request.customer_acceptance.clone(),
        split_payments: None, // TODO: allow charges on mandates?
        merchant_order_reference_id: None,
        integrity_object: None,
        additional_payment_method_data: None,
        shipping_cost: data.request.shipping_cost,
        merchant_account_id: None,
        merchant_config_currency: None,
    }
}

pub fn convert_payment_authorize_router_response<F1, F2, T1, T2>(
    item: (&RouterData<F1, T1, PaymentsResponseData>, T2),
) -> RouterData<F2, T2, PaymentsResponseData> {
    let data = item.0;
    let request = item.1;
    RouterData {
        flow: PhantomData,
        request,
        merchant_id: data.merchant_id.clone(),
        connector: data.connector.clone(),
        attempt_id: data.attempt_id.clone(),
        tenant_id: data.tenant_id.clone(),
        status: data.status,
        payment_method: data.payment_method,
        connector_auth_type: data.connector_auth_type.clone(),
        description: data.description.clone(),
        address: data.address.clone(),
        auth_type: data.auth_type,
        connector_meta_data: data.connector_meta_data.clone(),
        connector_wallets_details: data.connector_wallets_details.clone(),
        amount_captured: data.amount_captured,
        minor_amount_captured: data.minor_amount_captured,
        access_token: data.access_token.clone(),
        response: data.response.clone(),
        payment_id: data.payment_id.clone(),
        session_token: data.session_token.clone(),
        reference_id: data.reference_id.clone(),
        customer_id: data.customer_id.clone(),
        payment_method_token: None,
        preprocessing_id: None,
        connector_customer: data.connector_customer.clone(),
        recurring_mandate_payment_data: data.recurring_mandate_payment_data.clone(),
        connector_request_reference_id: data.connector_request_reference_id.clone(),
        #[cfg(feature = "payouts")]
        payout_method_data: data.payout_method_data.clone(),
        #[cfg(feature = "payouts")]
        quote_id: data.quote_id.clone(),
        test_mode: data.test_mode,
        payment_method_status: None,
        payment_method_balance: data.payment_method_balance.clone(),
        connector_api_version: data.connector_api_version.clone(),
        connector_http_status_code: data.connector_http_status_code,
        external_latency: data.external_latency,
        apple_pay_flow: data.apple_pay_flow.clone(),
        frm_metadata: data.frm_metadata.clone(),
        dispute_id: data.dispute_id.clone(),
        refund_id: data.refund_id.clone(),
        connector_response: data.connector_response.clone(),
        integrity_check: Ok(()),
        additional_merchant_data: data.additional_merchant_data.clone(),
        header_payload: data.header_payload.clone(),
        connector_mandate_request_reference_id: data.connector_mandate_request_reference_id.clone(),
        authentication_id: data.authentication_id.clone(),
        psd2_sca_exemption_type: data.psd2_sca_exemption_type,
    }
}
