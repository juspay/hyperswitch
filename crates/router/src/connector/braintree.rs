pub mod braintree_graphql_transformers;
pub mod transformers;

use std::fmt::Debug;

use error_stack::{IntoReport, Report, ResultExt};
use masking::PeekInterface;

use self::transformers as braintree;
use super::utils::PaymentsAuthorizeRequestData;
use crate::{
    configs::settings,
    consts,
    core::errors::{self, CustomResult},
    headers, logger,
    services::{
        self,
        request::{self, Mask},
    },
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Braintree;

impl ConnectorCommon for Braintree {
    fn id(&self) -> &'static str {
        "braintree"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.braintree.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: braintree_graphql_transformers::BraintreeAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.auth_header.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: Result<braintree::ErrorResponse, Report<common_utils::errors::ParsingError>> =
            res.response.parse_struct("Braintree Error Response");

        match response {
            Ok(braintree::ErrorResponse::BraintreeApiErrorResponse(response)) => {
                let error_object = response.api_error_response.errors;
                let error = error_object.errors.first().or(error_object
                    .transaction
                    .as_ref()
                    .and_then(|transaction_error| {
                        transaction_error.errors.first().or(transaction_error
                            .credit_card
                            .as_ref()
                            .and_then(|credit_card_error| credit_card_error.errors.first()))
                    }));
                let (code, message) = error.map_or(
                    (
                        consts::NO_ERROR_CODE.to_string(),
                        consts::NO_ERROR_MESSAGE.to_string(),
                    ),
                    |error| (error.code.clone(), error.message.clone()),
                );
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code,
                    message,
                    reason: Some(response.api_error_response.message),
                })
            }
            Ok(braintree::ErrorResponse::BraintreeErrorResponse(response)) => Ok(ErrorResponse {
                status_code: res.status_code,
                code: consts::NO_ERROR_CODE.to_string(),
                message: consts::NO_ERROR_MESSAGE.to_string(),
                reason: Some(response.errors),
            }),
            Err(error_msg) => {
                logger::error!(deserialization_error =? error_msg);
                utils::handle_json_response_deserialization_failure(res, "braintree".to_owned())
            }
        }
    }
}

impl api::Payment for Braintree {}

impl api::PaymentAuthorize for Braintree {}
impl api::PaymentSync for Braintree {}
impl api::PaymentVoid for Braintree {}
impl api::PaymentCapture for Braintree {}

impl api::PaymentSession for Braintree {}
impl api::ConnectorAccessToken for Braintree {}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Braintree
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsSessionRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Ok(vec![])
        } else {
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::PaymentsSessionType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (headers::X_API_VERSION.to_string(), "6".to_string().into()),
                (
                    headers::ACCEPT.to_string(),
                    "application/json".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        }
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
        } else {
            let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
                .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
            Ok(format!(
                "{}/merchants/{}/client_token",
                self.base_url(connectors),
                auth_type.merchant_id.peek(),
            ))
        }
    }

    fn build_request(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Ok(None)
        } else {
            let request = Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::PaymentsSessionType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::PaymentsSessionType::get_headers(
                        self, req, connectors,
                    )?)
                    .body(types::PaymentsSessionType::get_request_body(self, req)?)
                    .build(),
            );
            Ok(request)
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Err(
                errors::ConnectorError::NotImplemented("get_request_body method".to_string())
                    .into(),
            )
        } else {
            let connector_request = braintree::BraintreeSessionRequest::try_from(req)?;
            let braintree_session_request = types::RequestBody::log_and_get_request_body(
                &connector_request,
                utils::Encode::<braintree::BraintreeSessionRequest>::encode_to_string_of_json,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(braintree_session_request))
        }
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSessionRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSessionRouterData, errors::ConnectorError> {
        let response: braintree::BraintreeSessionTokenResponse = res
            .response
            .parse_struct("braintree SessionTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl api::PaymentToken for Braintree {}

impl
    services::ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::TokenizationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::TokenizationType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (
                    "Braintree-Version".to_string(),
                    "2019-01-01".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        } else {
            Ok(vec![])
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::TokenizationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Ok(format!(
                "https://payments.sandbox.braintree-api.com/graphql"
            ))
        } else {
            Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
        }
    }

    fn get_request_body(
        &self,
        req: &types::TokenizationRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let connector_request =
                braintree_graphql_transformers::BraintreeTokenRequest::try_from(req)?;

            let braintree_req = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<braintree_graphql_transformers::BraintreeTokenRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(braintree_req))
        } else {
            Err(
                errors::ConnectorError::NotImplemented("get_request_body method".to_string())
                    .into(),
            )
        }
    }

    fn build_request(
        &self,
        req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Ok(Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::TokenizationType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::TokenizationType::get_headers(self, req, connectors)?)
                    .body(types::TokenizationType::get_request_body(self, req)?)
                    .build(),
            ))
        } else {
            Ok(None)
        }
    }

    fn handle_response(
        &self,
        data: &types::TokenizationRouterData,
        res: types::Response,
    ) -> CustomResult<types::TokenizationRouterData, errors::ConnectorError>
    where
        types::PaymentsResponseData: Clone,
    {
        let response: braintree_graphql_transformers::BraintreeTokenResponse = res
            .response
            .parse_struct("BraintreeTokenResponse")
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PreVerify for Braintree {}

#[allow(dead_code)]
impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Braintree
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::TokenizationType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (
                    "Braintree-Version".to_string(),
                    "2019-01-01".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        } else {
            Ok(vec![])
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Ok(format!(
                "https://payments.sandbox.braintree-api.com/graphql"
            ))
        } else {
            Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let connector_request =
                braintree_graphql_transformers::BraintreeCaptureRequest::try_from(req)?;

            let braintree_req = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<braintree_graphql_transformers::BraintreeCaptureRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(braintree_req))
        } else {
            Err(
                errors::ConnectorError::NotImplemented("get_request_body method".to_string())
                    .into(),
            )
        }
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
        let response: braintree_graphql_transformers::BraintreeCaptureResponse = res
            .response
            .parse_struct("Braintree PaymentsCaptureResponse")
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::TokenizationType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (
                    "Braintree-Version".to_string(),
                    "2019-01-01".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        } else {
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::PaymentsSyncType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (headers::X_API_VERSION.to_string(), "6".to_string().into()),
                (
                    headers::ACCEPT.to_string(),
                    "application/json".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Ok(format!(
                "https://payments.sandbox.braintree-api.com/graphql"
            ))
        } else {
            let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
                .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
            let connector_payment_id = req
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
            Ok(format!(
                "{}/merchants/{}/transactions/{}",
                self.base_url(connectors),
                auth_type.merchant_id.peek(),
                connector_payment_id
            ))
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let connector_request =
                braintree_graphql_transformers::BraintreePSyncRequest::try_from(req)?;

            let braintree_req = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<braintree_graphql_transformers::BraintreePSyncRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(braintree_req))
        } else {
            Ok(None)
        }
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let is_connector_new_version = data.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let response: braintree_graphql_transformers::BraintreePSyncResponse = res
                .response
                .parse_struct("Braintree PaymentSyncResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        } else {
            let response: braintree::BraintreePaymentsResponse = res
                .response
                .parse_struct("Braintree PaymentsResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            println!("heyya->");
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::PaymentsAuthorizeType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (
                    "Braintree-Version".to_string(),
                    "2019-01-01".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        } else {
            println!("heloo->");
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::PaymentsAuthorizeType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (headers::X_API_VERSION.to_string(), "6".to_string().into()),
                (
                    headers::ACCEPT.to_string(),
                    "application/json".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        }
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Ok(format!(
                "https://payments.sandbox.braintree-api.com/graphql"
            ))
        } else {
            let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
                .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

            Ok(format!(
                "{}merchants/{}/transactions",
                self.base_url(connectors),
                auth_type.merchant_id.peek()
            ))
        }
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
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            match req.request.is_auto_capture()? {
                true => {
                    let connector_request =
                        braintree_graphql_transformers::BraintreePaymentsRequest::try_from(req)?;
                    let braintree_payment_request = types::RequestBody::log_and_get_request_body(
                    &connector_request,
                    utils::Encode::<braintree_graphql_transformers::BraintreePaymentsRequest>::encode_to_string_of_json,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                    Ok(Some(braintree_payment_request))
                }
                false => {
                    let connector_request =
                        braintree_graphql_transformers::BraintreeAuthRequest::try_from(req)?;
                    let braintree_payment_request = types::RequestBody::log_and_get_request_body(
                    &connector_request,
                    utils::Encode::<braintree_graphql_transformers::BraintreeAuthRequest>::encode_to_string_of_json,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                    Ok(Some(braintree_payment_request))
                }
            }
        } else {
            let connector_request = braintree::BraintreePaymentsRequest::try_from(req)?;
            let braintree_payment_request = types::RequestBody::log_and_get_request_body(
                &connector_request,
                utils::Encode::<braintree::BraintreePaymentsRequest>::encode_to_string_of_json,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(braintree_payment_request))
        }
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let is_connector_new_version = data.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            match data.request.is_auto_capture()? {
                true => {
                    let response: braintree_graphql_transformers::BraintreePaymentsResponse = res
                        .response
                        .parse_struct("Braintree PaymentsResponse")
                        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                    types::RouterData::try_from(types::ResponseRouterData {
                        response,
                        data: data.clone(),
                        http_code: res.status_code,
                    })
                }
                false => {
                    let response: braintree_graphql_transformers::BraintreeAuthResponse = res
                        .response
                        .parse_struct("Braintree AuthResponse")
                        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                    types::RouterData::try_from(types::ResponseRouterData {
                        response,
                        data: data.clone(),
                        http_code: res.status_code,
                    })
                }
            }
        } else {
            let response: braintree::BraintreePaymentsResponse = res
                .response
                .parse_struct("Braintree PaymentsResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::PaymentsVoidType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (
                    "Braintree-Version".to_string(),
                    "2019-01-01".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        } else {
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::PaymentsVoidType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (headers::X_API_VERSION.to_string(), "6".to_string().into()),
                (
                    headers::ACCEPT.to_string(),
                    "application/json".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Ok(format!(
                "https://payments.sandbox.braintree-api.com/graphql"
            ))
        } else {
            let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
                .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
            Ok(format!(
                "{}merchants/{}/transactions/{}/void",
                self.base_url(connectors),
                auth_type.merchant_id.peek(),
                req.request.connector_transaction_id
            ))
        }
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

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let connector_request =
                braintree_graphql_transformers::BraintreeCancelRequest::try_from(req)?;
            let braintree_req = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<braintree_graphql_transformers::BraintreeCancelRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(braintree_req))
        } else {
            Ok(None)
        }
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let is_connector_new_version = data.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let response: braintree_graphql_transformers::BraintreeCancelResponse = res
                .response
                .parse_struct("Braintree VoidResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        } else {
            let response: braintree::BraintreePaymentsResponse = res
                .response
                .parse_struct("Braintree PaymentsVoidResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Braintree {}
impl api::RefundExecute for Braintree {}
impl api::RefundSync for Braintree {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::TokenizationType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (
                    "Braintree-Version".to_string(),
                    "2019-01-01".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        } else {
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::RefundExecuteType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (headers::X_API_VERSION.to_string(), "6".to_string().into()),
                (
                    headers::ACCEPT.to_string(),
                    "application/json".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Ok(format!(
                "https://payments.sandbox.braintree-api.com/graphql"
            ))
        } else {
            let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
                .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
            let connector_payment_id = req.request.connector_transaction_id.clone();
            Ok(format!(
                "{}merchants/{}/transactions/{}",
                self.base_url(connectors),
                auth_type.merchant_id.peek(),
                connector_payment_id
            ))
        }
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let connector_request =
                braintree_graphql_transformers::BraintreeRefundRequest::try_from(req)?;
            let braintree_refund_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<braintree_graphql_transformers::BraintreeRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(braintree_refund_request))
        } else {
            let connector_request = braintree::BraintreeRefundRequest::try_from(req)?;
            let braintree_refund_request = types::RequestBody::log_and_get_request_body(
                &connector_request,
                utils::Encode::<braintree::BraintreeRefundRequest>::encode_to_string_of_json,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(braintree_refund_request))
        }
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
        let is_connector_new_version = data.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let response: braintree_graphql_transformers::BraintreeRefundResponse = res
                .response
                .parse_struct("Braintree RefundResponse")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        } else {
            let response: braintree::RefundResponse = res
                .response
                .parse_struct("Braintree RefundResponse")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let mut headers = vec![
                (
                    headers::CONTENT_TYPE.to_string(),
                    types::TokenizationType::get_content_type(self)
                        .to_string()
                        .into(),
                ),
                (
                    "Braintree-Version".to_string(),
                    "2019-01-01".to_string().into(),
                ),
            ];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            headers.append(&mut api_key);
            Ok(headers)
        } else {
            Ok(vec![])
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            Ok(format!(
                "https://payments.sandbox.braintree-api.com/graphql"
            ))
        } else {
            Ok(format!(""))
        }
    }

    fn get_request_body(
        &self,
        req: &types::RefundSyncRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let is_connector_new_version = req.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let connector_request =
                braintree_graphql_transformers::BraintreeRSyncRequest::try_from(req)?;
            let braintree_refund_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<braintree_graphql_transformers::BraintreeRSyncRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(braintree_refund_request))
        } else {
            Ok(None)
        }
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        errors::ConnectorError,
    > {
        let is_connector_new_version = data.is_connector_new_version;
        if is_connector_new_version == Some(true) {
            let response: braintree_graphql_transformers::BraintreeRSyncResponse = res
                .response
                .parse_struct("Braintree RefundResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        } else {
            let response: braintree::RefundResponse = res
                .response
                .parse_struct("Braintree RefundResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        }
    }
    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Braintree {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Ok(api::IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
