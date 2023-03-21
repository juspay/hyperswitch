mod transformers;
use std::fmt::Debug;

use base64::Engine;
use common_utils::{date_time, ext_traits::StringExt};
use error_stack::{IntoReport, ResultExt};
use rand::distributions::{Alphanumeric, DistString};
use ring::hmac;
use transformers as rapyd;

use crate::{
    configs::settings,
    connector::utils as conn_utils,
    consts,
    core::errors::{self, CustomResult},
    db::StorageInterface,
    headers, services,
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse,
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
        let to_sign =
            format!("{http_method}{url_path}{salt}{timestamp}{access_key}{secret_key}{body}");
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret_key.as_bytes());
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

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.rapyd.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status.error_code,
            message: response.status.status.unwrap_or_default(),
            reason: response.status.message,
        })
    }
}

impl api::ConnectorAccessToken for Rapyd {}

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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self).to_string(),
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

        let rapyd_req = utils::Encode::<rapyd::RapydPaymentsRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let signature =
            self.generate_signature(&auth, "post", "/v1/payments", &rapyd_req, &timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key),
            ("salt".to_string(), salt),
            ("timestamp".to_string(), timestamp.to_string()),
            ("signature".to_string(), signature),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsAuthorizeType::get_url(
                self, req, connectors,
            )?)
            .headers(types::PaymentsAuthorizeType::get_headers(
                self, req, connectors,
            )?)
            .headers(headers)
            .body(Some(rapyd_req))
            .build();
        Ok(Some(request))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let rapyd_req = utils::Encode::<rapyd::RapydPaymentsRequest>::convert_and_url_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(rapyd_req))
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

impl api::PreVerify for Rapyd {}
impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsVoidType::get_content_type(self).to_string(),
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
            ("access_key".to_string(), auth.access_key),
            ("salt".to_string(), salt),
            ("timestamp".to_string(), timestamp.to_string()),
            ("signature".to_string(), signature),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Delete)
            .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsSyncType::get_content_type(self).to_string(),
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
            ("access_key".to_string(), auth.access_key),
            ("salt".to_string(), salt),
            ("timestamp".to_string(), timestamp.to_string()),
            ("signature".to_string(), signature),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Get)
            .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsCaptureType::get_content_type(self).to_string(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let rapyd_req = utils::Encode::<rapyd::CaptureRequest>::convert_and_encode(req)
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

        let rapyd_req = utils::Encode::<rapyd::CaptureRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let url_path = format!(
            "/v1/payments/{}/capture",
            req.request.connector_transaction_id
        );
        let signature =
            self.generate_signature(&auth, "post", &url_path, &rapyd_req, &timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key),
            ("salt".to_string(), salt),
            ("timestamp".to_string(), timestamp.to_string()),
            ("signature".to_string(), signature),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
            .headers(types::PaymentsCaptureType::get_headers(
                self, req, connectors,
            )?)
            .headers(headers)
            .body(Some(rapyd_req))
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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::RefundExecuteType::get_content_type(self).to_string(),
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
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let rapyd_req = utils::Encode::<rapyd::RapydRefundRequest>::convert_and_url_encode(req)
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

        let rapyd_req = utils::Encode::<rapyd::RapydRefundRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let signature =
            self.generate_signature(&auth, "post", "/v1/refunds", &rapyd_req, &timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key),
            ("salt".to_string(), salt),
            ("timestamp".to_string(), timestamp.to_string()),
            ("signature".to_string(), signature),
        ];
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .headers(headers)
            .body(Some(rapyd_req))
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
    fn get_headers(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("RSync".to_string()).into())
    }

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

    fn get_error_response(
        &self,
        _res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("RSync".to_string()).into())
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
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let base64_signature = conn_utils::get_header_key_value("signature", request.headers)?;
        let signature = consts::BASE64_ENGINE_URL_SAFE
            .decode(base64_signature.as_bytes())
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(signature)
    }

    async fn get_webhook_source_verification_merchant_secret(
        &self,
        db: &dyn StorageInterface,
        merchant_id: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let key = format!("wh_mer_sec_verification_{}_{}", self.id(), merchant_id);
        let secret = db
            .get_key(&key)
            .await
            .change_context(errors::ConnectorError::WebhookVerificationSecretNotFound)?;

        Ok(secret)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_id: &str,
        secret: &[u8],
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let host = conn_utils::get_header_key_value("host", request.headers)?;
        let connector = self.id();
        let url_path = format!("https://{host}/webhooks/{merchant_id}/{connector}");
        let salt = conn_utils::get_header_key_value("salt", request.headers)?;
        let timestamp = conn_utils::get_header_key_value("timestamp", request.headers)?;
        let stringify_auth = String::from_utf8(secret.to_vec())
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
        let to_sign = format!("{url_path}{salt}{timestamp}{access_key}{secret_key}{body_string}");

        Ok(to_sign.into_bytes())
    }

    async fn verify_webhook_source(
        &self,
        db: &dyn StorageInterface,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let signature = self
            .get_webhook_source_verification_signature(request)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let secret = self
            .get_webhook_source_verification_merchant_secret(db, merchant_id)
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let message = self
            .get_webhook_source_verification_message(request, merchant_id, &secret)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let stringify_auth = String::from_utf8(secret.to_vec())
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        let auth: transformers::RapydAuthType = stringify_auth
            .parse_struct("RapydAuthType")
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let secret_key = auth.secret_key;
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret_key.as_bytes());
        let tag = hmac::sign(&key, &message);
        let hmac_sign = hex::encode(tag);
        Ok(hmac_sign.as_bytes().eq(&signature))
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<String, errors::ConnectorError> {
        let webhook: transformers::RapydIncomingWebhook = request
            .body
            .parse_struct("RapydIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(match webhook.data {
            transformers::WebhookData::PaymentData(payment_data) => payment_data.id,
            transformers::WebhookData::RefundData(refund_data) => refund_data.id,
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

        webhook.webhook_type.try_into()
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let webhook: transformers::RapydIncomingWebhook = request
            .body
            .parse_struct("RapydIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let response = match webhook.data {
            transformers::WebhookData::PaymentData(payment_data) => {
                let rapyd_response: transformers::RapydPaymentsResponse = payment_data.into();
                Ok(rapyd_response)
            }
            _ => Err(errors::ConnectorError::WebhookEventTypeNotFound),
        }?;
        let res_json =
            utils::Encode::<transformers::RapydPaymentsResponse>::encode_to_value(&response)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(res_json)
    }
}
