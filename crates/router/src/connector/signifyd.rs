pub mod transformers;
use std::fmt::Debug;

#[cfg(feature = "frm")]
use base64::Engine;
#[cfg(feature = "frm")]
use common_utils::{crypto, ext_traits::ByteSliceExt, request::RequestContent};
#[cfg(feature = "frm")]
use error_stack::ResultExt;
#[cfg(feature = "frm")]
use masking::PeekInterface;
#[cfg(feature = "frm")]
use ring::hmac;
#[cfg(feature = "frm")]
use transformers as signifyd;

#[cfg(feature = "frm")]
use super::utils as connector_utils;
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    headers,
    services::{self, request, ConnectorIntegration, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
    },
};
#[cfg(feature = "frm")]
use crate::{
    consts,
    events::connector_api_logs::ConnectorEvent,
    types::{
        api::fraud_check as frm_api, domain, fraud_check as frm_types, ErrorResponse, Response,
    },
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Signifyd;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Signifyd
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
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
    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.signifyd.base_url.as_ref()
    }

    #[cfg(feature = "frm")]
    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = signifyd::SignifydAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_api_key = format!(
            "Basic {}",
            consts::BASE64_ENGINE.encode(auth.api_key.peek())
        );

        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            request::Mask::into_masked(auth_api_key),
        )])
    }

    #[cfg(feature = "frm")]
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: signifyd::SignifydErrorResponse = res
            .response
            .parse_struct("SignifydErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: consts::NO_ERROR_CODE.to_string(),
            message: response.messages.join(" &"),
            reason: Some(response.errors.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl api::Payment for Signifyd {}
impl api::PaymentAuthorize for Signifyd {}
impl api::PaymentSync for Signifyd {}
impl api::PaymentVoid for Signifyd {}
impl api::PaymentCapture for Signifyd {}
impl api::MandateSetup for Signifyd {}
impl api::ConnectorAccessToken for Signifyd {}
impl api::PaymentToken for Signifyd {}
impl api::Refund for Signifyd {}
impl api::RefundExecute for Signifyd {}
impl api::RefundSync for Signifyd {}
impl ConnectorValidation for Signifyd {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Signifyd
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Signifyd
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Signifyd
{
    fn build_request(
        &self,
        _req: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Signifyd".to_string())
                .into(),
        )
    }
}

impl api::PaymentSession for Signifyd {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Signifyd
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Signifyd
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Signifyd
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Signifyd
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Signifyd
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Signifyd
{
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Signifyd {}

#[cfg(feature = "frm")]
impl api::FraudCheck for Signifyd {}
#[cfg(feature = "frm")]
impl frm_api::FraudCheckSale for Signifyd {}
#[cfg(feature = "frm")]
impl frm_api::FraudCheckCheckout for Signifyd {}
#[cfg(feature = "frm")]
impl frm_api::FraudCheckTransaction for Signifyd {}
#[cfg(feature = "frm")]
impl frm_api::FraudCheckFulfillment for Signifyd {}
#[cfg(feature = "frm")]
impl frm_api::FraudCheckRecordReturn for Signifyd {}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        frm_api::Sale,
        frm_types::FraudCheckSaleData,
        frm_types::FraudCheckResponseData,
    > for Signifyd
{
    fn get_headers(
        &self,
        req: &frm_types::FrmSaleRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &frm_types::FrmSaleRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/v3/orders/events/sales"
        ))
    }

    fn get_request_body(
        &self,
        req: &frm_types::FrmSaleRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = signifyd::SignifydPaymentsSaleRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &frm_types::FrmSaleRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&frm_types::FrmSaleType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(frm_types::FrmSaleType::get_headers(self, req, connectors)?)
                .set_body(frm_types::FrmSaleType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &frm_types::FrmSaleRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<frm_types::FrmSaleRouterData, errors::ConnectorError> {
        let response: signifyd::SignifydPaymentsResponse = res
            .response
            .parse_struct("SignifydPaymentsResponse Sale")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        <frm_types::FrmSaleRouterData>::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        frm_api::Checkout,
        frm_types::FraudCheckCheckoutData,
        frm_types::FraudCheckResponseData,
    > for Signifyd
{
    fn get_headers(
        &self,
        req: &frm_types::FrmCheckoutRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &frm_types::FrmCheckoutRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/v3/orders/events/checkouts"
        ))
    }

    fn get_request_body(
        &self,
        req: &frm_types::FrmCheckoutRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = signifyd::SignifydPaymentsCheckoutRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &frm_types::FrmCheckoutRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&frm_types::FrmCheckoutType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(frm_types::FrmCheckoutType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(frm_types::FrmCheckoutType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &frm_types::FrmCheckoutRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<frm_types::FrmCheckoutRouterData, errors::ConnectorError> {
        let response: signifyd::SignifydPaymentsResponse = res
            .response
            .parse_struct("SignifydPaymentsResponse Checkout")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        <frm_types::FrmCheckoutRouterData>::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        frm_api::Transaction,
        frm_types::FraudCheckTransactionData,
        frm_types::FraudCheckResponseData,
    > for Signifyd
{
    fn get_headers(
        &self,
        req: &frm_types::FrmTransactionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &frm_types::FrmTransactionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/v3/orders/events/transactions"
        ))
    }

    fn get_request_body(
        &self,
        req: &frm_types::FrmTransactionRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = signifyd::SignifydPaymentsTransactionRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &frm_types::FrmTransactionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&frm_types::FrmTransactionType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(frm_types::FrmTransactionType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(frm_types::FrmTransactionType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &frm_types::FrmTransactionRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<frm_types::FrmTransactionRouterData, errors::ConnectorError> {
        let response: signifyd::SignifydPaymentsResponse = res
            .response
            .parse_struct("SignifydPaymentsResponse Transaction")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        <frm_types::FrmTransactionRouterData>::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        frm_api::Fulfillment,
        frm_types::FraudCheckFulfillmentData,
        frm_types::FraudCheckResponseData,
    > for Signifyd
{
    fn get_headers(
        &self,
        req: &frm_types::FrmFulfillmentRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &frm_types::FrmFulfillmentRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/v3/orders/events/fulfillments"
        ))
    }

    fn get_request_body(
        &self,
        req: &frm_types::FrmFulfillmentRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = signifyd::FrmFulfillmentSignifydRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj.clone())))
    }

    fn build_request(
        &self,
        req: &frm_types::FrmFulfillmentRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&frm_types::FrmFulfillmentType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(frm_types::FrmFulfillmentType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(frm_types::FrmFulfillmentType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &frm_types::FrmFulfillmentRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<frm_types::FrmFulfillmentRouterData, errors::ConnectorError> {
        let response: signifyd::FrmFulfillmentSignifydApiResponse = res
            .response
            .parse_struct("FrmFulfillmentSignifydApiResponse Sale")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        frm_types::FrmFulfillmentRouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        frm_api::RecordReturn,
        frm_types::FraudCheckRecordReturnData,
        frm_types::FraudCheckResponseData,
    > for Signifyd
{
    fn get_headers(
        &self,
        req: &frm_types::FrmRecordReturnRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &frm_types::FrmRecordReturnRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/v3/orders/events/returns/records"
        ))
    }

    fn get_request_body(
        &self,
        req: &frm_types::FrmRecordReturnRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = signifyd::SignifydPaymentsRecordReturnRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &frm_types::FrmRecordReturnRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&frm_types::FrmRecordReturnType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(frm_types::FrmRecordReturnType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(frm_types::FrmRecordReturnType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &frm_types::FrmRecordReturnRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<frm_types::FrmRecordReturnRouterData, errors::ConnectorError> {
        let response: signifyd::SignifydPaymentsRecordReturnResponse = res
            .response
            .parse_struct("SignifydPaymentsResponse Transaction")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        <frm_types::FrmRecordReturnRouterData>::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
#[async_trait::async_trait]
impl api::IncomingWebhook for Signifyd {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let header_value =
            connector_utils::get_header_key_value("x-signifyd-sec-hmac-sha256", request.headers)?;
        Ok(header_value.as_bytes().to_vec())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(request.body.to_vec())
    }

    async fn verify_webhook_source(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_account: &domain::MerchantAccount,
        merchant_connector_account: domain::MerchantConnectorAccount,
        connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_account,
                connector_label,
                merchant_connector_account,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let message = self
            .get_webhook_source_verification_message(
                request,
                &merchant_account.merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signing_key = hmac::Key::new(hmac::HMAC_SHA256, &connector_webhook_secrets.secret);
        let signed_message = hmac::sign(&signing_key, &message);
        let payload_sign = consts::BASE64_ENGINE.encode(signed_message.as_ref());
        Ok(payload_sign.as_bytes().eq(&signature))
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let resource: signifyd::SignifydWebhookBody = request
            .body
            .parse_struct("SignifydWebhookBody")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::PaymentAttemptId(resource.order_id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let resource: signifyd::SignifydWebhookBody = request
            .body
            .parse_struct("SignifydWebhookBody")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(api::IncomingWebhookEvent::from(resource.review_disposition))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let resource: signifyd::SignifydWebhookBody = request
            .body
            .parse_struct("SignifydWebhookBody")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(resource))
    }
}
