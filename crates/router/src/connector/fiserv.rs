pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface};
use ring::hmac;
use time::OffsetDateTime;
use transformers as fiserv;
use uuid::Uuid;

use crate::{
    configs::settings,
    connector::utils as connector_utils,
    consts,
    core::errors::{self, CustomResult},
    headers, logger,
    services::{
        self,
        api::ConnectorIntegration,
        request::{self, Mask},
        ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Fiserv;

impl Fiserv {
    pub fn generate_authorization_signature(
        &self,
        auth: fiserv::FiservAuthType,
        request_id: &str,
        payload: &str,
        timestamp: i128,
    ) -> CustomResult<String, errors::ConnectorError> {
        let fiserv::FiservAuthType {
            api_key,
            api_secret,
            ..
        } = auth;
        let raw_signature = format!("{}{request_id}{timestamp}{payload}", api_key.peek());

        let key = hmac::Key::new(hmac::HMAC_SHA256, api_secret.expose().as_bytes());
        let signature_value =
            consts::BASE64_ENGINE.encode(hmac::sign(&key, raw_signature.as_bytes()).as_ref());
        Ok(signature_value)
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Fiserv
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
        let auth: fiserv::FiservAuthType =
            fiserv::FiservAuthType::try_from(&req.connector_auth_type)?;
        let mut auth_header = self.get_auth_header(&req.connector_auth_type)?;

        let fiserv_req = self
            .get_request_body(req)?
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;

        let client_request_id = Uuid::new_v4().to_string();
        let hmac = self
            .generate_authorization_signature(
                auth,
                &client_request_id,
                types::RequestBody::get_inner_value(fiserv_req).peek(),
                timestamp,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            ("Client-Request-Id".to_string(), client_request_id.into()),
            ("Auth-Token-Type".to_string(), "HMAC".to_string().into()),
            (headers::TIMESTAMP.to_string(), timestamp.to_string().into()),
            (headers::AUTHORIZATION.to_string(), hmac.into_masked()),
        ];
        headers.append(&mut auth_header);
        Ok(headers)
    }
}

impl ConnectorCommon for Fiserv {
    fn id(&self) -> &'static str {
        "fiserv"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.fiserv.base_url.as_ref()
    }
    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: fiserv::FiservAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::API_KEY.to_string(),
            auth.api_key.into_masked(),
        )])
    }
    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: fiserv::ErrorResponse = res
            .response
            .parse_struct("Fiserv ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let fiserv::ErrorResponse { error, details } = response;

        Ok(error
            .or(details)
            .and_then(|error_details| {
                error_details
                    .first()
                    .map(|first_error| types::ErrorResponse {
                        code: first_error
                            .code
                            .to_owned()
                            .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                        message: first_error.message.to_owned(),
                        reason: first_error.field.to_owned(),
                        status_code: res.status_code,
                    })
            })
            .unwrap_or(types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: consts::NO_ERROR_MESSAGE.to_string(),
                reason: None,
                status_code: res.status_code,
            }))
    }
}

impl ConnectorValidation for Fiserv {
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

impl api::ConnectorAccessToken for Fiserv {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Fiserv
{
    // Not Implemented (R)
}

impl api::Payment for Fiserv {}

impl api::PaymentToken for Fiserv {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Fiserv
{
    // Not Implemented (R)
}

impl api::MandateSetup for Fiserv {}

#[allow(dead_code)]
impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Fiserv
{
}

impl api::PaymentVoid for Fiserv {}

#[allow(dead_code)]
impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Fiserv
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            //The docs has this url wrong, cancels is the working endpoint
            "{}ch/payments/v1/cancels",
            connectors.fiserv.base_url
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = fiserv::FiservCancelRequest::try_from(req)?;
        let fiserv_payments_cancel_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<fiserv::FiservCancelRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(fiserv_payments_cancel_request))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .body(types::PaymentsVoidType::get_request_body(self, req)?)
                .build(),
        );

        Ok(request)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: fiserv::FiservPaymentsResponse = res
            .response
            .parse_struct("Fiserv PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSync for Fiserv {}

#[allow(dead_code)]
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Fiserv
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ch/payments/v1/transaction-inquiry",
            connectors.fiserv.base_url
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = fiserv::FiservSyncRequest::try_from(req)?;
        let fiserv_payments_sync_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<fiserv::FiservSyncRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(fiserv_payments_sync_request))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .body(types::PaymentsSyncType::get_request_body(self, req)?)
                .build(),
        );
        Ok(request)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: fiserv::FiservSyncResponse = res
            .response
            .parse_struct("Fiserv PaymentSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentCapture for Fiserv {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Fiserv
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let router_obj = fiserv::FiservRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let connector_request = fiserv::FiservCaptureRequest::try_from(&router_obj)?;
        let fiserv_payments_capture_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<fiserv::FiservCaptureRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(fiserv_payments_capture_request))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsCaptureType::get_request_body(self, req)?)
                .build(),
        );
        Ok(request)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: fiserv::FiservPaymentsResponse = res
            .response
            .parse_struct("Fiserv Payment Response")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ch/payments/v1/charges",
            connectors.fiserv.base_url
        ))
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Fiserv {}

#[allow(dead_code)]
impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Fiserv
{
}

impl api::PaymentAuthorize for Fiserv {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Fiserv
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ch/payments/v1/charges",
            connectors.fiserv.base_url
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let router_obj = fiserv::FiservRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_request = fiserv::FiservPaymentsRequest::try_from(&router_obj)?;
        let fiserv_payments_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<fiserv::FiservPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(fiserv_payments_request))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = Some(
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
        );

        Ok(request)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: fiserv::FiservPaymentsResponse = res
            .response
            .parse_struct("Fiserv PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Fiserv {}
impl api::RefundExecute for Fiserv {}
impl api::RefundSync for Fiserv {}

#[allow(dead_code)]
impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Fiserv {
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        "application/json"
    }
    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ch/payments/v1/refunds",
            connectors.fiserv.base_url
        ))
    }
    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let router_obj = fiserv::FiservRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_request = fiserv::FiservRefundRequest::try_from(&router_obj)?;
        let fiserv_refund_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<fiserv::FiservRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(fiserv_refund_request))
    }
    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
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
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        logger::debug!(target: "router::connector::fiserv", response=?res);
        let response: fiserv::RefundResponse =
            res.response
                .parse_struct("fiserv RefundResponse")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[allow(dead_code)]
impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Fiserv {
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ch/payments/v1/transaction-inquiry",
            connectors.fiserv.base_url
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundSyncRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = fiserv::FiservSyncRequest::try_from(req)?;
        let fiserv_sync_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<fiserv::FiservSyncRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(fiserv_sync_request))
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        );
        Ok(request)
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        logger::debug!(target: "router::connector::fiserv", response=?res);

        let response: fiserv::FiservSyncResponse = res
            .response
            .parse_struct("Fiserv Refund Response")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Fiserv {
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
