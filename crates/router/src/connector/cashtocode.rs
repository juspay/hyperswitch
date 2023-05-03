mod transformers;

use std::fmt::Debug;

use common_utils::errors::ReportSwitchExt;
use error_stack::{IntoReport, ResultExt};
use router_env::logger;
use transformers as cashtocode;

use crate::{
    configs::settings,
    connector::utils as conn_utils,
    core::errors::{self, CustomResult},
    db::StorageInterface,
    headers,
    services::{self, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, ByteSliceExt},
};

#[derive(Debug, Clone)]
pub struct Cashtocode;

impl api::Payment for Cashtocode {}
impl api::PaymentSession for Cashtocode {}
impl api::ConnectorAccessToken for Cashtocode {}
impl api::PreVerify for Cashtocode {}
impl api::PaymentAuthorize for Cashtocode {}
impl api::PaymentSync for Cashtocode {}
impl api::PaymentToken for Cashtocode {}
impl api::PaymentCapture for Cashtocode {}
impl api::PaymentVoid for Cashtocode {}
impl api::Refund for Cashtocode {}
impl api::RefundExecute for Cashtocode {}
impl api::RefundSync for Cashtocode {}

fn get_auth_cashtocode(
    payment_method_data: &api::payments::PaymentMethodData,
    auth_type: &types::ConnectorAuthType,
) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
    match payment_method_data {
        api_models::payments::PaymentMethodData::Reward(reward_data) => {
            if reward_data.reward_type == "classic" {
                match auth_type {
                    types::ConnectorAuthType::BodyKey { api_key, key1: _ } => Ok(vec![(
                        headers::AUTHORIZATION.to_string(),
                        format!("Basic {}", api_key.to_owned()),
                    )]),
                    _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
                }
            } else {
                match auth_type {
                    types::ConnectorAuthType::BodyKey { api_key: _, key1 } => Ok(vec![(
                        headers::AUTHORIZATION.to_string(),
                        format!("Basic {}", key1.to_owned()),
                    )]),
                    _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
                }
            }
        }
        _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Cashtocode
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self).to_string(),
        )];
        let default_key = match &req.connector_auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1: _ } => api_key.to_owned(),
            _ => return Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        };
        let mut api_key = vec![(headers::AUTHORIZATION.to_string(), default_key)];
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Cashtocode {
    fn id(&self) -> &'static str {
        "cashtocode"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.cashtocode.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth = cashtocode::CashtocodeAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key)])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: cashtocode::CashtocodeErrorResponse = res
            .response
            .parse_struct("CashtocodeErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.to_string(),
            message: response.error_description,
            reason: None,
        })
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Cashtocode
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Cashtocode
{
}

impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Cashtocode
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Cashtocode
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self).to_string(),
        )];
        let auth_differentiator =
            get_auth_cashtocode(&req.request.payment_method_data, &req.connector_auth_type);
        let mut api_key = match auth_differentiator {
            Ok(auth_type) => auth_type,
            Err(err) => return Err(err),
        };
        header.append(&mut api_key);
        Ok(header)
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
            "{}{}",
            connectors.cashtocode.base_url, "merchant/paytokens"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = cashtocode::CashtocodePaymentsRequest::try_from(req)?;
        let cashtocode_req =
            utils::Encode::<cashtocode::CashtocodePaymentsRequest>::encode_to_string_of_json(
                &req_obj,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        logger::info!(cashtocode_req);
        Ok(Some(cashtocode_req))
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
        let response: cashtocode::CashtocodePaymentsResponse = res
            .response
            .parse_struct("Cashtocode PaymentsAuthorizeResponse")
            .switch()?;
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

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Cashtocode
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
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        types::RouterData::try_from(types::ResponseRouterData {
            response: cashtocode::CashtocodePaymentsSyncResponse {},
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

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Cashtocode
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Cashtocode
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Cashtocode
{
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Cashtocode
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Cashtocode {
    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let base64_signature = conn_utils::get_header_key_value("authorization", request.headers)?;
        logger::info!(base64_signature);
        let signature = base64_signature.as_bytes().to_owned();
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
        let secret_auth = String::from_utf8(secret.to_vec())
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        let signature_auth = String::from_utf8(signature.to_vec())
            .into_report()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        let mut success = false;
        if signature_auth == secret_auth {
            success = true;
        }
        Ok(success)
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook: transformers::CashtocodeIncomingWebhook = request
            .body
            .parse_struct("CashtocodeIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(webhook.transaction_id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Ok(api::IncomingWebhookEvent::PaymentIntentSuccess)
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let webhook: transformers::CashtocodeIncomingWebhook = request
            .body
            .parse_struct("CashtocodeIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let response = webhook.transaction_id;
        let res_json =
            utils::Encode::<transformers::CashtocodeIncomingWebhook>::encode_to_value(&response)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(res_json)
    }

    fn get_webhook_api_response(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ConnectorError>
    {
        let status = "EXECUTED".to_string();
        let id = self
            .get_webhook_object_reference_id(request)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let txn_id = match id {
            api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(connector_txn_id),
            ) => connector_txn_id,
            _ => return Err(errors::ConnectorError::MissingConnectorTransactionID).into_report(),
        };
        let response: serde_json::Value =
            serde_json::json!({ "status": status, "transactionId" : txn_id});
        Ok(services::api::ApplicationResponse::Json(response))
    }
}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Cashtocode
{
    // Not Implemented (R)
}
