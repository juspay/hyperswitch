mod transformers;

use std::fmt::Debug;

use bytes::Bytes;
use common_utils::ext_traits::ByteSliceExt;
use error_stack::{IntoReport, ResultExt};
use transformers as shift4;

use crate::{
    configs::settings::Connectors,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger, services,
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Shift4;

impl api::ConnectorCommon for Shift4 {
    fn id(&self) -> &'static str {
        "shift4"
    }

    fn common_get_content_type(&self) -> &'static str {
        todo!()
        // Ex: "application/x-www-form-urlencoded"
    }

    fn base_url(&self, connectors: Connectors) -> String {
        connectors.shift4.base_url
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth: shift4::Shift4AuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key)])
    }
}

impl api::Payment for Shift4 {}

impl api::PreVerify for Shift4 {}
impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Shift4
{
}

impl api::PaymentVoid for Shift4 {}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Shift4
{
}

impl api::PaymentSync for Shift4 {}
impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Shift4
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsSyncType::get_content_type(self).to_string(),
            ),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/charges/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .headers(types::PaymentsSyncType::get_headers(self, req)?)
                .body(types::PaymentsSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: shift4::ErrorResponse = res
            .parse_struct("Shift4 ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            code: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response.error.message,
            reason: None,
        })
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        logger::debug!(payment_sync_response=?res);
        let response: shift4::Shift4PaymentsResponse = res
            .response
            .parse_struct("shift4 PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::PaymentCapture for Shift4 {}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Shift4
{
    fn get_headers(
        &self,
        req: &types::RouterData<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsSyncType::get_content_type(self).to_string(),
            ),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_request_body(
        &self,
        _req: &types::RouterData<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .headers(types::PaymentsCaptureType::get_headers(self, req)?)
                .body(types::PaymentsCaptureType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RouterData<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
        res: Response,
    ) -> CustomResult<
        types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>,
        errors::ConnectorError,
    >
    where
        api::Capture: Clone,
        types::PaymentsCaptureData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: shift4::Shift4PaymentsResponse = res
            .response
            .parse_struct("Shift4PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(shift4payments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_url(
        &self,
        req: &types::RouterData<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}charges/{}/capture",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: shift4::ErrorResponse = res
            .parse_struct("Shift4 ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            code: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response.error.message,
            reason: None,
        })
    }
}

impl api::PaymentSession for Shift4 {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Shift4
{
    //TODO: implement sessions flow
}

impl api::PaymentAuthorize for Shift4 {}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Shift4
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self).to_string(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}charges", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let shift4_req = utils::Encode::<shift4::Shift4PaymentsRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(shift4_req))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .headers(types::PaymentsAuthorizeType::get_headers(self, req)?)
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: shift4::Shift4PaymentsResponse = res
            .response
            .parse_struct("Shift4PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(shift4payments_create_response=?response);
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
        logger::debug!(shift4payments_create_response=?res);

        let response: shift4::ErrorResponse = res
            .parse_struct("Shift4 ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response.error.message,
            reason: None,
        })
    }
}

impl api::Refund for Shift4 {}
impl api::RefundExecute for Shift4 {}
impl api::RefundSync for Shift4 {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Shift4
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsSyncType::get_content_type(self).to_string(),
            ),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/refunds", self.base_url(connectors),))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let shift4_req = utils::Encode::<shift4::Shift4RefundRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(shift4_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
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
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        logger::debug!(target: "router::connector::shift4", response=?res);
        let response: shift4::RefundResponse = res
            .response
            .parse_struct("RefundResponse")
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
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: shift4::ErrorResponse = res
            .parse_struct("Shift4 ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            code: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response.error.message,
            reason: None,
        })
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Shift4
{
    fn get_headers(
        &self,
        req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsSyncType::get_content_type(self).to_string(),
            ),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/refunds", self.base_url(connectors),))
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        res: Response,
    ) -> CustomResult<
        types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        errors::ConnectorError,
    > {
        logger::debug!(target: "router::connector::shift4", response=?res);
        let response: shift4::RefundResponse =
            res.response
                .parse_struct("shift4 RefundResponse")
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
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: shift4::ErrorResponse = res
            .parse_struct("Shift4 ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            code: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response.error.message,
            reason: None,
        })
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Shift4 {
    fn get_webhook_object_reference_id(
        &self,
        body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError> {
        let details: shift4::Shift4WebhookObjectId = body
            .parse_struct("Shift4WebhookObjectId")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        Ok(details.data.id)
    }

    fn get_webhook_event_type(
        &self,
        body: &[u8],
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let details: shift4::Shift4WebhookObjectEventType = body
            .parse_struct("Shift4WebhookObjectEventType")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(match details.event_type.as_str() {
            "CHARGE_SUCCEEDED" => api::IncomingWebhookEvent::PaymentIntentSuccess,
            _ => Err(errors::ConnectorError::WebhookEventTypeNotFound).into_report()?,
        })
    }

    fn get_webhook_resource_object(
        &self,
        body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let details: shift4::Shift4WebhookObjectResource = body
            .parse_struct("Shift4WebhookObjectResource")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(details.data)
    }
}

impl services::ConnectorRedirectResponse for Shift4 {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}
