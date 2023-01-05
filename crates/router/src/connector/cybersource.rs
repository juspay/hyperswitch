mod transformers;

use std::fmt::Debug;

use base64;
use bytes::Bytes;
use error_stack::{IntoReport, ResultExt};
use ring::{digest, hmac};
use time::OffsetDateTime;
use transformers as cybersource;
use url::Url;

use crate::{
    configs::settings,
    consts,
    core::errors::{self, CustomResult},
    headers, logger, services,
    types::{self, api},
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Cybersource;

impl Cybersource {
    pub fn generate_digest(&self, payload: &[u8]) -> String {
        let payload_digest = digest::digest(&digest::SHA256, payload);
        base64::encode(payload_digest)
    }

    pub fn generate_signature(
        &self,
        auth: cybersource::CybersourceAuthType,
        host: String,
        resource: &str,
        payload: &str,
        date: OffsetDateTime,
    ) -> CustomResult<String, errors::ConnectorError> {
        let cybersource::CybersourceAuthType {
            api_key,
            merchant_account,
            api_secret,
        } = auth;

        let headers_for_post_method = "host date (request-target) digest v-c-merchant-id";
        let signature_string = format!(
            "host: {host}\n\
             date: {date}\n\
             (request-target): post {resource}\n\
             digest: SHA-256={}\n\
             v-c-merchant-id: {merchant_account}",
            self.generate_digest(payload.as_bytes())
        );
        let key_value = base64::decode(api_secret)
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let key = hmac::Key::new(hmac::HMAC_SHA256, &key_value);
        let signature_value =
            base64::encode(hmac::sign(&key, signature_string.as_bytes()).as_ref());
        let signature_header = format!(
            r#"keyid="{api_key}", algorithm="HmacSHA256", headers="{headers_for_post_method}", signature="{signature_value}""#
        );

        Ok(signature_header)
    }
}

impl api::ConnectorCommon for Cybersource {
    fn id(&self) -> &'static str {
        "cybersource"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.cybersource.base_url.as_ref()
    }
}

impl api::Payment for Cybersource {}
impl api::PaymentAuthorize for Cybersource {}
impl api::PaymentSync for Cybersource {}
impl api::PaymentVoid for Cybersource {}
impl api::PaymentCapture for Cybersource {}
impl api::PreVerify for Cybersource {}

impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Cybersource
{
}

impl api::PaymentSession for Cybersource {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Cybersource
{
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Cybersource
{
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Cybersource
{
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Cybersource
{
    fn get_headers(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (
                headers::ACCEPT.to_string(),
                "application/hal+json;charset=utf-8".to_string(),
            ),
        ];
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json;charset=utf-8"
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

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let date = OffsetDateTime::now_utc();

        let cybersource_req =
            utils::Encode::<cybersource::CybersourcePaymentsRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let auth: cybersource::CybersourceAuthType =
            cybersource::CybersourceAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account = auth.merchant_account.clone();

        let cybersource_host = Url::parse(connectors.cybersource.base_url.as_str())
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        match cybersource_host.host_str() {
            Some(host) => {
                let signature = self.generate_signature(
                    auth,
                    host.to_string(),
                    "/pts/v2/payments/",
                    &cybersource_req,
                    date,
                )?;
                let headers = vec![
                    (
                        "Digest".to_string(),
                        format!(
                            "SHA-256={}",
                            self.generate_digest(cybersource_req.as_bytes())
                        ),
                    ),
                    ("v-c-merchant-id".to_string(), merchant_account),
                    ("Date".to_string(), date.to_string()),
                    ("Host".to_string(), host.to_string()),
                    ("Signature".to_string(), signature),
                ];
                let request = services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::PaymentsAuthorizeType::get_url(
                        self, req, connectors,
                    )?)
                    .headers(headers)
                    .headers(types::PaymentsAuthorizeType::get_headers(
                        self, req, connectors,
                    )?)
                    .body(Some(cybersource_req))
                    .build();

                Ok(Some(request))
            }
            None => Err(errors::ConnectorError::RequestEncodingFailed.into()),
        }
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
        logger::debug!(cybersourcepayments_create_response=?response);
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
        res: Bytes,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: cybersource::ErrorResponse = res
            .parse_struct("Cybersource ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ErrorResponse {
            code: consts::NO_ERROR_CODE.to_string(),
            message: response
                .message
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
        })
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Cybersource
{
}

impl api::Refund for Cybersource {}
impl api::RefundExecute for Cybersource {}
impl api::RefundSync for Cybersource {}

#[allow(dead_code)]
impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Cybersource
{
}

#[allow(dead_code)]
impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Cybersource
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Cybersource {
    fn get_webhook_object_reference_id(
        &self,
        _body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("cybersource".to_string()).into())
    }

    fn get_webhook_event_type(
        &self,
        _body: &[u8],
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("cybersource".to_string()).into())
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("cybersource".to_string()).into())
    }
}

impl services::ConnectorRedirectResponse for Cybersource {}
