pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_utils::{
    crypto,
    ext_traits::{StringExt, ValueExt},
};
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use transformers as bluesnap;

use super::utils::{
    self as connector_utils, get_error_code_error_message_based_on_priority, ConnectorErrorType,
    ConnectorErrorTypeMapping, PaymentsAuthorizeRequestData, RefundsRequestData, RouterData,
};
use crate::{
    configs::settings,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    db::StorageInterface,
    headers, logger,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        domain, ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Bluesnap;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Bluesnap
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = self.get_auth_header(&req.connector_auth_type)?;
        header.push((
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        ));
        Ok(header)
    }
}

impl ConnectorCommon for Bluesnap {
    fn id(&self) -> &'static str {
        "bluesnap"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.bluesnap.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: bluesnap::BluesnapAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let encoded_api_key =
            consts::BASE64_ENGINE.encode(format!("{}:{}", auth.key1.peek(), auth.api_key.peek()));
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Basic {encoded_api_key}").into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        logger::debug!(bluesnap_error_response=?res);
        let response: bluesnap::BluesnapErrors = res
            .response
            .parse_struct("BluesnapErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);

        let response_error_message = match response {
            bluesnap::BluesnapErrors::Payment(error_response) => {
                let error_list = error_response.message.clone();
                let option_error_code_message = get_error_code_error_message_based_on_priority(
                    Self.clone(),
                    error_list.into_iter().map(|errors| errors.into()).collect(),
                );
                let reason = error_response
                    .message
                    .iter()
                    .map(|error| error.description.clone())
                    .collect::<Vec<String>>()
                    .join(" & ");
                ErrorResponse {
                    status_code: res.status_code,
                    code: option_error_code_message
                        .clone()
                        .map(|error_code_message| error_code_message.error_code)
                        .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                    message: option_error_code_message
                        .map(|error_code_message| error_code_message.error_message)
                        .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    reason: Some(reason),
                }
            }
            bluesnap::BluesnapErrors::Auth(error_res) => ErrorResponse {
                status_code: res.status_code,
                code: error_res.error_code.clone(),
                message: error_res.error_name.clone(),
                reason: Some(error_res.error_description),
            },
            bluesnap::BluesnapErrors::General(error_response) => ErrorResponse {
                status_code: res.status_code,
                code: consts::NO_ERROR_CODE.to_string(),
                message: error_response.clone(),
                reason: Some(error_response),
            },
        };
        Ok(response_error_message)
    }
}

impl ConnectorValidation for Bluesnap {
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

impl api::Payment for Bluesnap {}

impl api::PaymentToken for Bluesnap {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Bluesnap
{
    // Not Implemented (R)
}

impl api::PreVerify for Bluesnap {}
impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Bluesnap
{
}

impl api::ConnectorCustomer for Bluesnap {}

impl
    ConnectorIntegration<
        api::CreateConnectorCustomer,
        types::ConnectorCustomerData,
        types::PaymentsResponseData,
    > for Bluesnap
{
    fn get_headers(
        &self,
        req: &types::ConnectorCustomerRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::ConnectorCustomerRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}services/2/vaulted-shoppers",
            self.base_url(connectors),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::ConnectorCustomerRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = bluesnap::BluesnapCustomerRequest::try_from(req)?;
        let bluesnap_req = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<bluesnap::BluesnapCustomerRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(bluesnap_req))
    }

    fn build_request(
        &self,
        req: &types::ConnectorCustomerRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::ConnectorCustomerType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ConnectorCustomerType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::ConnectorCustomerType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::ConnectorCustomerRouterData,
        res: Response,
    ) -> CustomResult<types::ConnectorCustomerRouterData, errors::ConnectorError>
    where
        types::PaymentsResponseData: Clone,
    {
        let response: bluesnap::BluesnapCustomerResponse = res
            .response
            .parse_struct("BluesnapCustomerResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentVoid for Bluesnap {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Bluesnap
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
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "services/2/transactions"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = bluesnap::BluesnapVoidRequest::try_from(req)?;
        let bluesnap_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<bluesnap::BluesnapVoidRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(bluesnap_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Put)
            .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
            .body(types::PaymentsVoidType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: bluesnap::BluesnapPaymentsResponse = res
            .response
            .parse_struct("BluesnapPaymentsResponse")
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::ConnectorAccessToken for Bluesnap {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Bluesnap
{
}

impl api::PaymentSync for Bluesnap {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Bluesnap
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}{}{}",
            self.base_url(connectors),
            "services/2/transactions/",
            connector_payment_id
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
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: bluesnap::BluesnapPaymentsResponse = res
            .response
            .parse_struct("BluesnapPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::PaymentCapture for Bluesnap {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Bluesnap
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
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "services/2/transactions"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = bluesnap::BluesnapCaptureRequest::try_from(req)?;
        let bluesnap_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<bluesnap::BluesnapCaptureRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(bluesnap_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Put)
            .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsCaptureType::get_headers(
                self, req, connectors,
            )?)
            .body(types::PaymentsCaptureType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: bluesnap::BluesnapPaymentsResponse = res
            .response
            .parse_struct("Bluesnap BluesnapPaymentsResponse")
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

// This session code is not used
impl api::PaymentSession for Bluesnap {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Bluesnap
{
    fn get_headers(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSessionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "services/2/wallets"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = bluesnap::BluesnapCreateWalletToken::try_from(req)?;
        let bluesnap_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<bluesnap::BluesnapCreateWalletToken>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(bluesnap_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsSessionType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSessionType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsSessionType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSessionRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSessionRouterData, errors::ConnectorError> {
        let response: bluesnap::BluesnapWalletTokenResponse = res
            .response
            .parse_struct("BluesnapWalletTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentAuthorize for Bluesnap {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Bluesnap
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        match req.is_three_ds() && !req.request.is_wallet() {
            true => Ok(format!(
                "{}{}{}",
                self.base_url(connectors),
                "services/2/payment-fields-tokens?shopperId=",
                req.get_connector_customer_id()?
            )),
            _ => Ok(format!(
                "{}{}",
                self.base_url(connectors),
                "services/2/transactions"
            )),
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = bluesnap::BluesnapPaymentsRequest::try_from(req)?;
        let bluesnap_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<bluesnap::BluesnapPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(bluesnap_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        self.validate_capture_method(req.request.capture_method)?;
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
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        match (data.is_three_ds() && !data.request.is_wallet(), res.headers) {
            (true, Some(headers)) => {
                let location = connector_utils::get_http_header("Location", &headers)?;
                let payment_fields_token = location
                    .split('/')
                    .last()
                    .ok_or(errors::ConnectorError::ResponseHandlingFailed)?
                    .to_string();
                Ok(types::RouterData {
                    status: enums::AttemptStatus::AuthenticationPending,
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::NoResponseId,
                        redirection_data: Some(services::RedirectForm::BlueSnap {
                            payment_fields_token,
                        }),
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                    }),
                    ..data.clone()
                })
            }
            _ => {
                let response: bluesnap::BluesnapPaymentsResponse = res
                    .response
                    .parse_struct("BluesnapPaymentsResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
        }
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentsCompleteAuthorize for Bluesnap {}

impl
    ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Bluesnap
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
        Ok(format!(
            "{}services/2/transactions",
            self.base_url(connectors),
        ))
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = bluesnap::BluesnapPaymentsRequest::try_from(req)?;
        let bluesnap_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<bluesnap::BluesnapPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(bluesnap_req))
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
        res: Response,
    ) -> CustomResult<types::PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        let response: bluesnap::BluesnapPaymentsResponse = res
            .response
            .parse_struct("BluesnapPaymentsResponse")
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Bluesnap {}
impl api::RefundExecute for Bluesnap {}
impl api::RefundSync for Bluesnap {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Bluesnap
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
        Ok(format!(
            "{}{}{}",
            self.base_url(connectors),
            "services/2/transactions/refund/",
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = bluesnap::BluesnapRefundRequest::try_from(req)?;
        let bluesnap_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<bluesnap::BluesnapRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(bluesnap_req))
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
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: bluesnap::RefundResponse = res
            .response
            .parse_struct("bluesnap RefundResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Bluesnap {
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}{}",
            self.base_url(connectors),
            "services/2/transactions/",
            req.request.get_connector_refund_id()?
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
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: bluesnap::BluesnapPaymentsResponse = res
            .response
            .parse_struct("bluesnap BluesnapPaymentsResponse")
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Bluesnap {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::Md5))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let webhook_body: bluesnap::BluesnapWebhookBody =
            serde_urlencoded::from_bytes(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
        let signature = webhook_body.auth_key;
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
        let webhook_body: bluesnap::BluesnapWebhookBody =
            serde_urlencoded::from_bytes(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
        let msg = webhook_body.reference_number + webhook_body.contract_id.as_str();
        Ok(msg.into_bytes())
    }

    async fn verify_webhook_source(
        &self,
        db: &dyn StorageInterface,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_account: &domain::MerchantAccount,
        connector_label: &str,
        key_store: &domain::MerchantKeyStore,
        object_reference_id: api_models::webhooks::ObjectReferenceId,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let algorithm = self
            .get_webhook_source_verification_algorithm(request)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let mut secret = self
            .get_webhook_source_verification_merchant_secret(
                db,
                merchant_account,
                connector_label,
                key_store,
                object_reference_id,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let mut message = self
            .get_webhook_source_verification_message(
                request,
                &merchant_account.merchant_id,
                &secret,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        message.append(&mut secret);
        algorithm
            .verify_signature(&secret, &signature, &message)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook_body: bluesnap::BluesnapWebhookBody =
            serde_urlencoded::from_bytes(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(
                webhook_body.reference_number,
            ),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let details: bluesnap::BluesnapWebhookObjectEventType =
            serde_urlencoded::from_bytes(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(match details.transaction_type {
            bluesnap::BluesnapWebhookEvents::Decline
            | bluesnap::BluesnapWebhookEvents::CcChargeFailed => {
                api::IncomingWebhookEvent::PaymentIntentFailure
            }
            bluesnap::BluesnapWebhookEvents::Charge => {
                api::IncomingWebhookEvent::PaymentIntentSuccess
            }
            bluesnap::BluesnapWebhookEvents::Unknown => {
                api::IncomingWebhookEvent::EventNotSupported
            }
        })
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let details: bluesnap::BluesnapWebhookObjectResource =
            serde_urlencoded::from_bytes(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        let res_json =
            utils::Encode::<transformers::BluesnapWebhookObjectResource>::encode_to_value(&details)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(res_json)
    }
}

impl services::ConnectorRedirectResponse for Bluesnap {
    fn get_flow_type(
        &self,
        _query_params: &str,
        json_payload: Option<serde_json::Value>,
        _action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        let redirection_response: bluesnap::BluesnapRedirectionResponse = json_payload
            .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "json_payload",
            })?
            .parse_value("BluesnapRedirectionResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let redirection_result: bluesnap::BluesnapThreeDsResult = redirection_response
            .authentication_response
            .parse_struct("BluesnapThreeDsResult")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        match redirection_result.status.as_str() {
            "Success" => Ok(payments::CallConnectorAction::Trigger),
            _ => Ok(payments::CallConnectorAction::StatusUpdate {
                status: enums::AttemptStatus::AuthenticationFailed,
                error_code: redirection_result.code,
                error_message: redirection_result
                    .info
                    .as_ref()
                    .and_then(|info| info.errors.as_ref().and_then(|error| error.first()))
                    .cloned(),
            }),
        }
    }
}

impl ConnectorErrorTypeMapping for Bluesnap {
    fn get_connector_error_type(
        &self,
        error_code: String,
        error_message: String,
    ) -> ConnectorErrorType {
        match (error_code.as_str(), error_message.as_str()) {
            ("7", "INVALID_TRANSACTION_TYPE") => ConnectorErrorType::UserError,
            ("30", "MISSING_SHOPPER_OR_CARD_HOLDER") => ConnectorErrorType::UserError,
            ("85", "INVALID_HTTP_METHOD") => ConnectorErrorType::BusinessError,
            ("90", "MISSING_CARD_TYPE") => ConnectorErrorType::BusinessError,
            ("10000", "INVALID_API_VERSION") => ConnectorErrorType::BusinessError,
            ("10000", "PAYMENT_GENERAL_FAILURE") => ConnectorErrorType::TechnicalError,
            ("10000", "SERVER_GENERAL_FAILURE") => ConnectorErrorType::BusinessError,
            ("10001", "VALIDATION_GENERAL_FAILURE") => ConnectorErrorType::BusinessError,
            ("10001", "INVALID_MERCHANT_TRANSACTION_ID") => ConnectorErrorType::BusinessError,
            ("10001", "INVALID_RECURRING_TRANSACTION") => ConnectorErrorType::BusinessError,
            ("10001", "MERCHANT_CONFIGURATION_ERROR") => ConnectorErrorType::BusinessError,
            ("10001", "MISSING_CARD_TYPE") => ConnectorErrorType::BusinessError,
            ("11001", "XSS_EXCEPTION") => ConnectorErrorType::UserError,
            ("14002", "THREE_D_SECURITY_AUTHENTICATION_REQUIRED") => {
                ConnectorErrorType::TechnicalError
            }
            ("14002", "ACCOUNT_CLOSED") => ConnectorErrorType::BusinessError,
            ("14002", "AUTHORIZATION_AMOUNT_ALREADY_REVERSED") => ConnectorErrorType::BusinessError,
            ("14002", "AUTHORIZATION_AMOUNT_NOT_VALID") => ConnectorErrorType::BusinessError,
            ("14002", "AUTHORIZATION_EXPIRED") => ConnectorErrorType::BusinessError,
            ("14002", "AUTHORIZATION_REVOKED") => ConnectorErrorType::BusinessError,
            ("14002", "AUTHORIZATION_NOT_FOUND") => ConnectorErrorType::UserError,
            ("14002", "BLS_CONNECTION_PROBLEM") => ConnectorErrorType::BusinessError,
            ("14002", "CALL_ISSUER") => ConnectorErrorType::UnknownError,
            ("14002", "CARD_LOST_OR_STOLEN") => ConnectorErrorType::UserError,
            ("14002", "CVV_ERROR") => ConnectorErrorType::UserError,
            ("14002", "DO_NOT_HONOR") => ConnectorErrorType::TechnicalError,
            ("14002", "EXPIRED_CARD") => ConnectorErrorType::UserError,
            ("14002", "GENERAL_PAYMENT_PROCESSING_ERROR") => ConnectorErrorType::TechnicalError,
            ("14002", "HIGH_RISK_ERROR") => ConnectorErrorType::BusinessError,
            ("14002", "INCORRECT_INFORMATION") => ConnectorErrorType::BusinessError,
            ("14002", "INCORRECT_SETUP") => ConnectorErrorType::BusinessError,
            ("14002", "INSUFFICIENT_FUNDS") => ConnectorErrorType::UserError,
            ("14002", "INVALID_AMOUNT") => ConnectorErrorType::BusinessError,
            ("14002", "INVALID_CARD_NUMBER") => ConnectorErrorType::UserError,
            ("14002", "INVALID_CARD_TYPE") => ConnectorErrorType::BusinessError,
            ("14002", "INVALID_PIN_OR_PW_OR_ID_ERROR") => ConnectorErrorType::UserError,
            ("14002", "INVALID_TRANSACTION") => ConnectorErrorType::BusinessError,
            ("14002", "LIMIT_EXCEEDED") => ConnectorErrorType::TechnicalError,
            ("14002", "PICKUP_CARD") => ConnectorErrorType::UserError,
            ("14002", "PROCESSING_AMOUNT_ERROR") => ConnectorErrorType::BusinessError,
            ("14002", "PROCESSING_DUPLICATE") => ConnectorErrorType::BusinessError,
            ("14002", "PROCESSING_GENERAL_DECLINE") => ConnectorErrorType::TechnicalError,
            ("14002", "PROCESSING_TIMEOUT") => ConnectorErrorType::TechnicalError,
            ("14002", "REFUND_FAILED") => ConnectorErrorType::TechnicalError,
            ("14002", "RESTRICTED_CARD") => ConnectorErrorType::UserError,
            ("14002", "STRONG_CUSTOMER_AUTHENTICATION_REQUIRED") => ConnectorErrorType::UserError,
            ("14002", "SYSTEM_TECHNICAL_ERROR") => ConnectorErrorType::BusinessError,
            ("14002", "THE_ISSUER_IS_UNAVAILABLE_OR_OFFLINE") => ConnectorErrorType::TechnicalError,
            ("14002", "THREE_D_SECURE_FAILURE") => ConnectorErrorType::UserError,
            ("14010", "FAILED_CREATING_PAYPAL_TOKEN") => ConnectorErrorType::TechnicalError,
            ("14011", "PAYMENT_METHOD_NOT_SUPPORTED") => ConnectorErrorType::BusinessError,
            ("14016", "NO_AVAILABLE_PROCESSORS") => ConnectorErrorType::TechnicalError,
            ("14034", "INVALID_PAYMENT_DETAILS") => ConnectorErrorType::UserError,
            ("15008", "SHOPPER_NOT_FOUND") => ConnectorErrorType::BusinessError,
            ("15012", "SHOPPER_COUNTRY_OFAC_SANCTIONED") => ConnectorErrorType::BusinessError,
            ("16003", "MULTIPLE_PAYMENT_METHODS_NON_SELECTED") => ConnectorErrorType::BusinessError,
            ("16001", "MISSING_ARGUMENTS") => ConnectorErrorType::BusinessError,
            ("17005", "INVALID_STEP_FIELD") => ConnectorErrorType::BusinessError,
            ("20002", "MULTIPLE_TRANSACTIONS_FOUND") => ConnectorErrorType::BusinessError,
            ("20003", "TRANSACTION_LOCKED") => ConnectorErrorType::BusinessError,
            ("20004", "TRANSACTION_PAYMENT_METHOD_NOT_SUPPORTED") => {
                ConnectorErrorType::BusinessError
            }
            ("20005", "TRANSACTION_NOT_AUTHORIZED") => ConnectorErrorType::UserError,
            ("20006", "TRANSACTION_ALREADY_EXISTS") => ConnectorErrorType::BusinessError,
            ("20007", "TRANSACTION_EXPIRED") => ConnectorErrorType::UserError,
            ("20008", "TRANSACTION_ID_REQUIRED") => ConnectorErrorType::TechnicalError,
            ("20009", "INVALID_TRANSACTION_ID") => ConnectorErrorType::BusinessError,
            ("20010", "TRANSACTION_ALREADY_CAPTURED") => ConnectorErrorType::BusinessError,
            ("20017", "TRANSACTION_CARD_NOT_VALID") => ConnectorErrorType::UserError,
            ("20031", "MISSING_RELEVANT_METHOD_FOR_SHOPPER") => ConnectorErrorType::BusinessError,
            ("20020", "INVALID_ALT_TRANSACTION_TYPE") => ConnectorErrorType::BusinessError,
            ("20021", "MULTI_SHOPPER_INFORMATION") => ConnectorErrorType::BusinessError,
            ("20022", "MISSING_SHOPPER_INFORMATION") => ConnectorErrorType::UserError,
            ("20023", "MISSING_PAYER_INFO_FIELDS") => ConnectorErrorType::UserError,
            ("20024", "EXPECT_NO_ECP_DETAILS") => ConnectorErrorType::UserError,
            ("20025", "INVALID_ECP_ACCOUNT_TYPE") => ConnectorErrorType::UserError,
            ("20025", "INVALID_PAYER_INFO_FIELDS") => ConnectorErrorType::UserError,
            ("20026", "MISMATCH_SUBSCRIPTION_CURRENCY") => ConnectorErrorType::BusinessError,
            ("20027", "PAYPAL_UNSUPPORTED_CURRENCY") => ConnectorErrorType::UserError,
            ("20033", "IDEAL_UNSUPPORTED_PAYMENT_INFO") => ConnectorErrorType::BusinessError,
            ("20035", "SOFORT_UNSUPPORTED_PAYMENT_INFO") => ConnectorErrorType::BusinessError,
            ("23001", "MISSING_WALLET_FIELDS") => ConnectorErrorType::BusinessError,
            ("23002", "INVALID_WALLET_FIELDS") => ConnectorErrorType::UserError,
            ("23003", "WALLET_PROCESSING_FAILURE") => ConnectorErrorType::TechnicalError,
            ("23005", "WALLET_EXPIRED") => ConnectorErrorType::UserError,
            ("23006", "WALLET_DUPLICATE_PAYMENT_METHODS") => ConnectorErrorType::BusinessError,
            ("23007", "WALLET_PAYMENT_NOT_ENABLED") => ConnectorErrorType::BusinessError,
            ("23008", "DUPLICATE_WALLET_RESOURCE") => ConnectorErrorType::BusinessError,
            ("23009", "WALLET_CLIENT_KEY_FAILURE") => ConnectorErrorType::BusinessError,
            ("23010", "INVALID_WALLET_PAYMENT_DATA") => ConnectorErrorType::UserError,
            ("23011", "WALLET_ONBOARDING_ERROR") => ConnectorErrorType::BusinessError,
            ("23012", "WALLET_MISSING_DOMAIN") => ConnectorErrorType::UserError,
            ("23013", "WALLET_UNREGISTERED_DOMAIN") => ConnectorErrorType::BusinessError,
            ("23014", "WALLET_CHECKOUT_CANCELED") => ConnectorErrorType::UserError,
            ("24012", "USER_NOT_AUTHORIZED") => ConnectorErrorType::UserError,
            ("24011", "CURRENCY_CODE_NOT_FOUND") => ConnectorErrorType::UserError,
            ("90009", "SUBSCRIPTION_NOT_FOUND") => ConnectorErrorType::UserError,
            (_, " MISSING_ARGUMENTS") => ConnectorErrorType::UnknownError,
            ("43008", "EXTERNAL_TAX_SERVICE_MISMATCH_CURRENCY") => {
                ConnectorErrorType::BusinessError
            }
            ("43009", "EXTERNAL_TAX_SERVICE_UNEXPECTED_TOTAL_PAYMENT") => {
                ConnectorErrorType::BusinessError
            }
            ("43010", "EXTERNAL_TAX_SERVICE_TAX_REFERENCE_ALREADY_USED") => {
                ConnectorErrorType::BusinessError
            }
            (
                _,
                "USER_NOT_AUTHORIZED"
                | "CREDIT_CARD_DETAILS_PLAIN_AND_ENCRYPTED"
                | "CREDIT_CARD_ENCRYPTED_SECURITY_CODE_REQUIRED"
                | "CREDIT_CARD_ENCRYPTED_NUMBER_REQUIRED",
            ) => ConnectorErrorType::UserError,
            _ => ConnectorErrorType::UnknownError,
        }
    }
}
