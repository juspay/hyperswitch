pub mod transformers;
use std::fmt::Debug;

#[cfg(feature = "frm")]
use api_models::webhooks::IncomingWebhookEvent;
#[cfg(feature = "frm")]
use base64::Engine;
#[cfg(feature = "frm")]
use common_utils::{
    consts,
    request::{Method, RequestBuilder},
};
#[cfg(feature = "frm")]
use common_utils::{crypto, ext_traits::ByteSliceExt, request::RequestContent};
use common_utils::{errors::CustomResult, request::Request};
#[cfg(feature = "frm")]
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
    router_flow_types::{
        AccessTokenAuth, Authorize, Capture, Execute, PSync, PaymentMethodToken, RSync, Session,
        SetupMandate, Void,
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentsResponseData, RefundsResponseData, SupportedPaymentMethods,
    },
};
#[cfg(feature = "frm")]
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse},
    router_flow_types::{Checkout, Fulfillment, RecordReturn, Sale, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
        FraudCheckSaleData, FraudCheckTransactionData,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};
use hyperswitch_interfaces::{
    api::{
        ConnectorAccessToken, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration,
        ConnectorSpecifications, ConnectorValidation, MandateSetup, Payment, PaymentAuthorize,
        PaymentCapture, PaymentSession, PaymentSync, PaymentToken, PaymentVoid, Refund,
        RefundExecute, RefundSync,
    },
    configs::Connectors,
    errors::ConnectorError,
};
#[cfg(feature = "frm")]
use hyperswitch_interfaces::{
    api::{
        FraudCheck, FraudCheckCheckout, FraudCheckFulfillment, FraudCheckRecordReturn,
        FraudCheckSale, FraudCheckTransaction,
    },
    consts::NO_ERROR_CODE,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
#[cfg(feature = "frm")]
use masking::Mask;
use masking::Maskable;
#[cfg(feature = "frm")]
use masking::{PeekInterface, Secret};
#[cfg(feature = "frm")]
use ring::hmac;
#[cfg(feature = "frm")]
use transformers as signifyd;

use crate::constants::headers;
#[cfg(feature = "frm")]
use crate::{
    types::{
        FrmCheckoutRouterData, FrmCheckoutType, FrmFulfillmentRouterData, FrmFulfillmentType,
        FrmRecordReturnRouterData, FrmRecordReturnType, FrmSaleRouterData, FrmSaleType,
        FrmTransactionRouterData, FrmTransactionType, ResponseRouterData,
    },
    utils::get_header_key_value,
};

#[derive(Debug, Clone)]
pub struct Signifyd;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Signifyd
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];

        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Signifyd {
    fn id(&self) -> &'static str {
        "signifyd"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }
    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.signifyd.base_url.as_ref()
    }

    #[cfg(feature = "frm")]
    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth = signifyd::SignifydAuthType::try_from(auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        let auth_api_key = format!(
            "Basic {}",
            consts::BASE64_ENGINE.encode(auth.api_key.peek())
        );

        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            Mask::into_masked(auth_api_key),
        )])
    }

    #[cfg(feature = "frm")]
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: signifyd::SignifydErrorResponse = res
            .response
            .parse_struct("SignifydErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: NO_ERROR_CODE.to_string(),
            message: response.messages.join(" &"),
            reason: Some(response.errors.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl Payment for Signifyd {}
impl PaymentAuthorize for Signifyd {}
impl PaymentSync for Signifyd {}
impl PaymentVoid for Signifyd {}
impl PaymentCapture for Signifyd {}
impl MandateSetup for Signifyd {}
impl ConnectorAccessToken for Signifyd {}
impl PaymentToken for Signifyd {}
impl Refund for Signifyd {}
impl RefundExecute for Signifyd {}
impl RefundSync for Signifyd {}
impl ConnectorValidation for Signifyd {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Signifyd
{
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Signifyd {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Signifyd
{
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Err(ConnectorError::NotImplemented("Setup Mandate flow for Signifyd".to_string()).into())
    }
}

impl PaymentSession for Signifyd {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Signifyd {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Signifyd {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Signifyd {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Signifyd {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Signifyd {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Signifyd {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Signifyd {}

#[cfg(feature = "frm")]
impl FraudCheck for Signifyd {}
#[cfg(feature = "frm")]
impl FraudCheckSale for Signifyd {}
#[cfg(feature = "frm")]
impl FraudCheckCheckout for Signifyd {}
#[cfg(feature = "frm")]
impl FraudCheckTransaction for Signifyd {}
#[cfg(feature = "frm")]
impl FraudCheckFulfillment for Signifyd {}
#[cfg(feature = "frm")]
impl FraudCheckRecordReturn for Signifyd {}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Sale, FraudCheckSaleData, FraudCheckResponseData> for Signifyd {
    fn get_headers(
        &self,
        req: &FrmSaleRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &FrmSaleRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "v3/orders/events/sales"
        ))
    }

    fn get_request_body(
        &self,
        req: &FrmSaleRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let req_obj = signifyd::SignifydPaymentsSaleRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &FrmSaleRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&FrmSaleType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(FrmSaleType::get_headers(self, req, connectors)?)
                .set_body(FrmSaleType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &FrmSaleRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FrmSaleRouterData, ConnectorError> {
        let response: signifyd::SignifydPaymentsResponse = res
            .response
            .parse_struct("SignifydPaymentsResponse Sale")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        <FrmSaleRouterData>::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData> for Signifyd {
    fn get_headers(
        &self,
        req: &FrmCheckoutRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &FrmCheckoutRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "v3/orders/events/checkouts"
        ))
    }

    fn get_request_body(
        &self,
        req: &FrmCheckoutRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let req_obj = signifyd::SignifydPaymentsCheckoutRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &FrmCheckoutRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&FrmCheckoutType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(FrmCheckoutType::get_headers(self, req, connectors)?)
                .set_body(FrmCheckoutType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &FrmCheckoutRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FrmCheckoutRouterData, ConnectorError> {
        let response: signifyd::SignifydPaymentsResponse = res
            .response
            .parse_struct("SignifydPaymentsResponse Checkout")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        <FrmCheckoutRouterData>::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData>
    for Signifyd
{
    fn get_headers(
        &self,
        req: &FrmTransactionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &FrmTransactionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "v3/orders/events/transactions"
        ))
    }

    fn get_request_body(
        &self,
        req: &FrmTransactionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let req_obj = signifyd::SignifydPaymentsTransactionRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &FrmTransactionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&FrmTransactionType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(FrmTransactionType::get_headers(self, req, connectors)?)
                .set_body(FrmTransactionType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &FrmTransactionRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FrmTransactionRouterData, ConnectorError> {
        let response: signifyd::SignifydPaymentsResponse = res
            .response
            .parse_struct("SignifydPaymentsResponse Transaction")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        <FrmTransactionRouterData>::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
    for Signifyd
{
    fn get_headers(
        &self,
        req: &FrmFulfillmentRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &FrmFulfillmentRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "v3/orders/events/fulfillments"
        ))
    }

    fn get_request_body(
        &self,
        req: &FrmFulfillmentRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let req_obj = signifyd::FrmFulfillmentSignifydRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj.clone())))
    }

    fn build_request(
        &self,
        req: &FrmFulfillmentRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&FrmFulfillmentType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(FrmFulfillmentType::get_headers(self, req, connectors)?)
                .set_body(FrmFulfillmentType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &FrmFulfillmentRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FrmFulfillmentRouterData, ConnectorError> {
        let response: signifyd::FrmFulfillmentSignifydApiResponse = res
            .response
            .parse_struct("FrmFulfillmentSignifydApiResponse Sale")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        FrmFulfillmentRouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl ConnectorIntegration<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
    for Signifyd
{
    fn get_headers(
        &self,
        req: &FrmRecordReturnRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &FrmRecordReturnRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "v3/orders/events/returns/records"
        ))
    }

    fn get_request_body(
        &self,
        req: &FrmRecordReturnRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let req_obj = signifyd::SignifydPaymentsRecordReturnRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &FrmRecordReturnRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&FrmRecordReturnType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(FrmRecordReturnType::get_headers(self, req, connectors)?)
                .set_body(FrmRecordReturnType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &FrmRecordReturnRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FrmRecordReturnRouterData, ConnectorError> {
        let response: signifyd::SignifydPaymentsRecordReturnResponse = res
            .response
            .parse_struct("SignifydPaymentsResponse Transaction")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        <FrmRecordReturnRouterData>::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
#[async_trait::async_trait]
impl IncomingWebhook for Signifyd {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        let header_value = get_header_key_value("x-signifyd-sec-hmac-sha256", request.headers)?;
        Ok(header_value.as_bytes().to_vec())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        Ok(request.body.to_vec())
    }

    async fn verify_webhook_source(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
        connector_label: &str,
    ) -> CustomResult<bool, ConnectorError> {
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_label,
                connector_webhook_details,
            )
            .await
            .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

        let message = self
            .get_webhook_source_verification_message(
                request,
                merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

        let signing_key = hmac::Key::new(hmac::HMAC_SHA256, &connector_webhook_secrets.secret);
        let signed_message = hmac::sign(&signing_key, &message);
        let payload_sign = consts::BASE64_ENGINE.encode(signed_message.as_ref());
        Ok(payload_sign.as_bytes().eq(&signature))
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, ConnectorError> {
        let resource: signifyd::SignifydWebhookBody = request
            .body
            .parse_struct("SignifydWebhookBody")
            .change_context(ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::PaymentAttemptId(resource.order_id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        let resource: signifyd::SignifydWebhookBody = request
            .body
            .parse_struct("SignifydWebhookBody")
            .change_context(ConnectorError::WebhookEventTypeNotFound)?;
        Ok(IncomingWebhookEvent::from(resource.review_disposition))
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        let resource: signifyd::SignifydWebhookBody = request
            .body
            .parse_struct("SignifydWebhookBody")
            .change_context(ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(resource))
    }
}

static SYGNIFYD_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Signifyd",
    description: "Signifyd fraud and risk management provider with AI-driven commerce protection platform for maximizing conversions and eliminating fraud risk with guaranteed fraud liability coverage",
    connector_type: common_enums::HyperswitchConnectorCategory::FraudAndRiskManagementProvider,
    integration_status: common_enums::ConnectorIntegrationStatus::Sandbox,
};

impl ConnectorSpecifications for Signifyd {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&SYGNIFYD_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::enums::EventClass]> {
        None
    }
}
