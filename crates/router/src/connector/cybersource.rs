pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface};
use ring::{digest, hmac};
use time::OffsetDateTime;
use transformers as cybersource;
use url::Url;

use crate::{
    configs::settings,
    connector::{utils as connector_utils, utils::RefundsRequestData},
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
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Cybersource;

impl Cybersource {
    pub fn generate_digest(&self, payload: &[u8]) -> String {
        let payload_digest = digest::digest(&digest::SHA256, payload);
        consts::BASE64_ENGINE.encode(payload_digest)
    }

    pub fn generate_signature(
        &self,
        auth: cybersource::CybersourceAuthType,
        host: String,
        resource: &str,
        payload: &String,
        date: OffsetDateTime,
        http_method: services::Method,
    ) -> CustomResult<String, errors::ConnectorError> {
        let cybersource::CybersourceAuthType {
            api_key,
            merchant_account,
            api_secret,
        } = auth;
        let is_post_method = matches!(http_method, services::Method::Post);
        let digest_str = if is_post_method { "digest " } else { "" };
        let headers = format!("host date (request-target) {digest_str}v-c-merchant-id");
        let request_target = if is_post_method {
            format!("(request-target): post {resource}\ndigest: SHA-256={payload}\n")
        } else {
            format!("(request-target): get {resource}\n")
        };
        let signature_string = format!(
            "host: {host}\ndate: {date}\n{request_target}v-c-merchant-id: {}",
            merchant_account.peek()
        );
        let key_value = consts::BASE64_ENGINE
            .decode(api_secret.expose())
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let key = hmac::Key::new(hmac::HMAC_SHA256, &key_value);
        let signature_value =
            consts::BASE64_ENGINE.encode(hmac::sign(&key, signature_string.as_bytes()).as_ref());
        let signature_header = format!(
            r#"keyid="{}", algorithm="HmacSHA256", headers="{headers}", signature="{signature_value}""#,
            api_key.peek()
        );

        Ok(signature_header)
    }
}

impl ConnectorCommon for Cybersource {
    fn id(&self) -> &'static str {
        "cybersource"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json;charset=utf-8"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.cybersource.base_url.as_ref()
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: cybersource::ErrorResponse = res
            .response
            .parse_struct("Cybersource ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let details = response.details.unwrap_or_default();
        let connector_reason = details
            .iter()
            .map(|det| format!("{} : {}", det.field, det.reason))
            .collect::<Vec<_>>()
            .join(", ");

        let error_message = if res.status_code == 401 {
            consts::CONNECTOR_UNAUTHORIZED_ERROR
        } else {
            consts::NO_ERROR_MESSAGE
        };

        let (code, message) = match response.error_information {
            Some(ref error_info) => (error_info.reason.clone(), error_info.message.clone()),
            None => (
                response
                    .reason
                    .map_or(consts::NO_ERROR_CODE.to_string(), |reason| {
                        reason.to_string()
                    }),
                response
                    .message
                    .map_or(error_message.to_string(), |message| message),
            ),
        };
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code,
            message,
            reason: Some(connector_reason),
        })
    }
}

impl ConnectorValidation for Cybersource {
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

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Cybersource
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, services::request::Maskable<String>)>, errors::ConnectorError>
    {
        let date = OffsetDateTime::now_utc();
        let cybersource_req = self.get_request_body(req)?;
        let auth = cybersource::CybersourceAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account = auth.merchant_account.clone();
        let base_url = connectors.cybersource.base_url.as_str();
        let cybersource_host = Url::parse(base_url)
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let host = cybersource_host
            .host_str()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let path: String = self
            .get_url(req, connectors)?
            .chars()
            .skip(base_url.len() - 1)
            .collect();
        let sha256 = self.generate_digest(
            cybersource_req
                .map_or("{}".to_string(), |s| {
                    types::RequestBody::get_inner_value(s).expose()
                })
                .as_bytes(),
        );
        let http_method = self.get_http_method();
        let signature = self.generate_signature(
            auth,
            host.to_string(),
            path.as_str(),
            &sha256,
            date,
            http_method,
        )?;

        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),
                "application/hal+json;charset=utf-8".to_string().into(),
            ),
            (
                "v-c-merchant-id".to_string(),
                merchant_account.into_masked(),
            ),
            ("Date".to_string(), date.to_string().into()),
            ("Host".to_string(), host.to_string().into()),
            ("Signature".to_string(), signature.into_masked()),
        ];
        if matches!(http_method, services::Method::Post | services::Method::Put) {
            headers.push((
                "Digest".to_string(),
                format!("SHA-256={sha256}").into_masked(),
            ));
        }
        Ok(headers)
    }
}

impl api::Payment for Cybersource {}
impl api::PaymentAuthorize for Cybersource {}
impl api::PaymentSync for Cybersource {}
impl api::PaymentVoid for Cybersource {}
impl api::PaymentCapture for Cybersource {}
impl api::MandateSetup for Cybersource {}
impl api::ConnectorAccessToken for Cybersource {}
impl api::PaymentToken for Cybersource {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Cybersource
{
    // Not Implemented (R)
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Cybersource
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Cybersource
{
    // Not Implemented (R)
}

impl api::PaymentSession for Cybersource {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Cybersource
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Cybersource
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
            "{}pts/v2/payments/{}/captures",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = cybersource::CybersourcePaymentsRequest::try_from(req)?;
        let cybersource_payments_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<cybersource::CybersourcePaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(cybersource_payments_request))
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
        data: &types::PaymentsCaptureRouterData,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>,
        errors::ConnectorError,
    > {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            true,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, services::request::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_http_method(&self) -> services::Method {
        services::Method::Get
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}tss/v2/transactions/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        Ok(Some(
            types::RequestBody::log_and_get_request_body("{}".to_string(), Ok)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
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
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceTransactionResponse = res
            .response
            .parse_struct("Cybersource PaymentSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let is_auto_capture =
            data.request.capture_method == Some(diesel_models::enums::CaptureMethod::Automatic);
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            is_auto_capture,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Cybersource
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
        Ok(format!(
            "{}pts/v2/payments/",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = cybersource::CybersourceRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_request =
            cybersource::CybersourcePaymentsRequest::try_from(&connector_router_data)?;
        let cybersource_payments_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<cybersource::CybersourcePaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(cybersource_payments_request))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsAuthorizeType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::PaymentsAuthorizeType::get_headers(
                self, req, connectors,
            )?)
            .body(self.get_request_body(req)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let is_auto_capture =
            data.request.capture_method == Some(diesel_models::enums::CaptureMethod::Automatic);
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            is_auto_capture,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{}/voids",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        Ok(Some(
            types::RequestBody::log_and_get_request_body("{}".to_string(), Ok)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
        ))
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
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .body(self.get_request_body(req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            false,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Cybersource {}
impl api::RefundExecute for Cybersource {}
impl api::RefundSync for Cybersource {}

#[allow(dead_code)]
impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &types::RefundExecuteRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundExecuteRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{}/refunds",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundExecuteRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = cybersource::CybersourceRefundRequest::try_from(req)?;
        let cybersource_refund_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<cybersource::CybersourceRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(cybersource_refund_request))
    }
    fn build_request(
        &self,
        req: &types::RefundExecuteRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundExecuteType::get_headers(
                    self, req, connectors,
                )?)
                .body(self.get_request_body(req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundExecuteRouterData,
        res: types::Response,
    ) -> CustomResult<types::RefundExecuteRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
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
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[allow(dead_code)]
impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_http_method(&self) -> services::Method {
        services::Method::Get
    }
    fn get_url(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}tss/v2/transactions/{}",
            self.base_url(connectors),
            refund_id
        ))
    }
    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceTransactionResponse = res
            .response
            .parse_struct("Cybersource RefundsSyncResponse")
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
impl api::IncomingWebhook for Cybersource {
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
