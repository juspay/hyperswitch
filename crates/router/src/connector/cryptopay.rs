pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_utils::{
    crypto::{self, GenerateDigest, SignMessage},
    date_time,
    ext_traits::ByteSliceExt,
    request::RequestContent,
};
use error_stack::{IntoReport, ResultExt};
use hex::encode;
use masking::PeekInterface;
use transformers as cryptopay;

use self::cryptopay::CryptopayWebhookDetails;
use crate::{
    configs::settings,
    connector::utils::{self as connector_utils, PaymentMethodDataType},
    consts,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers, is_mandate_supported, mandate_not_supported_error,
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
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Cryptopay;

impl api::Payment for Cryptopay {}
impl api::PaymentSession for Cryptopay {}
impl api::ConnectorAccessToken for Cryptopay {}
impl api::MandateSetup for Cryptopay {}
impl api::PaymentAuthorize for Cryptopay {}
impl api::PaymentSync for Cryptopay {}
impl api::PaymentCapture for Cryptopay {}
impl api::PaymentVoid for Cryptopay {}
impl api::Refund for Cryptopay {}
impl api::RefundExecute for Cryptopay {}
impl api::RefundSync for Cryptopay {}
impl api::PaymentToken for Cryptopay {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Cryptopay
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Cryptopay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let method = self.get_http_method();
        let payload = match method {
            common_utils::request::Method::Get => String::default(),
            common_utils::request::Method::Post
            | common_utils::request::Method::Put
            | common_utils::request::Method::Delete
            | common_utils::request::Method::Patch => {
                let body = self
                    .get_request_body(req, connectors)?
                    .get_inner_value()
                    .peek()
                    .to_owned();
                let md5_payload = crypto::Md5
                    .generate_digest(body.as_bytes())
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                encode(md5_payload)
            }
        };
        let api_method = method.to_string();

        let now = date_time::date_as_yyyymmddthhmmssmmmz()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let date = format!("{}+00:00", now.split_at(now.len() - 5).0);

        let content_type = self.get_content_type().to_string();

        let api = (self.get_url(req, connectors)?).replace(self.base_url(connectors), "");

        let auth = cryptopay::CryptopayAuthType::try_from(&req.connector_auth_type)?;

        let sign_req: String = format!(
            "{}\n{}\n{}\n{}\n{}",
            api_method, payload, content_type, date, api
        );
        let authz = crypto::HmacSha1::sign_message(
            &crypto::HmacSha1,
            auth.api_secret.peek().as_bytes(),
            sign_req.as_bytes(),
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to sign the message")?;
        let authz = consts::BASE64_ENGINE.encode(authz);
        let auth_string: String = format!("HMAC {}:{}", auth.api_key.peek(), authz);

        let headers = vec![
            (
                headers::AUTHORIZATION.to_string(),
                auth_string.into_masked(),
            ),
            (headers::DATE.to_string(), date.into()),
            (
                headers::CONTENT_TYPE.to_string(),
                Self.get_content_type().to_string().into(),
            ),
        ];
        Ok(headers)
    }
}

impl ConnectorCommon for Cryptopay {
    fn id(&self) -> &'static str {
        "cryptopay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.cryptopay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = cryptopay::CryptopayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.peek().to_owned().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: cryptopay::CryptopayErrorResponse = res
            .response
            .parse_struct("CryptopayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.code,
            message: response.error.message,
            reason: response.error.reason,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Cryptopay
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Cryptopay
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Cryptopay
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
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Cryptopay".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Cryptopay
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
        Ok(format!("{}/api/invoices", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = cryptopay::CryptopayRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = cryptopay::CryptopayPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
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
                .set_body(types::PaymentsAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: cryptopay::CryptopayPaymentsResponse = res
            .response
            .parse_struct("Cryptopay PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
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

impl ConnectorValidation for Cryptopay {
    fn validate_psync_reference_id(
        &self,
        _data: &types::PaymentsSyncRouterData,
    ) -> CustomResult<(), errors::ConnectorError> {
        // since we can make psync call with our reference_id, having connector_transaction_id is not an mandatory criteria
        Ok(())
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<types::storage::enums::PaymentMethodType>,
        pm_data: api_models::payments::PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::<PaymentMethodDataType>::new();
        is_mandate_supported!(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Cryptopay
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

    fn get_http_method(&self) -> services::Method {
        services::Method::Get
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let custom_id = req.connector_request_reference_id.clone();
        Ok(format!(
            "{}/api/invoices/custom_id/{custom_id}",
            self.base_url(connectors)
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
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: cryptopay::CryptopayPaymentsResponse = res
            .response
            .parse_struct("cryptopay PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
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

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Cryptopay
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Cryptopay
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Cryptopay
{
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Cryptopay
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Cryptopay {
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
        let base64_signature =
            connector_utils::get_header_key_value("X-Cryptopay-Signature", request.headers)?;
        hex::decode(base64_signature)
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let message = std::str::from_utf8(request.body)
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(message.to_string().into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif: CryptopayWebhookDetails =
            request
                .body
                .parse_struct("CryptopayWebhookDetails")
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        match notif.data.custom_id {
            Some(custom_id) => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::PaymentAttemptId(custom_id),
            )),
            None => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(notif.data.id),
            )),
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let notif: CryptopayWebhookDetails =
            request
                .body
                .parse_struct("CryptopayWebhookDetails")
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        match notif.data.status {
            cryptopay::CryptopayPaymentStatus::Completed => {
                Ok(api::IncomingWebhookEvent::PaymentIntentSuccess)
            }
            cryptopay::CryptopayPaymentStatus::Unresolved => {
                Ok(api::IncomingWebhookEvent::PaymentActionRequired)
            }
            cryptopay::CryptopayPaymentStatus::Cancelled => {
                Ok(api::IncomingWebhookEvent::PaymentIntentFailure)
            }
            _ => Ok(api::IncomingWebhookEvent::EventNotSupported),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let notif: CryptopayWebhookDetails =
            request
                .body
                .parse_struct("CryptopayWebhookDetails")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(Box::new(notif))
    }
}
