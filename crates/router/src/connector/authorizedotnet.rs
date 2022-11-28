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

#[allow(dead_code)]
type PCapture = dyn services::ConnectorIntegration<
    api::PCapture,
    types::PaymentsRequestCaptureData,
    types::PaymentsResponseData,
>;
impl
    services::ConnectorIntegration<
        api::PCapture,
        types::PaymentsRequestCaptureData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    // Not Implemented (R)
}

type PSync = dyn services::ConnectorIntegration<
    api::PSync,
    types::PaymentsRequestSyncData,
    types::PaymentsResponseData,
>;
impl
    services::ConnectorIntegration<
        api::PSync,
        types::PaymentsRequestSyncData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::PaymentsRouterSyncData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                PSync::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &types::PaymentsRouterSyncData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsRouterSyncData,
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
        req: &types::PaymentsRouterSyncData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&PSync::get_url(self, req, connectors)?)
            .headers(PSync::get_headers(self, req)?)
            .body(PSync::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsRouterSyncData,
        res: Response,
    ) -> CustomResult<types::PaymentsRouterSyncData, errors::ConnectorError> {
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
    > for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::PaymentsRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                Authorize::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsRouterData,
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
    > for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::PaymentRouterCancelData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                Authorize::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentRouterCancelData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentRouterCancelData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let authorizedotnet_req =
            utils::Encode::<authorizedotnet::CancelTransactionRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
    }
    fn build_request(
        &self,
        req: &types::PaymentRouterCancelData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                // TODO: [ORCA-346] Requestbuilder needs &str migrate get_url to send &str instead of owned string
                .url(&Void::get_url(self, req, connectors)?)
                .headers(Void::get_headers(self, req)?)
                .header(headers::X_ROUTER, "test")
                .body(Void::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentRouterCancelData,
        res: Response,
    ) -> CustomResult<types::PaymentRouterCancelData, errors::ConnectorError> {
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
    > for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                Authorize::get_content_type(self).to_string(),
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
            .url(&Execute::get_url(self, req, connectors)?)
            .headers(Execute::get_headers(self, req)?)
            .body(Execute::get_request_body(self, req)?)
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

type RSync = dyn services::ConnectorIntegration<
    api::RSync,
    types::RefundsRequestData,
    types::RefundsResponseData,
>;

impl
    services::ConnectorIntegration<
        api::RSync,
        types::RefundsRequestData,
        types::RefundsResponseData,
    > for Authorizedotnet
{
    fn get_headers(
        &self,
        _req: &types::RefundsRouterData<api::RSync>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                RSync::get_content_type(self).to_string(),
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
            .url(&RSync::get_url(self, req, connectors)?)
            .headers(RSync::get_headers(self, req)?)
            .body(RSync::get_request_body(self, req)?)
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
