pub mod transformers;

use std::fmt::Debug;

use common_utils::{crypto, ext_traits::ByteSliceExt};
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use transformers as authorizedotnet;

use crate::{
    configs::settings,
    connector::utils as connector_utils,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers,
    services::{self, request, ConnectorIntegration, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt, PaymentsCompleteAuthorize},
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Authorizedotnet;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Authorizedotnet
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, services::request::Maskable<String>)>, errors::ConnectorError>
    {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )])
    }
}

impl ConnectorCommon for Authorizedotnet {
    fn id(&self) -> &'static str {
        "authorizedotnet"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.authorizedotnet.base_url.as_ref()
    }
}

impl ConnectorValidation for Authorizedotnet {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_supported_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl api::Payment for Authorizedotnet {}
impl api::PaymentAuthorize for Authorizedotnet {}
impl api::PaymentSync for Authorizedotnet {}
impl api::PaymentVoid for Authorizedotnet {}
impl api::PaymentCapture for Authorizedotnet {}
impl api::PaymentSession for Authorizedotnet {}
impl api::ConnectorAccessToken for Authorizedotnet {}
impl api::PaymentToken for Authorizedotnet {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    // Not Implemented (R)
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Authorizedotnet
{
    // Not Implemented (R)
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Authorizedotnet
{
    // Not Implemented (R)
}

impl api::MandateSetup for Authorizedotnet {}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    // Issue: #173
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = authorizedotnet::AuthorizedotnetRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let connector_req =
            authorizedotnet::CancelOrCaptureTransactionRequest::try_from(&connector_router_data)?;

        let authorizedotnet_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<authorizedotnet::CancelOrCaptureTransactionRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
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
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsCaptureType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = intermediate_response
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = authorizedotnet::AuthorizedotnetCreateSyncRequest::try_from(req)?;
        let sync_request = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<authorizedotnet::AuthorizedotnetCreateSyncRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(sync_request))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
            .body(types::PaymentsSyncType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: types::Response,
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
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
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
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = authorizedotnet::AuthorizedotnetRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req =
            authorizedotnet::CreateTransactionRequest::try_from(&connector_router_data)?;

        let authorizedotnet_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<authorizedotnet::CreateTransactionRequest>::encode_to_string_of_json,
        )
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
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        res: types::Response,
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
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = authorizedotnet::CancelOrCaptureTransactionRequest::try_from(req)?;

        let authorizedotnet_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<authorizedotnet::CancelOrCaptureTransactionRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
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
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .body(types::PaymentsVoidType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetVoidResponse = intermediate_response
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

impl api::Refund for Authorizedotnet {}
impl api::RefundExecute for Authorizedotnet {}
impl api::RefundSync for Authorizedotnet {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = authorizedotnet::AuthorizedotnetRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req = authorizedotnet::CreateRefundRequest::try_from(&connector_router_data)?;

        let authorizedotnet_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<authorizedotnet::CreateRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetRefundResponse = intermediate_response
            .parse_struct("AuthorizedotnetRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Authorizedotnet
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        // This connector does not require an auth header, the authentication details are sent in the request body
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::RSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = authorizedotnet::AuthorizedotnetRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req =
            authorizedotnet::AuthorizedotnetCreateSyncRequest::try_from(&connector_router_data)?;

        let sync_request = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<authorizedotnet::AuthorizedotnetCreateSyncRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(sync_request))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
            .body(types::RefundSyncType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::RSync>,
        res: types::Response,
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
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

impl PaymentsCompleteAuthorize for Authorizedotnet {}

impl
    ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Authorizedotnet
{
    fn get_headers(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = authorizedotnet::AuthorizedotnetRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req =
            authorizedotnet::PaypalConfirmRequest::try_from(&connector_router_data)?;

        let authorizedotnet_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<authorizedotnet::PaypalConfirmRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(authorizedotnet_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCompleteAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsCompleteAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsCompleteAuthorizeType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCompleteAuthorizeRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        use bytes::Buf;

        // Handle the case where response bytes contains U+FEFF (BOM) character sent by connector
        let encoding = encoding_rs::UTF_8;
        let intermediate_response = encoding.decode_with_bom_removal(res.response.chunk());
        let intermediate_response =
            bytes::Bytes::copy_from_slice(intermediate_response.0.as_bytes());

        let response: authorizedotnet::AuthorizedotnetPaymentsResponse = intermediate_response
            .parse_struct("AuthorizedotnetPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        get_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Authorizedotnet {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha512))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let security_header = request
            .headers
            .get("X-ANET-Signature")
            .map(|header_value| {
                header_value
                    .to_str()
                    .map(String::from)
                    .map_err(|_| errors::ConnectorError::WebhookSignatureNotFound)
                    .into_report()
            })
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)
            .into_report()??
            .to_lowercase();
        let (_, sig_value) = security_header
            .split_once('=')
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)
            .into_report()?;
        hex::decode(sig_value)
            .into_report()
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(request.body.to_vec())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: authorizedotnet::AuthorizedotnetWebhookObjectId = request
            .body
            .parse_struct("AuthorizedotnetWebhookObjectId")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        match details.event_type {
            authorizedotnet::AuthorizedotnetWebhookEvent::RefundCreated => {
                Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                    api_models::webhooks::RefundIdType::ConnectorRefundId(
                        authorizedotnet::get_trans_id(&details)?,
                    ),
                ))
            }
            _ => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    authorizedotnet::get_trans_id(&details)?,
                ),
            )),
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let details: authorizedotnet::AuthorizedotnetWebhookEventType = request
            .body
            .parse_struct("AuthorizedotnetWebhookEventType")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(api::IncomingWebhookEvent::from(details.event_type))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let payload: authorizedotnet::AuthorizedotnetWebhookObjectId = request
            .body
            .parse_struct("AuthorizedotnetWebhookObjectId")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        let sync_payload = serde_json::to_value(
            authorizedotnet::AuthorizedotnetSyncResponse::try_from(payload)?,
        )
        .into_report()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        Ok(sync_payload)
    }
}

#[inline]
fn get_error_response(
    types::Response {
        response,
        status_code,
        ..
    }: types::Response,
) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
    let response: authorizedotnet::AuthorizedotnetPaymentsResponse = response
        .parse_struct("AuthorizedotnetPaymentsResponse")
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

    match response.transaction_response {
        Some(authorizedotnet::TransactionResponse::AuthorizedotnetTransactionResponse(
            payment_response,
        )) => Ok(payment_response
            .errors
            .and_then(|errors| {
                errors.into_iter().next().map(|error| types::ErrorResponse {
                    code: error.error_code,
                    message: error.error_text.to_owned(),
                    reason: Some(error.error_text),
                    status_code,
                })
            })
            .unwrap_or_else(|| types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(), // authorizedotnet sends 200 in case of bad request so this are hard coded to NO_ERROR_CODE and NO_ERROR_MESSAGE
                message: consts::NO_ERROR_MESSAGE.to_string(),
                reason: None,
                status_code,
            })),
        Some(authorizedotnet::TransactionResponse::AuthorizedotnetTransactionResponseError(_))
        | None => {
            let message = &response.messages.message[0].text;
            Ok(types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: message.to_string(),
                reason: Some(message.to_string()),
                status_code,
            })
        }
    }
}

impl services::ConnectorRedirectResponse for Authorizedotnet {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        match action {
            services::PaymentAction::PSync | services::PaymentAction::CompleteAuthorize => {
                Ok(payments::CallConnectorAction::Trigger)
            }
        }
    }
}
