pub mod transformers;

use std::collections::HashMap;

use common_utils::{
    errors::CustomResult,
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
};
use error_stack::{report, ResultExt};
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use hyperswitch_domain_models::revenue_recovery;
use hyperswitch_domain_models::{
    router_data::AccessToken,
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors, webhooks,
};

// use transformers as stripebilling;

#[derive(Clone)]
pub struct DummyBillingConnector;

impl api::Payment for DummyBillingConnector {}
impl api::PaymentSession for DummyBillingConnector {}
impl api::ConnectorAccessToken for DummyBillingConnector {}
impl api::MandateSetup for DummyBillingConnector {}
impl api::PaymentAuthorize for DummyBillingConnector {}
impl api::PaymentSync for DummyBillingConnector {}
impl api::PaymentCapture for DummyBillingConnector {}
impl api::PaymentVoid for DummyBillingConnector {}
impl api::Refund for DummyBillingConnector {}
impl api::RefundExecute for DummyBillingConnector {}
impl api::RefundSync for DummyBillingConnector {}
impl api::PaymentToken for DummyBillingConnector {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for DummyBillingConnector
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for DummyBillingConnector where
    Self: ConnectorIntegration<Flow, Request, Response>
{
}

impl ConnectorCommon for DummyBillingConnector {
    fn id(&self) -> &'static str {
        "stripebillingtest"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.dummyconnector.base_url.as_ref()
    }
}

impl ConnectorValidation for DummyBillingConnector {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>
    for DummyBillingConnector
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for DummyBillingConnector
{
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for DummyBillingConnector
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for DummyBillingConnector
{
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for DummyBillingConnector {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>
    for DummyBillingConnector
{
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData>
    for DummyBillingConnector
{
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for DummyBillingConnector {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for DummyBillingConnector {}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for DummyBillingConnector {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn common_utils::crypto::VerifySignature + Send>, errors::ConnectorError>
    {
        Ok(Box::new(common_utils::crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let mut header_hashmap = get_signature_elements_from_header(request.headers)?;
        let signature = header_hashmap
            .remove("v1")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        hex::decode(signature).change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let mut header_hashmap = get_signature_elements_from_header(request.headers)?;
        let timestamp = header_hashmap
            .remove("t")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        Ok(format!(
            "{}.{}",
            String::from_utf8_lossy(&timestamp),
            String::from_utf8_lossy(request.body)
        )
        .into_bytes())
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        //  For Stripe billing, we need an additional call to fetch the required recovery data. So, instead of the Invoice ID, we send the Charge ID.
        let webhook =
            transformers::DummyBillingWebhookBody::get_webhook_object_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(webhook.data.object.charge),
        ))
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook =
            transformers::DummyBillingWebhookBody::get_webhook_object_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        let event = match webhook.event_type {
            transformers::DummyBillingEventType::PaymentSucceeded => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentSuccess
            }
            transformers::DummyBillingEventType::PaymentFailed => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentFailure
            }
            transformers::DummyBillingEventType::InvoiceDeleted => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryInvoiceCancel
            }
        };
        Ok(event)
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook =
            transformers::DummyBillingInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(webhook))
    }
}

fn get_signature_elements_from_header(
    headers: &actix_web::http::header::HeaderMap,
) -> CustomResult<HashMap<String, Vec<u8>>, errors::ConnectorError> {
    let security_header = headers
        .get("stripe-signature")
        .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
    let security_header_str = security_header
        .to_str()
        .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
    let header_parts = security_header_str.split(',').collect::<Vec<&str>>();
    let mut header_hashmap: HashMap<String, Vec<u8>> = HashMap::with_capacity(header_parts.len());

    for header_part in header_parts {
        let (header_key, header_value) = header_part
            .split_once('=')
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        header_hashmap.insert(header_key.to_string(), header_value.bytes().collect());
    }

    Ok(header_hashmap)
}

impl ConnectorSpecifications for DummyBillingConnector {}
