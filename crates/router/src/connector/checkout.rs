#![allow(dead_code)]

mod transformers;

use std::fmt::Debug;

use bytes::Bytes;
use error_stack::{IntoReport, ResultExt};

use self::transformers as checkout;
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
    utils::{self, BytesExt, OptionExt},
};

#[derive(Debug, Clone)]
pub struct Checkout;

impl api::ConnectorCommon for Checkout {
    fn id(&self) -> &'static str {
        "checkout"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth: checkout::CheckoutAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key)])
    }

    fn base_url(&self, connectors: Connectors) -> String {
        connectors.checkout.base_url
    }
}

impl api::Payment for Checkout {}

impl api::PaymentAuthorize for Checkout {}
impl api::PaymentSync for Checkout {}
impl api::PaymentVoid for Checkout {}
impl api::PaymentCapture for Checkout {}
impl api::PreVerify for Checkout {}

impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Checkout
{
    // TODO: Critical Implement
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Checkout
{
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}{}",
            self.base_url(connectors),
            "payments/",
            req.request.connector_transaction_id
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
                .header(headers::X_ROUTER, "test")
                .body(types::PaymentsSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError>
    where
        api::PSync: Clone,
        types::PaymentsSyncData: Clone,
        types::PaymentsResponseData: Clone,
    {
        logger::debug!(raw_response=?res);
        let response: checkout::PaymentsResponse = res
            .response
            .parse_struct("PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(payment_sync_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: checkout::ErrorResponse = res
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response
                .error_codes
                .unwrap_or_else(|| vec![consts::NO_ERROR_CODE.to_string()])
                .join(" &"),
            message: response
                .error_type
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
        })
    }
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Checkout
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "payments"))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let checkout_req = utils::Encode::<checkout::PaymentsRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(checkout_req))
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
                .header(headers::X_ROUTER, "test")
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        //TODO: [ORCA-618] If 3ds fails, the response should be a redirect response, to redirect client to success/failed page
        let response: checkout::PaymentsResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(payments_create_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: checkout::ErrorResponse = res
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response
                .error_codes
                .unwrap_or_else(|| vec![consts::NO_ERROR_CODE.to_string()])
                //Considered all the codes here but have to look into the exact no.of codes
                .join(" & "),
            message: response
                .error_type
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
        })
        //TODO : No sufficient information of error codes (no.of error codes to consider)
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Checkout
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsVoidType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}payments/{}/voids",
            self.base_url(connectors),
            &req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let checkout_req = utils::Encode::<checkout::PaymentVoidRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(checkout_req))
    }
    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .headers(types::PaymentsVoidType::get_headers(self, req)?)
                .body(types::PaymentsVoidType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let mut response: checkout::PaymentVoidResponse = res
            .response
            .parse_struct("PaymentVoidResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        response.status = res.status_code;
        logger::debug!(payments_create_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: checkout::ErrorResponse = res
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response
                .error_codes
                .unwrap_or_else(|| vec![consts::NO_ERROR_CODE.to_string()])
                //Considered all the codes here but have to look into the exact no.of codes
                .join(" & "),
            message: response
                .error_type
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
        })
    }
}

impl api::Refund for Checkout {}
impl api::RefundExecute for Checkout {}
impl api::RefundSync for Checkout {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefundExecuteType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}payments/{}/refunds",
            self.base_url(connectors),
            id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let body = utils::Encode::<checkout::RefundRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(body))
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
        logger::debug!(response=?res);
        let response: checkout::RefundResponse = res
            .response
            .parse_struct("checkout::RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let response = checkout::CheckoutRefundResponse {
            response,
            status: res.status_code,
        };
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
        let response: checkout::ErrorResponse = res
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response
                .error_codes
                .unwrap_or_else(|| vec![consts::NO_ERROR_CODE.to_string()])
                //Considered all the codes here but have to look into the exact no.of codes
                .join(" & "),
            message: response
                .error_type
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
        })
        //TODO : No sufficient information of error codes (no.of error codes to consider)
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefundSyncType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}/payments/{}/actions",
            self.base_url(connectors),
            id
        ))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .headers(types::RefundSyncType::get_headers(self, req)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::RSync>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::RSync>, errors::ConnectorError> {
        let refund_action_id = data
            .response
            .clone()
            .ok()
            .get_required_value("response")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?
            .connector_refund_id;

        let response: Vec<checkout::ActionResponse> = res
            .response
            .parse_struct("checkout::CheckoutRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let response = response
            .iter()
            .find(|&x| x.action_id.clone() == refund_action_id)
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
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
        let response: checkout::ErrorResponse = res
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response
                .error_codes
                .unwrap_or_else(|| vec![consts::NO_ERROR_CODE.to_string()])
                //Considered all the codes here but have to look into the exact no.of codes
                .join(" & "),
            message: response
                .error_type
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
        })
        //TODO : No sufficient information of error codes (no.of error codes to consider)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Checkout {
    fn get_webhook_object_reference_id(
        &self,
        _body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _body: &[u8],
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}

impl services::ConnectorRedirectResponse for Checkout {
    fn get_flow_type(
        &self,
        query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        let query =
            serde_urlencoded::from_str::<transformers::CheckoutRedirectResponse>(query_params)
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(query
            .status
            .map(|checkout_status| {
                payments::CallConnectorAction::StatusUpdate(checkout_status.into())
            })
            .unwrap_or(payments::CallConnectorAction::Trigger))
    }
}
