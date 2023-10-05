pub mod transformers;

use std::fmt::Debug;

use common_utils::crypto::{self, GenerateDigest};
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use hex::encode;
use masking::ExposeInterface;
use rand::distributions::DistString;
use time::OffsetDateTime;
use transformers as globepay;

use crate::{
    configs::settings,
    consts,
    core::errors::{self, CustomResult},
    headers,
    services::{self, request, ConnectorIntegration, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Globepay;

impl api::Payment for Globepay {}
impl api::PaymentSession for Globepay {}
impl api::ConnectorAccessToken for Globepay {}
impl api::MandateSetup for Globepay {}
impl api::PaymentAuthorize for Globepay {}
impl api::PaymentSync for Globepay {}
impl api::PaymentCapture for Globepay {}
impl api::PaymentVoid for Globepay {}
impl api::Refund for Globepay {}
impl api::RefundExecute for Globepay {}
impl api::RefundSync for Globepay {}
impl api::PaymentToken for Globepay {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Globepay
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Globepay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        Ok(header)
    }
}

fn get_globlepay_query_params(
    connector_auth_type: &types::ConnectorAuthType,
) -> CustomResult<String, errors::ConnectorError> {
    let auth_type = globepay::GlobepayAuthType::try_from(connector_auth_type)?;
    let time = (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000).to_string();
    let nonce_str = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 12);
    let valid_string = format!(
        "{}&{time}&{nonce_str}&{}",
        auth_type.partner_code.expose(),
        auth_type.credential_code.expose()
    );
    let digest = crypto::Sha256
        .generate_digest(valid_string.as_bytes())
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("error encoding the query params")?;
    let sign = encode(digest).to_lowercase();
    let param = format!("?sign={sign}&time={time}&nonce_str={nonce_str}");
    Ok(param)
}

fn get_partner_code(
    connector_auth_type: &types::ConnectorAuthType,
) -> CustomResult<String, errors::ConnectorError> {
    let auth_type = globepay::GlobepayAuthType::try_from(connector_auth_type)?;
    Ok(auth_type.partner_code.expose())
}

impl ConnectorCommon for Globepay {
    fn id(&self) -> &'static str {
        "globepay"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.globepay.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: globepay::GlobepayErrorResponse = res
            .response
            .parse_struct("GlobepayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.return_code.to_string(),
            message: consts::NO_ERROR_MESSAGE.to_string(),
            reason: Some(response.return_msg),
        })
    }
}

impl ConnectorValidation for Globepay {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Globepay
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Globepay
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Globepay
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Globepay
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
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let query_params = get_globlepay_query_params(&req.connector_auth_type)?;
        if req.request.capture_method == Some(enums::CaptureMethod::Automatic) {
            Ok(format!(
                "{}api/v1.0/gateway/partners/{}/orders/{}{query_params}",
                self.base_url(connectors),
                get_partner_code(&req.connector_auth_type)?,
                req.payment_id
            ))
        } else {
            Err(errors::ConnectorError::FlowNotSupported {
                flow: "Manual Capture".to_owned(),
                connector: "Globepay".to_owned(),
            }
            .into())
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = globepay::GlobepayPaymentsRequest::try_from(req)?;
        let globepay_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<globepay::GlobepayPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(globepay_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Put)
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
        let response: globepay::GlobepayPaymentsResponse = res
            .response
            .parse_struct("Globepay PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    for Globepay
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
        let query_params = get_globlepay_query_params(&req.connector_auth_type)?;
        Ok(format!(
            "{}api/v1.0/gateway/partners/{}/orders/{}{query_params}",
            self.base_url(connectors),
            get_partner_code(&req.connector_auth_type)?,
            req.payment_id
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
        let response: globepay::GlobepaySyncResponse = res
            .response
            .parse_struct("globepay PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    for Globepay
{
    fn build_request(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Manual Capture".to_owned(),
            connector: "Globepay".to_owned(),
        }
        .into())
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Globepay
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Globepay
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
        let query_params = get_globlepay_query_params(&req.connector_auth_type)?;
        Ok(format!(
            "{}api/v1.0/gateway/partners/{}/orders/{}/refunds/{}{query_params}",
            self.base_url(connectors),
            get_partner_code(&req.connector_auth_type)?,
            req.payment_id,
            req.request.refund_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = globepay::GlobepayRefundRequest::try_from(req)?;
        let globepay_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<globepay::GlobepayRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(globepay_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Put)
                .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundExecuteType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::RefundExecuteType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: globepay::GlobepayRefundResponse = res
            .response
            .parse_struct("Globalpay RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Globepay {
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

    fn get_url(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let query_params = get_globlepay_query_params(&req.connector_auth_type)?;
        Ok(format!(
            "{}api/v1.0/gateway/partners/{}/orders/{}/refunds/{}{query_params}",
            self.base_url(connectors),
            get_partner_code(&req.connector_auth_type)?,
            req.payment_id,
            req.request.refund_id
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
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: globepay::GlobepayRefundResponse = res
            .response
            .parse_struct("Globalpay RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

#[async_trait::async_trait]
impl api::IncomingWebhook for Globepay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
