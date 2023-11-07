mod transformers;

use std::fmt::Debug;

use base64::Engine;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use rand::distributions::DistString;
use ring::hmac;
use transformers as payeezy;

use crate::{
    configs::settings,
    connector::utils as connector_utils,
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
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Payeezy;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Payeezy
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = payeezy::PayeezyAuthType::try_from(&req.connector_auth_type)?;
        let option_request_payload = self.get_request_body(req)?;
        let request_payload = option_request_payload.map_or("{}".to_string(), |payload| {
            types::RequestBody::get_inner_value(payload).expose()
        });
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?
            .as_millis()
            .to_string();
        let nonce = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 19);
        let signature_string = auth.api_key.clone().zip(auth.merchant_token.clone()).map(
            |(api_key, merchant_token)| {
                format!(
                    "{}{}{}{}{}",
                    api_key, nonce, timestamp, merchant_token, request_payload
                )
            },
        );
        let key = hmac::Key::new(hmac::HMAC_SHA256, auth.api_secret.expose().as_bytes());
        let tag = hmac::sign(&key, signature_string.expose().as_bytes());
        let hmac_sign = hex::encode(tag);
        let signature_value = consts::BASE64_ENGINE_URL_SAFE.encode(hmac_sign);
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                Self.get_content_type().to_string().into(),
            ),
            (headers::APIKEY.to_string(), auth.api_key.into_masked()),
            (
                headers::TOKEN.to_string(),
                auth.merchant_token.into_masked(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                signature_value.into_masked(),
            ),
            (headers::NONCE.to_string(), nonce.into_masked()),
            (headers::TIMESTAMP.to_string(), timestamp.into()),
        ])
    }
}

impl ConnectorCommon for Payeezy {
    fn id(&self) -> &'static str {
        "payeezy"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.payeezy.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: payeezy::PayeezyErrorResponse = res
            .response
            .parse_struct("payeezy ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let error_messages: Vec<String> = response
            .error
            .messages
            .iter()
            .map(|m| m.description.clone())
            .collect();

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.transaction_status,
            message: error_messages.join(", "),
            reason: None,
        })
    }
}

impl ConnectorValidation for Payeezy {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl api::Payment for Payeezy {}

impl api::MandateSetup for Payeezy {}
impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Payeezy
{
}

impl api::PaymentToken for Payeezy {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Payeezy
{
    // Not Implemented (R)
}

impl api::PaymentVoid for Payeezy {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Payeezy
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}v1/transactions/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = payeezy::PayeezyCaptureOrVoidRequest::try_from(req)?;
        let payeezy_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<payeezy::PayeezyCaptureOrVoidRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payeezy_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .body(types::PaymentsVoidType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: payeezy::PayeezyPaymentsResponse = res
            .response
            .parse_struct("Payeezy PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::ConnectorAccessToken for Payeezy {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Payeezy
{
}

impl api::PaymentSync for Payeezy {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Payeezy
{
    // default implementation of build_request method will be executed
}

impl api::PaymentCapture for Payeezy {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Payeezy
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}v1/transactions/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let router_obj = payeezy::PayeezyRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let req_obj = payeezy::PayeezyCaptureOrVoidRequest::try_from(&router_obj)?;
        let payeezy_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payeezy::PayeezyCaptureOrVoidRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(payeezy_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsCaptureType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: payeezy::PayeezyPaymentsResponse = res
            .response
            .parse_struct("Payeezy PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Payeezy {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Payeezy
{
    //TODO: implement sessions flow
}

impl api::PaymentAuthorize for Payeezy {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Payeezy
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
        Ok(format!("{}v1/transactions", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let router_obj = payeezy::PayeezyRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let req_obj = payeezy::PayeezyPaymentsRequest::try_from(&router_obj)?;

        let payeezy_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payeezy::PayeezyPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payeezy_req))
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
        let response: payeezy::PayeezyPaymentsResponse = res
            .response
            .parse_struct("payeezy Response")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Payeezy {}
impl api::RefundExecute for Payeezy {}
impl api::RefundSync for Payeezy {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Payeezy
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}v1/transactions/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let router_obj = payeezy::PayeezyRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let req_obj = payeezy::PayeezyRefundRequest::try_from(&router_obj)?;
        let payeezy_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payeezy::PayeezyRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payeezy_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        // Parse the response into a payeezy::RefundResponse
        let response: payeezy::RefundResponse = res
            .response
            .parse_struct("payeezy RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        // Create a new instance of types::RefundsRouterData based on the response, input data, and HTTP code
        let response_data = types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        };
        let router_data = types::RefundsRouterData::try_from(response_data)
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        Ok(router_data)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Payeezy {
    // default implementation of build_request method will be executed
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Payeezy {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Ok(api::IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
