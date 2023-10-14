pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_utils::ext_traits::ByteSliceExt;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface};
use ring::hmac;
use time::{format_description, OffsetDateTime};
use transformers as worldline;

use super::utils::RefundsRequestData;
use crate::{
    configs::settings::Connectors,
    connector::{utils as connector_utils, utils as conn_utils},
    consts,
    core::errors::{self, CustomResult},
    headers, logger,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse,
    },
    utils::{self, crypto, BytesExt, OptionExt},
};

#[derive(Debug, Clone)]
pub struct Worldline;

impl Worldline {
    pub fn generate_authorization_token(
        &self,
        auth: worldline::WorldlineAuthType,
        http_method: &services::Method,
        content_type: &str,
        date: &str,
        endpoint: &str,
    ) -> CustomResult<String, errors::ConnectorError> {
        let signature_data: String = format!(
            "{}\n{}\n{}\n/{}\n",
            http_method,
            content_type.trim(),
            date.trim(),
            endpoint.trim()
        );
        let worldline::WorldlineAuthType {
            api_key,
            api_secret,
            ..
        } = auth;
        let key = hmac::Key::new(hmac::HMAC_SHA256, api_secret.expose().as_bytes());
        let signed_data = consts::BASE64_ENGINE.encode(hmac::sign(&key, signature_data.as_bytes()));

        Ok(format!("GCS v1HMAC:{}:{signed_data}", api_key.peek()))
    }

    pub fn get_current_date_time() -> CustomResult<String, errors::ConnectorError> {
        let format = format_description::parse(
            "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT",
        )
        .into_report()
        .change_context(errors::ConnectorError::InvalidDateFormat)?;
        OffsetDateTime::now_utc()
            .format(&format)
            .into_report()
            .change_context(errors::ConnectorError::InvalidDateFormat)
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Worldline
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let url = Self::get_url(self, req, connectors)?;
        let endpoint = url.replace(base_url, "");
        let http_method = Self::get_http_method(self);
        let auth = worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let date = Self::get_current_date_time()?;
        let content_type = Self::get_content_type(self);
        let signed_data: String =
            self.generate_authorization_token(auth, &http_method, content_type, &date, &endpoint)?;

        Ok(vec![
            (headers::DATE.to_string(), date.into()),
            (
                headers::AUTHORIZATION.to_string(),
                signed_data.into_masked(),
            ),
            (
                headers::CONTENT_TYPE.to_string(),
                content_type.to_string().into(),
            ),
        ])
    }
}

impl ConnectorCommon for Worldline {
    fn id(&self) -> &'static str {
        "worldline"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.worldline.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: worldline::ErrorResponse = res
            .response
            .parse_struct("Worldline ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let error = response.errors.into_iter().next().unwrap_or_default();
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: error
                .message
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            ..Default::default()
        })
    }
}

impl ConnectorValidation for Worldline {
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

impl api::ConnectorAccessToken for Worldline {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Worldline
{
}

impl api::Payment for Worldline {}

impl api::MandateSetup for Worldline {}
impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Worldline
{
}

impl api::PaymentToken for Worldline {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Worldline
{
    // Not Implemented (R)
}

impl api::PaymentVoid for Worldline {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Worldline
{
    fn get_headers(
        &self,
        req: &types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let auth: worldline::WorldlineAuthType =
            worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        let payment_id: &str = req.request.connector_transaction_id.as_ref();
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}/cancel"
        ))
    }

    fn build_request(
        &self,
        req: &types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(types::PaymentsVoidType::get_http_method(self))
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: worldline::PaymentResponse = res
            .response
            .parse_struct("Worldline PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(payments_cancel_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSync for Worldline {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Worldline
{
    fn get_http_method(&self) -> services::Method {
        services::Method::Get
    }

    fn get_content_type(&self) -> &'static str {
        ""
    }

    fn get_headers(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        let base_url = self.base_url(connectors);
        let auth = worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}"
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(types::PaymentsSyncType::get_http_method(self))
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        logger::debug!(payment_sync_response=?res);
        let mut response: worldline::Payment = res
            .response
            .parse_struct("Worldline Payment")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        response.capture_method = data.request.capture_method.unwrap_or_default();
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::PaymentCapture for Worldline {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Worldline
{
    fn get_headers(
        &self,
        req: &types::RouterData<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::RouterData<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_id = req.request.connector_transaction_id.clone();
        let base_url = self.base_url(connectors);
        let auth = worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}/approve"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RouterData<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = worldline::ApproveRequest::try_from(req)?;

        let worldline_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<worldline::ApproveRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(worldline_req))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(types::PaymentsCaptureType::get_http_method(self))
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsCaptureType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RouterData<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>,
        errors::ConnectorError,
    >
    where
        api::Capture: Clone,
        types::PaymentsCaptureData: Clone,
        types::PaymentsResponseData: Clone,
    {
        logger::debug!(payment_capture_response=?res);
        let mut response: worldline::PaymentResponse = res
            .response
            .parse_struct("Worldline PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        response.payment.capture_method = enums::CaptureMethod::Manual;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Worldline {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Worldline
{
    // Not Implemented
}

impl api::PaymentAuthorize for Worldline {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Worldline
{
    fn get_headers(
        &self,
        req: &types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let auth = worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        Ok(format!("{base_url}v1/{merchant_account_id}/payments"))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = worldline::WorldlineRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = worldline::PaymentsRequest::try_from(&connector_router_data)?;
        let worldline_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<worldline::PaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(worldline_req))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(types::PaymentsAuthorizeType::get_http_method(self))
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
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        logger::debug!(payment_authorize_response=?res);
        let mut response: worldline::PaymentResponse = res
            .response
            .parse_struct("Worldline PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        response.payment.capture_method = data.request.capture_method.unwrap_or_default();
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Worldline {}
impl api::RefundExecute for Worldline {}
impl api::RefundSync for Worldline {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Worldline
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_id = req.request.connector_transaction_id.clone();
        let base_url = self.base_url(connectors);
        let auth = worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}/refund"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = worldline::WorldlineRefundRequest::try_from(req)?;
        let refund_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<worldline::WorldlineRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(refund_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(types::RefundExecuteType::get_http_method(self))
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
        logger::debug!(target: "router::connector::worldline", response=?res);
        let response: worldline::RefundResponse = res
            .response
            .parse_struct("Worldline RefundResponse")
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Worldline
{
    fn get_http_method(&self) -> services::Method {
        services::Method::Get
    }

    fn get_content_type(&self) -> &'static str {
        ""
    }

    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        let base_url = self.base_url(connectors);
        let auth: worldline::WorldlineAuthType =
            worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/refunds/{refund_id}/"
        ))
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(types::RefundSyncType::get_http_method(self))
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        logger::debug!(target: "router::connector::worldline", response=?res);
        let response: worldline::RefundResponse = res
            .response
            .parse_struct("Worldline RefundResponse")
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

fn is_endpoint_verification(headers: &actix_web::http::header::HeaderMap) -> bool {
    headers
        .get("x-gcs-webhooks-endpoint-verification")
        .is_some()
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Worldline {
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
        let header_value = conn_utils::get_header_key_value("X-GCS-Signature", request.headers)?;
        let signature = consts::BASE64_ENGINE
            .decode(header_value.as_bytes())
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(signature)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(request.body.to_vec())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        || -> _ {
            Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    request
                        .body
                        .parse_struct::<worldline::WebhookBody>("WorldlineWebhookEvent")?
                        .payment
                        .parse_value::<worldline::Payment>("WorldlineWebhookObjectId")?
                        .id,
                ),
            ))
        }()
        .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        if is_endpoint_verification(request.headers) {
            Ok(api::IncomingWebhookEvent::EndpointVerification)
        } else {
            let details: worldline::WebhookBody = request
                .body
                .parse_struct("WorldlineWebhookObjectId")
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
            let event = match details.event_type {
                worldline::WebhookEvent::Paid => api::IncomingWebhookEvent::PaymentIntentSuccess,
                worldline::WebhookEvent::Rejected | worldline::WebhookEvent::RejectedCapture => {
                    api::IncomingWebhookEvent::PaymentIntentFailure
                }
                worldline::WebhookEvent::Unknown => api::IncomingWebhookEvent::EventNotSupported,
            };
            Ok(event)
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let details = request
            .body
            .parse_struct::<worldline::WebhookBody>("WorldlineWebhookObjectId")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?
            .payment
            .ok_or(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(details)
    }

    fn get_webhook_api_response(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ConnectorError>
    {
        let verification_header = request.headers.get("x-gcs-webhooks-endpoint-verification");
        let response = match verification_header {
            None => services::api::ApplicationResponse::StatusOk,
            Some(header_value) => {
                let verification_signature_value = header_value
                    .to_str()
                    .into_report()
                    .change_context(errors::ConnectorError::WebhookResponseEncodingFailed)?
                    .to_string();
                services::api::ApplicationResponse::TextPlain(verification_signature_value)
            }
        };
        Ok(response)
    }
}
