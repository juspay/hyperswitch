#![allow(dead_code)]
mod result_codes;
mod transformers;
use std::fmt::Debug;

use bytes::Bytes;
use error_stack::{IntoReport, ResultExt};
use transformers as aci;

use crate::{
    configs::settings::Connectors,
    core::errors::{self, CustomResult},
    headers,
    services::{self, logger},
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Aci;

impl api::ConnectorCommon for Aci {
    fn id(&self) -> &'static str {
        "aci"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url(&self, connectors: Connectors) -> String {
        connectors.aci.base_url
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth: aci::AciAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key)])
    }
}

impl api::Payment for Aci {}

impl api::PaymentAuthorize for Aci {}
impl api::PaymentSync for Aci {}
impl api::PaymentVoid for Aci {}
impl api::PaymentCapture for Aci {}

impl
    services::ConnectorIntegration<
        api::PCapture,
        types::PaymentsRequestCaptureData,
        types::PaymentsResponseData,
    > for Aci
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::PSync,
        types::PaymentsRequestSyncData,
        types::PaymentsResponseData,
    > for Aci
{
    // Not Implemented (R)
}

type Authorize = dyn services::ConnectorIntegration<
    api::Authorize,
    types::PaymentsRequestData,
    types::PaymentsResponseData,
>;

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsRequestData,
        types::PaymentsResponseData,
    > for Aci
{
    fn get_headers(
        &self,
        req: &types::PaymentsRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                Authorize::get_content_type(self).to_string(),
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
        _req: &types::PaymentsRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "v1/payments"))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        // encode only for for urlencoded things.
        let aci_req = utils::Encode::<aci::AciPaymentsRequest>::convert_and_url_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        logger::debug!(aci_payment_logs=?aci_req);
        Ok(Some(aci_req))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::Authorize,
            types::PaymentsRequestData,
            types::PaymentsResponseData,
        >,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                // TODO: [ORCA-346] Requestbuilder needs &str migrate get_url to send &str instead of owned string
                .url(&Authorize::get_url(self, req, connectors)?)
                .headers(Authorize::get_headers(self, req)?)
                .header(headers::X_ROUTER, "test")
                .body(Authorize::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsRouterData, errors::ConnectorError> {
        let response: aci::AciPaymentsResponse =
            res.response
                .parse_struct("AciPaymentsResponse")
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
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: aci::AciPaymentsFailureResponse = res
            .parse_struct("AciPaymentsFailureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response.result.code,
            message: response.result.description,
            reason: response.result.parameter_errors.and_then(|errors| {
                errors.first().map(|error_description| {
                    format!(
                        "Field is {} and the message is {}",
                        error_description.name, error_description.message
                    )
                })
            }),
        })
    }
}

type Void = dyn services::ConnectorIntegration<
    api::Void,
    types::PaymentRequestCancelData,
    types::PaymentsResponseData,
>;

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentRequestCancelData,
        types::PaymentsResponseData,
    > for Aci
{
    fn get_headers(
        &self,
        _req: &types::PaymentRouterCancelData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("aci".to_string()).into())
    }

    fn get_content_type(&self) -> &'static str {
        ""
    }

    fn get_url(
        &self,
        _req: &types::PaymentRouterCancelData,
        _connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("aci".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentRouterCancelData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("aci".to_string()).into())
    }
    fn build_request(
        &self,
        _req: &types::PaymentRouterCancelData,
        _connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("aci".to_string()).into())
    }

    fn handle_response(
        &self,
        _data: &types::PaymentRouterCancelData,
        _res: Response,
    ) -> CustomResult<types::PaymentRouterCancelData, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("aci".to_string()).into())
    }

    fn get_error_response(
        &self,
        _res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("aci".to_string()).into())
    }
}

impl api::Refund for Aci {}
impl api::RefundExecute for Aci {}
impl api::RefundSync for Aci {}

type Execute = dyn services::ConnectorIntegration<
    api::Execute,
    types::RefundsRequestData,
    types::RefundsResponseData,
>;
impl
    services::ConnectorIntegration<
        api::Execute,
        types::RefundsRequestData,
        types::RefundsResponseData,
    > for Aci
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                Execute::get_content_type(self).to_string(),
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
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}v1/payments/{}",
            self.base_url(connectors),
            connector_payment_id,
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let body = utils::Encode::<aci::AciRefundRequest>::convert_and_url_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(body))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&Execute::get_url(self, req, connectors)?)
                .headers(Execute::get_headers(self, req)?)
                .body(Execute::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        logger::debug!(response=?res);

        let response: aci::AciRefundResponse = res
            .response
            .parse_struct("AciRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }
    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: aci::AciErrorRefundResponse = res
            .parse_struct("AciErrorRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response.result.code,
            message: response.result.description,
            reason: response.result.parameter_errors.and_then(|errors| {
                errors.first().map(|error_description| {
                    format!(
                        "Field is {} and the message is {}",
                        error_description.name, error_description.message
                    )
                })
            }),
        })
    }
}

impl
    services::ConnectorIntegration<
        api::RSync,
        types::RefundsRequestData,
        types::RefundsResponseData,
    > for Aci
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Aci {
    fn get_webhook_object_reference_id(
        &self,
        _body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(&self, _body: &[u8]) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}

impl services::ConnectorRedirectResponse for Aci {}
