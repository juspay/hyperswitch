mod transformers;

use std::fmt::Debug;

use base64;
use bytes::Bytes;
use common_utils::generate_id;
use error_stack::ResultExt;
use ring::hmac;
use time::OffsetDateTime;

use crate::{
    configs::settings,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger, services,
    types::{self, api, ErrorResponse, Response},
    utils::{self, BytesExt},
};

use transformers as rapyd;

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
        let signature_value = base64::encode(hmac::sign(&key, to_sign.as_bytes()).as_ref());
        Ok(signature_value)
    }
}

impl api::ConnectorCommon for Rapyd {
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
        todo!()
    }
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
        Ok(vec![])
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v1/payments/",
            api::ConnectorCommon::base_url(self, connectors)
        ))
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
        let timestamp = OffsetDateTime::unix_timestamp(OffsetDateTime::now_utc());
        let salt = generate_id(12, "");

        let rapyd_req = utils::Encode::<rapyd::RapydPaymentsRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let signature =
            self.generate_signature(&auth, "post", "/v1/payments", &rapyd_req, &timestamp, &salt)?;
        let headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
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
            .headers(headers)
            .body(Some(rapyd_req))
            .build();
        print!("myrequest {:?}", request);
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
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(rapydpayments_create_response=?response);
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .parse_struct("Rapyd ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response.status.error_code,
            message: response.status.status,
            reason: response.status.message,
        })
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
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn build_request(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        todo!()
    }

    fn get_error_response(
        &self,
        _res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        todo!()
    }

    fn handle_response(
        &self,
        _data: &types::PaymentsSyncRouterData,
        _res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        todo!()
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
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        todo!()
    }

    fn handle_response(
        &self,
        _data: &types::PaymentsCaptureRouterData,
        _res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        todo!()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn get_error_response(
        &self,
        _res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        todo!()
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
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        api::ConnectorCommon::common_get_content_type(self)
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v1/refunds",
            api::ConnectorCommon::base_url(self, connectors)
        ))
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
        let timestamp = OffsetDateTime::unix_timestamp(OffsetDateTime::now_utc());
        let salt = generate_id(12, "");

        let rapyd_req = utils::Encode::<rapyd::RapydRefundRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let signature =
            self.generate_signature(&auth, "post", "/v1/refunds", &rapyd_req, &timestamp, &salt)?;
        let headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
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
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        logger::debug!(target: "router::connector::rapyd", response=?res);
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
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .parse_struct("Rapyd ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response.status.error_code,
            message: response.status.status,
            reason: response.status.message,
        })
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
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        logger::debug!(target: "router::connector::rapyd", response=?res);
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
        _res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Rapyd {
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

impl services::ConnectorRedirectResponse for Rapyd {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}
