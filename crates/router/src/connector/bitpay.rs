pub mod transformers;

use std::fmt::Debug;

use common_utils::{errors::ReportSwitchExt, ext_traits::ByteSliceExt};
use error_stack::ResultExt;
use masking::PeekInterface;
use transformers as bitpay;

use self::bitpay::BitpayWebhookDetails;
use crate::{
    configs::settings,
    consts,
    core::errors::{self, CustomResult},
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt, Encode},
};

#[derive(Debug, Clone)]
pub struct Bitpay;

impl api::Payment for Bitpay {}
impl api::PaymentToken for Bitpay {}
impl api::PaymentSession for Bitpay {}
impl api::ConnectorAccessToken for Bitpay {}
impl api::MandateSetup for Bitpay {}
impl api::PaymentAuthorize for Bitpay {}
impl api::PaymentSync for Bitpay {}
impl api::PaymentCapture for Bitpay {}
impl api::PaymentVoid for Bitpay {}
impl api::Refund for Bitpay {}
impl api::RefundExecute for Bitpay {}
impl api::RefundSync for Bitpay {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Bitpay
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Bitpay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (
                headers::X_ACCEPT_VERSION.to_string(),
                "2.0.0".to_string().into(),
            ),
        ];
        Ok(header)
    }
}

impl ConnectorCommon for Bitpay {
    fn id(&self) -> &'static str {
        "bitpay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.bitpay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = bitpay::BitpayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bitpay::BitpayErrorResponse =
            res.response.parse_struct("BitpayErrorResponse").switch()?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response.error,
            reason: response.message,
        })
    }
}

impl ConnectorValidation for Bitpay {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Bitpay
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Bitpay
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Bitpay
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Bitpay
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/invoices", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = bitpay::BitpayRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let req_obj = bitpay::BitpayPaymentsRequest::try_from(&connector_router_data)?;

        let bitpay_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<bitpay::BitpayPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(bitpay_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: bitpay::BitpayPaymentsResponse = res
            .response
            .parse_struct("Bitpay PaymentsAuthorizeResponse")
            .switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Bitpay
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth = bitpay::BitpayAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let connector_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/invoices/{}?token={}",
            self.base_url(connectors),
            connector_id,
            auth.api_key.peek(),
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: bitpay::BitpayPaymentsResponse = res
            .response
            .parse_struct("bitpay PaymentsSyncResponse")
            .switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Bitpay
{
    fn build_request(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "Bitpay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Bitpay
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Bitpay {
    fn build_request(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Refund flow not Implemented".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Bitpay {
    // default implementation of build_request method will be executed
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Bitpay {
    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif: BitpayWebhookDetails = request
            .body
            .parse_struct("BitpayWebhookDetails")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(notif.data.id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let notif: BitpayWebhookDetails = request
            .body
            .parse_struct("BitpayWebhookDetails")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        match notif.event.name {
            bitpay::WebhookEventType::Confirmed | bitpay::WebhookEventType::Completed => {
                Ok(api::IncomingWebhookEvent::PaymentIntentSuccess)
            }
            bitpay::WebhookEventType::Paid => {
                Ok(api::IncomingWebhookEvent::PaymentIntentProcessing)
            }
            bitpay::WebhookEventType::Declined => {
                Ok(api::IncomingWebhookEvent::PaymentIntentFailure)
            }
            bitpay::WebhookEventType::Unknown
            | bitpay::WebhookEventType::Expired
            | bitpay::WebhookEventType::Invalid
            | bitpay::WebhookEventType::Refunded
            | bitpay::WebhookEventType::Resent => Ok(api::IncomingWebhookEvent::EventNotSupported),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let notif: BitpayWebhookDetails = request
            .body
            .parse_struct("BitpayWebhookDetails")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Encode::<BitpayWebhookDetails>::encode_to_value(&notif)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
    }
}
