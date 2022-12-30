mod transformers;

use std::{fmt::Debug, time::{SystemTime, UNIX_EPOCH}};

use bytes::Bytes;
use error_stack::{ResultExt, IntoReport};
use ring::hmac;
use uuid::Uuid;

use crate::{
    configs::settings,
    consts,
    utils::{self, BytesExt},
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger, services,
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
    }
};


use transformers as fiserv;

#[derive(Debug, Clone)]
pub struct Fiserv;

impl Fiserv {
    pub fn generate_authorization_signature(
        &self,
        auth: fiserv::FiservAuthType,
        request_id: &str,
        payload: &str,
        timestamp: String,
    ) -> CustomResult<String, errors::ConnectorError> {
        let fiserv::FiservAuthType {
            api_key,
            merchant_account,
            api_secret,
        } = auth;
        let raw_signature = format!("{}{}{}{}", api_key, request_id, timestamp, payload);

        let key = hmac::Key::new(hmac::HMAC_SHA256, &api_secret.as_bytes());
        let signature_value =
            base64::encode(hmac::sign(&key, raw_signature.as_bytes()).as_ref());
        Ok(signature_value)
    }
}

impl api::ConnectorCommon for Fiserv {
    fn id(&self) -> &'static str {
        "fiserv"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.fiserv.base_url.as_ref()
    }
}

impl api::Payment for Fiserv {}

impl api::PreVerify for Fiserv {}
impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Fiserv
{
}

impl api::PaymentVoid for Fiserv {}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Fiserv
{}

impl api::PaymentSync for Fiserv {}
impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Fiserv
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        todo!()
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        todo!()
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        todo!()
    }
}


impl api::PaymentCapture for Fiserv {}
impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Fiserv
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        todo!()
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        todo!()
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        todo!()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        todo!()
    }
}

impl api::PaymentSession for Fiserv {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Fiserv
{
    //TODO: implement sessions flow
}

impl api::PaymentAuthorize for Fiserv {}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Fiserv {
    fn get_headers(&self, req: &types::PaymentsAuthorizeRouterData) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        let timestamp =  SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?
            .as_millis().to_string();
        let auth: fiserv::FiservAuthType =
            fiserv::FiservAuthType::try_from(&req.connector_auth_type)?;

        let fiserv_req =
            utils::Encode::<fiserv::FiservPaymentsRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let client_request_id = Uuid::new_v4().to_string();
        let hmac = self
            .generate_authorization_signature(auth, &client_request_id, &fiserv_req, timestamp.clone())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let headers = vec![
            (
                "Auth-Token-Type".to_string(),
                "HMAC".to_string(),
            ),
            (
                "Timestamp".to_string(),
                timestamp,
            ),
            (
                "Client-Request-Id".to_string(),
                client_request_id,
            ),
            (
                "Authorization".to_string(),
                hmac,
            )

        ];
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(&self, _req: &types::PaymentsAuthorizeRouterData, connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        Ok(format!("{}ch/payments/v1/charges", connectors.fiserv.base_url))
    }

    fn get_request_body(&self, req: &types::PaymentsAuthorizeRouterData) -> CustomResult<Option<String>,errors::ConnectorError> {
        let fiserv_req =
            utils::Encode::<fiserv::FiservPaymentsRequest>::convert_and_encode(req).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(fiserv_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(self, req, connectors)?)
                .headers(types::PaymentsAuthorizeType::get_headers(self, req)?)
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        );

        logger::debug!(braintree_session_request=?request);
        Ok(request)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData,errors::ConnectorError> {
        let response: fiserv::FiservPaymentsResponse = res.response.parse_struct("PaymentIntentResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(fiservpayments_create_response=?response);
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
        let response: fiserv::ErrorResponse = res
            .parse_struct("Fiserv ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let message = response
            .details
            .iter()
            .map(|v| v.message.clone())
            .collect::<Vec<String>>()
            .join("");
        Ok(types::ErrorResponse {
            code: consts::NO_ERROR_CODE.to_string(),
            message,
            reason: None,
        })
    }
}

impl api::Refund for Fiserv {}
impl api::RefundExecute for Fiserv {}
impl api::RefundSync for Fiserv {}

impl
    services::ConnectorIntegration<
        api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > for Fiserv {
    fn get_headers(&self, _req: &types::RefundsRouterData<api::Execute>) -> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(&self, _req: &types::RefundsRouterData<api::Execute>, connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(&self, req: &types::RefundsRouterData<api::Execute>) -> CustomResult<Option<String>,errors::ConnectorError> {
        let fiserv_req = utils::Encode::<fiserv::FiservRefundRequest>::convert_and_url_encode(req).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(fiserv_req))
    }

    fn build_request(&self, req: &types::RefundsRouterData<api::Execute>, connectors: &settings::Connectors,) -> CustomResult<Option<services::Request>,errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .headers(types::RefundExecuteType::get_headers(self, req)?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>,errors::ConnectorError> {
        logger::debug!(target: "router::connector::fiserv", response=?res);
        let response: fiserv::RefundResponse = res.response.parse_struct("fiserv RefundResponse").change_context(errors::ConnectorError::RequestEncodingFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, _res: Bytes) -> CustomResult<types::ErrorResponse,errors::ConnectorError> {
        todo!()
    }
}

impl
    services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Fiserv {
    fn get_headers(&self, _req: &types::RefundSyncRouterData) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(&self, _req: &types::RefundSyncRouterData,_connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::RefundSyncRouterData,errors::ConnectorError,> {
        logger::debug!(target: "router::connector::fiserv", response=?res);
        let response: fiserv::RefundResponse = res.response.parse_struct("fiserv RefundResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, _res: Bytes) -> CustomResult<types::ErrorResponse,errors::ConnectorError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Fiserv {
    fn get_webhook_object_reference_id(
        &self,
        _body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn get_webhook_event_type(
        &self,
        _body: &[u8],
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        todo!()
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        todo!()
    }
}

impl services::ConnectorRedirectResponse for Fiserv {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}
