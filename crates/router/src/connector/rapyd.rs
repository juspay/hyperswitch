pub mod transformers;
use std::fmt::Debug;

use base64::Engine;
use common_utils::{date_time, ext_traits::StringExt};
use diesel_models::enums;
use error_stack::{IntoReport, Report, ResultExt};
use masking::{ExposeInterface, PeekInterface};
use rand::distributions::{Alphanumeric, DistString};
use ring::hmac;
use transformers as rapyd;

use crate::{
    configs::settings,
    connector::{utils as connector_utils, utils as conn_utils},
    consts,
    core::errors::{self, CustomResult},
    headers, logger,
    services::{
        self,
        request::{self, Mask},
        ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon},
        domain, ErrorResponse,
    },
    utils::{self, crypto, ByteSliceExt, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Rapyd;

impl Rapyd {
    pub fn generate_signature(
        &self,
        auth: &rapyd::RapydAuthType,
        http_method: &str,
        url_path: &str,
        body: &str,
        timestamp: &i64,
        salt: &str,
    ) -> CustomResult<String, errors::ConnectorError> {
        let rapyd::RapydAuthType {
            access_key,
            secret_key,
        } = auth;
        let to_sign = format!(
            "{http_method}{url_path}{salt}{timestamp}{}{}{body}",
            access_key.peek(),
            secret_key.peek()
        );
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret_key.peek().as_bytes());
        let tag = hmac::sign(&key, to_sign.as_bytes());
        let hmac_sign = hex::encode(tag);
        let signature_value = consts::BASE64_ENGINE_URL_SAFE.encode(hmac_sign);
        Ok(signature_value)
    }
}

impl ConnectorCommon for Rapyd {
    fn id(&self) -> &'static str {
        "rapyd"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.rapyd.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: Result<
            rapyd::RapydPaymentsResponse,
            Report<common_utils::errors::ParsingError>,
        > = res.response.parse_struct("Rapyd ErrorResponse");

        match response {
            Ok(response_data) => Ok(ErrorResponse {
                status_code: res.status_code,
                code: response_data.status.error_code,
                message: response_data.status.status.unwrap_or_default(),
                reason: response_data.status.message,
            }),
            Err(error_msg) => {
                logger::error!(deserialization_error =? error_msg);
                utils::handle_json_response_deserialization_failure(res, "rapyd".to_owned())
            }
        }
    }
}

impl ConnectorValidation for Rapyd {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_supported_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl api::ConnectorAccessToken for Rapyd {}

impl api::PaymentToken for Rapyd {}

impl
    services::ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Rapyd
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Rapyd
{
}

impl api::PaymentAuthorize for Rapyd {}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v1/payments", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = rapyd::RapydRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let req_obj = rapyd::RapydPaymentsRequest::try_from(&connector_router_data)?;
        let rapyd_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<rapyd::RapydPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(rapyd_req))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let body = types::PaymentsAuthorizeType::get_request_body(self, req)?
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let req_body = types::RequestBody::get_inner_value(body).expose();
        let signature =
            self.generate_signature(&auth, "post", "/v1/payments", &req_body, &timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key.into_masked()),
            ("salt".to_string(), salt.into_masked()),
            ("timestamp".to_string(), timestamp.to_string().into()),
            ("signature".to_string(), signature.into_masked()),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsAuthorizeType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::PaymentsAuthorizeType::get_headers(
                self, req, connectors,
            )?)
            .headers(headers)
            .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd PaymentResponse")
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

impl api::Payment for Rapyd {}

impl api::MandateSetup for Rapyd {}
impl
    services::ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Rapyd
{
}

impl api::PaymentVoid for Rapyd {}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsVoidType::get_content_type(self)
                .to_string()
                .into(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v1/payments/{}",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let url_path = format!("/v1/payments/{}", req.request.connector_transaction_id);
        let signature =
            self.generate_signature(&auth, "delete", &url_path, "", &timestamp, &salt)?;

        let headers = vec![
            ("access_key".to_string(), auth.access_key.into_masked()),
            ("salt".to_string(), salt.into_masked()),
            ("timestamp".to_string(), timestamp.to_string().into()),
            ("signature".to_string(), signature.into_masked()),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Delete)
            .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
            .headers(headers)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd PaymentResponse")
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

impl api::PaymentSync for Rapyd {}
impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsSyncType::get_content_type(self)
                .to_string()
                .into(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}/v1/payments/{}",
            self.base_url(connectors),
            id.get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let response_id = req.request.connector_transaction_id.clone();
        let url_path = format!(
            "/v1/payments/{}",
            response_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
        );
        let signature = self.generate_signature(&auth, "get", &url_path, "", &timestamp, &salt)?;

        let headers = vec![
            ("access_key".to_string(), auth.access_key.into_masked()),
            ("salt".to_string(), salt.into_masked()),
            ("timestamp".to_string(), timestamp.to_string().into()),
            ("signature".to_string(), signature.into_masked()),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Get)
            .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
            .headers(headers)
            .build();
        Ok(Some(request))
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
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::PaymentCapture for Rapyd {}
impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsCaptureType::get_content_type(self)
                .to_string()
                .into(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = rapyd::RapydRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let req_obj = rapyd::CaptureRequest::try_from(&connector_router_data)?;
        let rapyd_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<rapyd::CaptureRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(rapyd_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let url_path = format!(
            "/v1/payments/{}/capture",
            req.request.connector_transaction_id
        );
        let body = types::PaymentsCaptureType::get_request_body(self, req)?
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let req_body = types::RequestBody::get_inner_value(body).expose();
        let signature =
            self.generate_signature(&auth, "post", &url_path, &req_body, &timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key.into_masked()),
            ("salt".to_string(), salt.into_masked()),
            ("timestamp".to_string(), timestamp.to_string().into()),
            ("signature".to_string(), signature.into_masked()),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsCaptureType::get_headers(
                self, req, connectors,
            )?)
            .headers(headers)
            .body(types::PaymentsCaptureType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("RapydPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v1/payments/{}/capture",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Rapyd {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Rapyd
{
    //TODO: implement sessions flow
}

impl api::Refund for Rapyd {}
impl api::RefundExecute for Rapyd {}
impl api::RefundSync for Rapyd {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Rapyd
{
    fn get_headers(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::RefundExecuteType::get_content_type(self)
                .to_string()
                .into(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        api::ConnectorCommon::common_get_content_type(self)
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v1/refunds", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = rapyd::RapydRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let req_obj = rapyd::RapydRefundRequest::try_from(&connector_router_data)?;
        let rapyd_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<rapyd::RapydRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(rapyd_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let body = types::RefundExecuteType::get_request_body(self, req)?
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let req_body = types::RequestBody::get_inner_value(body).expose();
        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let signature =
            self.generate_signature(&auth, "post", "/v1/refunds", &req_body, &timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key.into_masked()),
            ("salt".to_string(), salt.into_masked()),
            ("timestamp".to_string(), timestamp.to_string().into()),
            ("signature".to_string(), signature.into_masked()),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(headers)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: rapyd::RefundResponse = res
            .response
            .parse_struct("rapyd RefundResponse")
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

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Rapyd
{
    // default implementation of build_request method will be executed
    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: rapyd::RefundResponse = res
            .response
            .parse_struct("rapyd RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Rapyd {
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
        let base64_signature = conn_utils::get_header_key_value("signature", request.headers)?;
        let signature = consts::BASE64_ENGINE_URL_SAFE
            .decode(base64_signature.as_bytes())
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(signature)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_id: &str,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let host = conn_utils::get_header_key_value("host", request.headers)?;
        let connector = self.id();
        let url_path = format!("https://{host}/webhooks/{merchant_id}/{connector}");
        let salt = conn_utils::get_header_key_value("salt", request.headers)?;
        let timestamp = conn_utils::get_header_key_value("timestamp", request.headers)?;
        let stringify_auth = String::from_utf8(connector_webhook_secrets.secret.to_vec())
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        let auth: transformers::RapydAuthType = stringify_auth
            .parse_struct("RapydAuthType")
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let access_key = auth.access_key;
        let secret_key = auth.secret_key;
        let body_string = String::from_utf8(request.body.to_vec())
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert body to UTF-8")?;
        let to_sign = format!(
            "{url_path}{salt}{timestamp}{}{}{body_string}",
            access_key.peek(),
            secret_key.peek()
        );

        Ok(to_sign.into_bytes())
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

        let stringify_auth = String::from_utf8(connector_webhook_secrets.secret.to_vec())
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        let auth: transformers::RapydAuthType = stringify_auth
            .parse_struct("RapydAuthType")
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let secret_key = auth.secret_key;
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret_key.peek().as_bytes());
        let tag = hmac::sign(&key, &message);
        let hmac_sign = hex::encode(tag);
        Ok(hmac_sign.as_bytes().eq(&signature))
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook: transformers::RapydIncomingWebhook = request
            .body
            .parse_struct("RapydIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(match webhook.data {
            transformers::WebhookData::Payment(payment_data) => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(payment_data.id),
                )
            }
            transformers::WebhookData::Refund(refund_data) => {
                api_models::webhooks::ObjectReferenceId::RefundId(
                    api_models::webhooks::RefundIdType::ConnectorRefundId(refund_data.id),
                )
            }
            transformers::WebhookData::Dispute(dispute_data) => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        dispute_data.original_transaction_id,
                    ),
                )
            }
        })
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook: transformers::RapydIncomingWebhook = request
            .body
            .parse_struct("RapydIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(match webhook.webhook_type {
            rapyd::RapydWebhookObjectEventType::PaymentCompleted
            | rapyd::RapydWebhookObjectEventType::PaymentCaptured => {
                api::IncomingWebhookEvent::PaymentIntentSuccess
            }
            rapyd::RapydWebhookObjectEventType::PaymentFailed => {
                api::IncomingWebhookEvent::PaymentIntentFailure
            }
            rapyd::RapydWebhookObjectEventType::PaymentRefundFailed
            | rapyd::RapydWebhookObjectEventType::PaymentRefundRejected => {
                api::IncomingWebhookEvent::RefundFailure
            }
            rapyd::RapydWebhookObjectEventType::RefundCompleted => {
                api::IncomingWebhookEvent::RefundSuccess
            }
            rapyd::RapydWebhookObjectEventType::PaymentDisputeCreated => {
                api::IncomingWebhookEvent::DisputeOpened
            }
            rapyd::RapydWebhookObjectEventType::Unknown => {
                api::IncomingWebhookEvent::EventNotSupported
            }
            rapyd::RapydWebhookObjectEventType::PaymentDisputeUpdated => match webhook.data {
                rapyd::WebhookData::Dispute(data) => api::IncomingWebhookEvent::from(data.status),
                _ => api::IncomingWebhookEvent::EventNotSupported,
            },
        })
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let webhook: transformers::RapydIncomingWebhook = request
            .body
            .parse_struct("RapydIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let res_json = match webhook.data {
            transformers::WebhookData::Payment(payment_data) => {
                let rapyd_response: transformers::RapydPaymentsResponse = payment_data.into();

                utils::Encode::<transformers::RapydPaymentsResponse>::encode_to_value(
                    &rapyd_response,
                )
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?
            }
            transformers::WebhookData::Refund(refund_data) => {
                utils::Encode::<transformers::RefundResponseData>::encode_to_value(&refund_data)
                    .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?
            }
            transformers::WebhookData::Dispute(dispute_data) => {
                utils::Encode::<transformers::DisputeResponseData>::encode_to_value(&dispute_data)
                    .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?
            }
        };
        Ok(res_json)
    }

    fn get_dispute_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::disputes::DisputePayload, errors::ConnectorError> {
        let webhook: transformers::RapydIncomingWebhook = request
            .body
            .parse_struct("RapydIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let webhook_dispute_data = match webhook.data {
            transformers::WebhookData::Dispute(dispute_data) => Ok(dispute_data),
            _ => Err(errors::ConnectorError::WebhookBodyDecodingFailed),
        }?;
        Ok(api::disputes::DisputePayload {
            amount: webhook_dispute_data.amount.to_string(),
            currency: webhook_dispute_data.currency.to_string(),
            dispute_stage: api_models::enums::DisputeStage::Dispute,
            connector_dispute_id: webhook_dispute_data.token,
            connector_reason: Some(webhook_dispute_data.dispute_reason_description),
            connector_reason_code: None,
            challenge_required_by: webhook_dispute_data.due_date,
            connector_status: webhook_dispute_data.status.to_string(),
            created_at: webhook_dispute_data.created_at,
            updated_at: webhook_dispute_data.updated_at,
        })
    }
}
