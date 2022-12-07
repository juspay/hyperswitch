#![allow(dead_code)]
mod transformers;

use std::fmt::Debug;

use bytes::Bytes;
use error_stack::{IntoReport, ResultExt};
use transformers as authorizedotnet;

use crate::{
    configs::settings::Connectors,
    consts,
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
pub struct Authorizedotnet;

impl api::ConnectorCommon for Authorizedotnet {
    fn id(&self) -> &'static str {
        "authorizedotnet"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url(&self, connectors: Connectors) -> String {
        connectors.authorizedotnet.base_url
    }
}

impl api::Payment for Authorizedotnet {}
impl api::PaymentAuthorize for Authorizedotnet {}
impl api::PaymentSync for Authorizedotnet {}
impl api::PaymentVoid for Authorizedotnet {}
impl api::PaymentCapture for Authorizedotnet {}
impl api::PreVerify for Authorizedotnet {}

impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    // TODO: Critical Implement
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsSyncType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let sync_request =
            utils::Encode::<authorizedotnet::AuthorizedotnetCreateSyncRequest>::convert_and_encode(
                req,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(sync_request))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
            .headers(types::PaymentsSyncType::get_headers(self, req)?)
            .body(types::PaymentsSyncType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetSyncResponse = intermediate_response
            .parse_struct("AuthorizedotnetSyncResponse")
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
        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = res
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let error = response
            .transaction_response
            .errors
            .and_then(|errors| {
                errors.first().map(|error| types::ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_text.clone(),
                    reason: None,
                })
            })
            .unwrap_or_else(|| types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: consts::NO_ERROR_MESSAGE.to_string(),
                reason: None,
            });

        Ok(error)
    }
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        logger::debug!(request=?req);
        let authorizedotnet_req =
            utils::Encode::<authorizedotnet::CreateTransactionRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
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
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = intermediate_response
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(authorizedotnetpayments_create_response=?response);

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
        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = res
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let error = response
            .transaction_response
            .errors
            .and_then(|errors| {
                errors.first().map(|error| types::ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_text.clone(),
                    reason: None,
                })
            })
            .unwrap_or_else(|| types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: consts::NO_ERROR_MESSAGE.to_string(),
                reason: None,
            });

        Ok(error)
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCancelRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let authorizedotnet_req =
            utils::Encode::<authorizedotnet::CancelTransactionRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
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
                .header(headers::X_ROUTER, "test")
                .body(types::PaymentsVoidType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = intermediate_response
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(authorizedotnetpayments_create_response=?response);

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
        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = res
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let error = response
            .transaction_response
            .errors
            .and_then(|errors| {
                errors.first().map(|error| types::ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_text.clone(),
                    reason: None,
                })
            })
            .unwrap_or_else(|| types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: consts::NO_ERROR_MESSAGE.to_string(),
                reason: None,
            });

        Ok(error)
    }
}

impl api::Refund for Authorizedotnet {}
impl api::RefundExecute for Authorizedotnet {}
impl api::RefundSync for Authorizedotnet {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        logger::debug!(refund_request=?req);
        let authorizedotnet_req =
            utils::Encode::<authorizedotnet::CreateRefundRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
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
        use bytes::Buf;
        logger::debug!(response=?res);

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetRefundResponse = intermediate_response
            .parse_struct("AuthorizedotnetRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);

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
        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = res
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);

        let error = response
            .transaction_response
            .errors
            .and_then(|errors| {
                errors.first().map(|error| types::ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_text.clone(),
                    reason: None,
                })
            })
            .unwrap_or_else(|| types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: consts::NO_ERROR_MESSAGE.to_string(),
                reason: None,
            });

        Ok(error)
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::RefundsRouterData<api::RSync>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefundSyncType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::RSync>,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let sync_request =
            utils::Encode::<authorizedotnet::AuthorizedotnetCreateSyncRequest>::convert_and_encode(
                req,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(sync_request))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundSyncType::get_url(self, req, connectors)?)
            .headers(types::RefundSyncType::get_headers(self, req)?)
            .body(types::RefundSyncType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::RSync>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::RSync>, errors::ConnectorError> {
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetSyncResponse = intermediate_response
            .parse_struct("AuthorizedotnetSyncResponse")
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
        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = res
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let error = response
            .transaction_response
            .errors
            .and_then(|errors| {
                errors.first().map(|error| types::ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_text.clone(),
                    reason: None,
                })
            })
            .unwrap_or_else(|| types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: consts::NO_ERROR_MESSAGE.to_string(),
                reason: None,
            });

        Ok(error)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Authorizedotnet {
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

impl services::ConnectorRedirectResponse for Authorizedotnet {}
