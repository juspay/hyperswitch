mod transformers;

use std::fmt::Debug;

use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use transformers as intuit;
use uuid::Uuid;

use crate::{
    configs::settings,
    connector::utils::RefundsRequestData,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers,
    services::{self, request::ContentType, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Intuit;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Intuit
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let access_token = req
            .access_token
            .clone()
            .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;

        let auth_header = (
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", access_token.token),
        );

        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string(),
            ),
            (
                headers::ACCEPT.to_string(),
                self.get_content_type().to_string(),
            ),
            //Unique ID used by the connector to differentiate among the incoming operations which might look similar
            ("request-id".to_string(), Uuid::new_v4().to_string()),
        ];
        headers.push(auth_header);
        Ok(headers)
    }
}

impl ConnectorCommon for Intuit {
    fn id(&self) -> &'static str {
        "intuit"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.intuit.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: intuit::IntuitErrorResponse = res
            .response
            .parse_struct("Intuit ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        if let Some(error_resp) = response.errors.get(0) {
            return Ok(ErrorResponse {
                status_code: res.status_code,
                code: error_resp.code.clone(),
                message: error_resp.message.clone(),
                reason: None,
            });
        }

        Ok(ErrorResponse::default())
    }
}

impl api::Payment for Intuit {}

impl api::PreVerify for Intuit {}
impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Intuit
{
}

impl api::PaymentVoid for Intuit {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Intuit
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}/quickbooks/v4/payments/txn-requests/{}/void",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
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
        let response: intuit::IntuitVoidResponse = res
            .response
            .parse_struct("intuit VoidResponse")
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

impl api::ConnectorAccessToken for Intuit {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Intuit
{
    fn get_url(
        &self,
        _req: &types::RefreshTokenRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok("https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer".to_string())
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_headers(
        &self,
        req: &types::RefreshTokenRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth_type = intuit::IntuitAuthType::try_from(&req.connector_auth_type)?;
        let encoded_api_key = consts::BASE64_ENGINE.encode(format!(
            "{}:{}",
            auth_type.client_id, auth_type.client_secret
        ));

        let headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefreshTokenType::get_content_type(self).to_string(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Basic {}", encoded_api_key),
            ),
        ];
        Ok(headers)
    }

    fn get_request_body(
        &self,
        req: &types::RefreshTokenRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = intuit::IntuitAuthUpdateRequest::try_from(req)?;
        let intuit_req =
            utils::Encode::<intuit::IntuitAuthUpdateRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(intuit_req))
    }

    fn build_request(
        &self,
        req: &types::RefreshTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let req = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .headers(types::RefreshTokenType::get_headers(self, req, connectors)?)
                .url(&types::RefreshTokenType::get_url(self, req, connectors)?)
                .body(types::RefreshTokenType::get_request_body(self, req)?)
                .content_type(ContentType::FormUrlEncoded)
                .build(),
        );
        Ok(req)
    }
    fn handle_response(
        &self,
        data: &types::RefreshTokenRouterData,
        res: Response,
    ) -> CustomResult<types::RefreshTokenRouterData, errors::ConnectorError> {
        let response: intuit::IntuitAuthUpdateResponse = res
            .response
            .parse_struct("airwallex AirwallexAuthUpdateResponse")
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
        let response: intuit::IntuitAuthErrorResponse = res
            .response
            .parse_struct("Intuit IntuitAuthErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            message: response.error,
            ..Default::default()
        })
    }
}

impl api::PaymentSync for Intuit {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Intuit
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/quickbooks/v4/payments/charges/{}",
            self.base_url(connectors),
            connector_payment_id
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
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: intuit::IntuitPaymentsResponse = res
            .response
            .parse_struct("intuit PaymentsResponse")
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

impl api::PaymentCapture for Intuit {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Intuit
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
            "{}/quickbooks/v4/payments/charges/{}/capture",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = intuit::IntuitPaymentsCaptureRequest::try_from(req)?;
        let req = utils::Encode::<intuit::IntuitPaymentsCaptureRequest>::encode_to_string_of_json(
            &req_obj,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
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
                .body(self.get_request_body(req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: intuit::IntuitPaymentsResponse = res
            .response
            .parse_struct("Intuit PaymentsResponse")
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

impl api::PaymentSession for Intuit {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Intuit
{
    //TODO: implement sessions flow
}

impl api::PaymentAuthorize for Intuit {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Intuit
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
            "{}/quickbooks/v4/payments/charges",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = intuit::IntuitPaymentsRequest::try_from(req)?;
        let req =
            utils::Encode::<intuit::IntuitPaymentsRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
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
                .body(self.get_request_body(req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: intuit::IntuitPaymentsResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
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

impl api::Refund for Intuit {}
impl api::RefundExecute for Intuit {}
impl api::RefundSync for Intuit {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Intuit {
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
            "{}/quickbooks/v4/payments/charges/{}/refunds",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = intuit::IntuitRefundRequest::try_from(req)?;
        let req =
            utils::Encode::<intuit::IntuitPaymentsRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
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
            .body(self.get_request_body(req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: intuit::RefundResponse =
            res.response
                .parse_struct("intuit RefundResponse")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
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

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Intuit {
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
        let connector_payment_id = req.request.connector_transaction_id.clone();
        let connector_refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}/quickbooks/v4/payments/charges/{}/refunds/{}",
            self.base_url(connectors),
            connector_payment_id,
            connector_refund_id
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
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: intuit::RefundResponse =
            res.response
                .parse_struct("intuit RefundResponse")
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

#[async_trait::async_trait]
impl api::IncomingWebhook for Intuit {
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
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}

impl services::ConnectorRedirectResponse for Intuit {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        _action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}
