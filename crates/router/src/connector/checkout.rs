#![allow(dead_code)]

mod transformers;

use std::fmt::Debug;

use common_utils::{crypto, ext_traits::ByteSliceExt};
use error_stack::{IntoReport, ResultExt};

use self::transformers as checkout;
use super::utils::{self as conn_utils, RefundsRequestData};
use crate::{
    configs::settings,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Checkout;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Checkout
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Checkout {
    fn id(&self) -> &'static str {
        "checkout"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }
    fn validate_auth_type(
        &self,
        val: &types::ConnectorAuthType,
    ) -> Result<(), error_stack::Report<errors::ConnectorError>> {
        checkout::CheckoutAuthType::try_from(val)?;
        Ok(())
    }
    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: checkout::CheckoutAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.api_secret).into_masked(),
        )])
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.checkout.base_url.as_ref()
    }
    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: checkout::ErrorResponse = if res.response.is_empty() {
            let (error_codes, error_type) = if res.status_code == 401 {
                (
                    Some(vec!["Invalid api key".to_string()]),
                    Some("invalid_api_key".to_string()),
                )
            } else {
                (None, None)
            };
            checkout::ErrorResponse {
                request_id: None,
                error_codes,
                error_type,
            }
        } else {
            res.response
                .parse_struct("ErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?
        };

        router_env::logger::info!(error_response=?response);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error_type
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error_codes
                .as_ref()
                .and_then(|error_codes| error_codes.first().cloned())
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error_codes.map(|errors| errors.join(" & ")),
        })
    }
}

impl api::Payment for Checkout {}

impl api::PaymentAuthorize for Checkout {}
impl api::PaymentSync for Checkout {}
impl api::PaymentVoid for Checkout {}
impl api::PaymentCapture for Checkout {}
impl api::PaymentSession for Checkout {}
impl api::ConnectorAccessToken for Checkout {}
impl api::AcceptDispute for Checkout {}
impl api::PaymentToken for Checkout {}
impl api::Dispute for Checkout {}
impl api::RetrieveFile for Checkout {}
impl api::DefendDispute for Checkout {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Checkout
{
    fn get_headers(
        &self,
        req: &types::TokenizationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        )];
        let api_key = checkout::CheckoutAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut auth = vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", api_key.api_key).into_masked(),
        )];
        header.append(&mut auth);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}tokens", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::TokenizationRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = checkout::TokenRequest::try_from(req)?;
        let checkout_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<checkout::TokenRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(checkout_req))
    }

    fn build_request(
        &self,
        req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::TokenizationType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::TokenizationType::get_headers(self, req, connectors)?)
                .body(types::TokenizationType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::TokenizationRouterData,
        res: types::Response,
    ) -> CustomResult<types::TokenizationRouterData, errors::ConnectorError>
    where
        types::PaymentsResponseData: Clone,
    {
        let response: checkout::CheckoutTokenResponse = res
            .response
            .parse_struct("CheckoutTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Checkout
{
    // Not Implemented (R)
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Checkout
{
    // Not Implemented (R)
}

impl api::PreVerify for Checkout {}

impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Checkout
{
    // Issue: #173
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.as_str();
        Ok(format!(
            "{}payments/{id}/captures",
            self.base_url(connectors)
        ))
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = checkout::PaymentCaptureRequest::try_from(req)?;
        let checkout_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<checkout::PaymentCaptureRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(checkout_req))
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
        let response: checkout::PaymentCaptureResponse = res
            .response
            .parse_struct("CaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}{}",
            self.base_url(connectors),
            "payments/",
            req.request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
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
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .body(types::PaymentsSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError>
    where
        api::PSync: Clone,
        types::PaymentsSyncData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: checkout::PaymentsResponse = res
            .response
            .parse_struct("PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "payments"))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = checkout::PaymentsRequest::try_from(req)?;
        let checkout_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<checkout::PaymentsRequest>::encode_to_string_of_json,
        )
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
        let response: checkout::PaymentsResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
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
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = checkout::PaymentVoidRequest::try_from(req)?;
        let checkout_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<checkout::PaymentVoidRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(checkout_req))
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
        let mut response: checkout::PaymentVoidResponse = res
            .response
            .parse_struct("PaymentVoidResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        response.status = res.status_code;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Checkout {}
impl api::RefundExecute for Checkout {}
impl api::RefundSync for Checkout {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
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
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = checkout::RefundRequest::try_from(req)?;
        let body = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<checkout::RefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(body))
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
        let response: checkout::RefundResponse = res
            .response
            .parse_struct("checkout::RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
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
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Checkout {
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
        connectors: &settings::Connectors,
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
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::RSync>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::RSync>, errors::ConnectorError> {
        let refund_action_id = data.request.get_connector_refund_id()?;

        let response: Vec<checkout::ActionResponse> = res
            .response
            .parse_struct("checkout::CheckoutRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);

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
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<api::Accept, types::AcceptDisputeRequestData, types::AcceptDisputeResponse>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::AcceptDisputeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::AcceptDisputeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::AcceptDisputeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}{}{}",
            self.base_url(connectors),
            "disputes/",
            req.request.connector_dispute_id,
            "/accept"
        ))
    }

    fn build_request(
        &self,
        req: &types::AcceptDisputeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::AcceptDisputeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::AcceptDisputeType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::AcceptDisputeRouterData,
        _res: types::Response,
    ) -> CustomResult<types::AcceptDisputeRouterData, errors::ConnectorError> {
        Ok(types::AcceptDisputeRouterData {
            response: Ok(types::AcceptDisputeResponse {
                dispute_status: api::enums::DisputeStatus::DisputeAccepted,
                connector_status: None,
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::UploadFile for Checkout {}

impl
    ConnectorIntegration<api::Retrieve, types::RetrieveFileRequestData, types::RetrieveFileResponse>
    for Checkout
{
}

#[async_trait::async_trait]
impl api::FileUpload for Checkout {
    fn validate_file_upload(
        &self,
        purpose: api::FilePurpose,
        file_size: i32,
        file_type: mime::Mime,
    ) -> CustomResult<(), errors::ConnectorError> {
        match purpose {
            api::FilePurpose::DisputeEvidence => {
                let supported_file_types =
                    vec!["image/jpeg", "image/jpg", "image/png", "application/pdf"];
                // 4 Megabytes (MB)
                if file_size > 4000000 {
                    Err(errors::ConnectorError::FileValidationFailed {
                        reason: "file_size exceeded the max file size of 4MB".to_owned(),
                    })?
                }
                if !supported_file_types.contains(&file_type.to_string().as_str()) {
                    Err(errors::ConnectorError::FileValidationFailed {
                        reason: "file_type does not match JPEG, JPG, PNG, or PDF format".to_owned(),
                    })?
                }
            }
        }
        Ok(())
    }
}

impl ConnectorIntegration<api::Upload, types::UploadFileRequestData, types::UploadFileResponse>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::RouterData<
            api::Upload,
            types::UploadFileRequestData,
            types::UploadFileResponse,
        >,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_content_type(&self) -> &'static str {
        "multipart/form-data"
    }

    fn get_url(
        &self,
        _req: &types::UploadFileRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "files"))
    }

    fn get_request_form_data(
        &self,
        req: &types::UploadFileRouterData,
    ) -> CustomResult<Option<reqwest::multipart::Form>, errors::ConnectorError> {
        let checkout_req = transformers::construct_file_upload_request(req.clone())?;
        Ok(Some(checkout_req))
    }

    fn build_request(
        &self,
        req: &types::UploadFileRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::UploadFileType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::UploadFileType::get_headers(self, req, connectors)?)
                .form_data(types::UploadFileType::get_request_form_data(self, req)?)
                .content_type(services::request::ContentType::FormData)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::UploadFileRouterData,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<api::Upload, types::UploadFileRequestData, types::UploadFileResponse>,
        errors::ConnectorError,
    > {
        let response: checkout::FileUploadResponse = res
            .response
            .parse_struct("Checkout FileUploadResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        Ok(types::UploadFileRouterData {
            response: Ok(types::UploadFileResponse {
                provider_file_id: response.file_id,
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::SubmitEvidence for Checkout {}

impl
    ConnectorIntegration<
        api::Evidence,
        types::SubmitEvidenceRequestData,
        types::SubmitEvidenceResponse,
    > for Checkout
{
    fn get_headers(
        &self,
        req: &types::SubmitEvidenceRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::SubmitEvidenceType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::SubmitEvidenceRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}disputes/{}/evidence",
            self.base_url(connectors),
            req.request.connector_dispute_id,
        ))
    }

    fn get_request_body(
        &self,
        req: &types::SubmitEvidenceRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let checkout_req = checkout::Evidence::try_from(req)?;
        let checkout_req_string = types::RequestBody::log_and_get_request_body(
            &checkout_req,
            utils::Encode::<checkout::Evidence>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(checkout_req_string))
    }

    fn build_request(
        &self,
        req: &types::SubmitEvidenceRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Put)
            .url(&types::SubmitEvidenceType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::SubmitEvidenceType::get_headers(
                self, req, connectors,
            )?)
            .body(types::SubmitEvidenceType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::SubmitEvidenceRouterData,
        _res: types::Response,
    ) -> CustomResult<types::SubmitEvidenceRouterData, errors::ConnectorError> {
        Ok(types::SubmitEvidenceRouterData {
            response: Ok(types::SubmitEvidenceResponse {
                dispute_status: api_models::enums::DisputeStatus::DisputeChallenged,
                connector_status: None,
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<api::Defend, types::DefendDisputeRequestData, types::DefendDisputeResponse>
    for Checkout
{
    fn get_headers(
        &self,
        req: &types::DefendDisputeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::DefendDisputeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::DefendDisputeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}disputes/{}/evidence",
            self.base_url(connectors),
            req.request.connector_dispute_id,
        ))
    }

    fn build_request(
        &self,
        req: &types::DefendDisputeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::DefendDisputeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::DefendDisputeType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::DefendDisputeRouterData,
        _res: types::Response,
    ) -> CustomResult<types::DefendDisputeRouterData, errors::ConnectorError> {
        Ok(types::DefendDisputeRouterData {
            response: Ok(types::DefendDisputeResponse {
                dispute_status: api::enums::DisputeStatus::DisputeChallenged,
                connector_status: None,
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Checkout {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }
    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let signature = conn_utils::get_header_key_value("cko-signature", request.headers)
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
        hex::decode(signature)
            .into_report()
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }
    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _secret: &[u8],
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(format!("{}", String::from_utf8_lossy(request.body)).into_bytes())
    }
    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: checkout::CheckoutWebhookBody = request
            .body
            .parse_struct("CheckoutWebhookBody")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        if checkout::is_chargeback_event(&details.transaction_type) {
            return Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    details
                        .data
                        .payment_id
                        .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                ),
            ));
        }
        if checkout::is_refund_event(&details.transaction_type) {
            return Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::ConnectorRefundId(
                    details
                        .data
                        .action_id
                        .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                ),
            ));
        }
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(details.data.id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let details: checkout::CheckoutWebhookEventTypeBody = request
            .body
            .parse_struct("CheckoutWebhookBody")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(api::IncomingWebhookEvent::from(details.transaction_type))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let details: checkout::CheckoutWebhookObjectResource = request
            .body
            .parse_struct("CheckoutWebhookObjectResource")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(details.data)
    }

    fn get_dispute_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::disputes::DisputePayload, errors::ConnectorError> {
        let dispute_details: checkout::CheckoutDisputeWebhookBody = request
            .body
            .parse_struct("CheckoutWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(api::disputes::DisputePayload {
            amount: dispute_details.data.amount.to_string(),
            currency: dispute_details.data.currency,
            dispute_stage: api_models::enums::DisputeStage::from(
                dispute_details.transaction_type.clone(),
            ),
            connector_dispute_id: dispute_details.data.id,
            connector_reason: None,
            connector_reason_code: dispute_details.data.reason_code,
            challenge_required_by: dispute_details.data.evidence_required_by,
            connector_status: dispute_details.transaction_type.to_string(),
            created_at: dispute_details.created_on,
            updated_at: dispute_details.data.date,
        })
    }
}

impl services::ConnectorRedirectResponse for Checkout {
    fn get_flow_type(
        &self,
        query_params: &str,
        _json_payload: Option<serde_json::Value>,
        _action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        let query =
            serde_urlencoded::from_str::<transformers::CheckoutRedirectResponse>(query_params)
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let connector_action = query
            .status
            .map(
                |checkout_status| payments::CallConnectorAction::StatusUpdate {
                    status: diesel_models::enums::AttemptStatus::from(checkout_status),
                    error_code: None,
                    error_message: None,
                },
            )
            .unwrap_or(payments::CallConnectorAction::Trigger);
        Ok(connector_action)
    }
}
