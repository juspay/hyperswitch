pub mod transformers;

use std::{collections::HashMap, fmt::Debug, ops::Deref};

use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use router_env::{instrument, tracing};

use self::transformers as stripe;
use super::utils::{self as connector_utils, RefundsRequestData};
use crate::{
    configs::settings,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger,
    services::{
        self,
        request::{self, Mask},
        ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon},
    },
    utils::{self, crypto, ByteSliceExt, BytesExt, OptionExt},
};

#[derive(Debug, Clone)]
pub struct Stripe;

impl ConnectorCommon for Stripe {
    fn id(&self) -> &'static str {
        "stripe"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        // &self.base_url
        connectors.stripe.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: stripe::StripeAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.api_key.peek()).into_masked(),
        )])
    }
}

impl ConnectorValidation for Stripe {
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

impl api::Payment for Stripe {}

impl api::PaymentAuthorize for Stripe {}
impl api::PaymentSync for Stripe {}
impl api::PaymentVoid for Stripe {}
impl api::PaymentCapture for Stripe {}
impl api::PaymentSession for Stripe {}
impl api::ConnectorAccessToken for Stripe {}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Stripe
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Stripe
{
    // Not Implemented (R)
}

impl api::PaymentsPreProcessing for Stripe {}

impl
    services::ConnectorIntegration<
        api::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
    > for Stripe
{
    fn get_headers(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsPreProcessingType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "v1/sources"))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req = stripe::StripeCreditTransferSourceRequest::try_from(req)?;
        let pre_processing_request = types::RequestBody::log_and_get_request_body(
            &req,
            utils::Encode::<stripe::StripeCreditTransferSourceRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(pre_processing_request))
    }

    fn build_request(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsPreProcessingType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsPreProcessingType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsPreProcessingType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsPreProcessingRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsPreProcessingRouterData, errors::ConnectorError> {
        let response: stripe::StripeSourceResponse = res
            .response
            .parse_struct("StripeSourceResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(connector_response=?response);

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
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

impl api::ConnectorCustomer for Stripe {}

impl
    services::ConnectorIntegration<
        api::CreateConnectorCustomer,
        types::ConnectorCustomerData,
        types::PaymentsResponseData,
    > for Stripe
{
    fn get_headers(
        &self,
        req: &types::ConnectorCustomerRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::ConnectorCustomerType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::ConnectorCustomerRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "v1/customers"))
    }

    fn get_request_body(
        &self,
        req: &types::ConnectorCustomerRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = stripe::CustomerRequest::try_from(req)?;
        let stripe_req = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<stripe::CustomerRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(stripe_req))
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
        res: types::Response,
    ) -> CustomResult<types::ConnectorCustomerRouterData, errors::ConnectorError>
    where
        types::PaymentsResponseData: Clone,
    {
        let response: stripe::StripeCustomerResponse = res
            .response
            .parse_struct("StripeCustomerResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(connector_response=?response);

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
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);

        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

impl api::PaymentToken for Stripe {}

impl
    services::ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Stripe
{
    fn get_headers(
        &self,
        req: &types::TokenizationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::TokenizationType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
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
        Ok(format!("{}{}", self.base_url(connectors), "v1/tokens"))
    }

    fn get_request_body(
        &self,
        req: &types::TokenizationRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = stripe::TokenRequest::try_from(req)?;
        let stripe_req = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<stripe::TokenRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(stripe_req))
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
        let response: stripe::StripeTokenResponse = res
            .response
            .parse_struct("StripeTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(connector_response=?response);

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
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);

        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

impl api::MandateSetup for Stripe {}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Stripe
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            Self::common_get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,

        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.as_str();

        Ok(format!(
            "{}{}/{}/capture",
            self.base_url(connectors),
            "v1/payment_intents",
            id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = stripe::CaptureRequest::try_from(req)?;
        let stripe_req = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<stripe::CaptureRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(stripe_req))
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
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError>
    where
        types::PaymentsCaptureData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: stripe::PaymentIntentResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(connector_response=?response);
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
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);

        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Stripe
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsSyncType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.clone();

        match id.get_connector_transaction_id() {
            Ok(x) if x.starts_with("set") => Ok(format!(
                "{}{}/{}",
                self.base_url(connectors),
                "v1/setup_intents",
                x
            )),
            Ok(x) => Ok(format!(
                "{}{}/{}{}",
                self.base_url(connectors),
                "v1/payment_intents",
                x,
                "?expand[0]=latest_charge" //updated payment_id(if present) reside inside latest_charge field
            )),
            x => x.change_context(errors::ConnectorError::MissingConnectorTransactionID),
        }
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
        types::PaymentsAuthorizeData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let id = data.request.connector_transaction_id.clone();
        match id.get_connector_transaction_id() {
            Ok(x) if x.starts_with("set") => {
                let response: stripe::SetupIntentResponse = res
                    .response
                    .parse_struct("SetupIntentSyncResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            Ok(_) => {
                let response: stripe::PaymentIntentSyncResponse = res
                    .response
                    .parse_struct("PaymentIntentSyncResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            Err(err) => {
                Err(err).change_context(errors::ConnectorError::MissingConnectorTransactionID)
            }
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);

        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

#[async_trait::async_trait]
impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Stripe
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
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

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        match &req.request.payment_method_data {
            api_models::payments::PaymentMethodData::BankTransfer(bank_transfer_data) => {
                match bank_transfer_data.deref() {
                    api_models::payments::BankTransferData::AchBankTransfer { .. }
                    | api_models::payments::BankTransferData::MultibancoBankTransfer { .. } => {
                        Ok(format!("{}{}", self.base_url(connectors), "v1/charges"))
                    }
                    _ => Ok(format!(
                        "{}{}",
                        self.base_url(connectors),
                        "v1/payment_intents"
                    )),
                }
            }
            _ => Ok(format!(
                "{}{}",
                self.base_url(connectors),
                "v1/payment_intents"
            )),
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        match &req.request.payment_method_data {
            api_models::payments::PaymentMethodData::BankTransfer(bank_transfer_data) => {
                stripe::get_bank_transfer_request_data(req, bank_transfer_data.deref())
            }
            _ => {
                let req = stripe::PaymentIntentRequest::try_from(req)?;
                let request = types::RequestBody::log_and_get_request_body(
                    &req,
                    utils::Encode::<stripe::PaymentIntentRequest>::url_encode,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                Ok(Some(request))
            }
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

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        match &data.request.payment_method_data {
            api_models::payments::PaymentMethodData::BankTransfer(bank_transfer_data) => {
                stripe::get_bank_transfer_authorize_response(data, res, bank_transfer_data.deref())
            }
            _ => {
                let response: stripe::PaymentIntentResponse = res
                    .response
                    .parse_struct("PaymentIntentResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                logger::info!(connector_response=?response);

                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
                .change_context(errors::ConnectorError::ResponseHandlingFailed)
            }
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Stripe
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsVoidType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_id = &req.request.connector_transaction_id;
        Ok(format!(
            "{}v1/payment_intents/{}/cancel",
            self.base_url(connectors),
            payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = stripe::CancelRequest::try_from(req)?;
        let stripe_req = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<stripe::CancelRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(stripe_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
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
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: stripe::PaymentIntentResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(connector_response=?response);
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
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

type Verify = dyn services::ConnectorIntegration<
    api::SetupMandate,
    types::SetupMandateRequestData,
    types::PaymentsResponseData,
>;
impl
    services::ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Stripe
{
    fn get_headers(
        &self,
        req: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            Verify::get_content_type(self).to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "v1/setup_intents"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req = stripe::SetupIntentRequest::try_from(req)?;
        let stripe_req = types::RequestBody::log_and_get_request_body(
            &req,
            utils::Encode::<stripe::SetupIntentRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(stripe_req))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&Verify::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(Verify::get_headers(self, req, connectors)?)
                .body(Verify::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        errors::ConnectorError,
    >
    where
        api::SetupMandate: Clone,
        types::SetupMandateRequestData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: stripe::SetupIntentResponse = res
            .response
            .parse_struct("SetupIntentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(connector_response=?response);
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
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

impl api::Refund for Stripe {}
impl api::RefundExecute for Stripe {}
impl api::RefundSync for Stripe {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Stripe
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::RefundExecuteType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "v1/refunds"))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = stripe::RefundRequest::try_from(req)?;
        let stripe_req = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<stripe::RefundRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(stripe_req))
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

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: stripe::RefundResponse =
            res.response
                .parse_struct("Stripe RefundResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(connector_response=?response);
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
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Stripe
{
    fn get_headers(
        &self,
        req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::RefundSyncType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.get_connector_refund_id()?;
        Ok(format!("{}v1/refunds/{}", self.base_url(connectors), id))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::RSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::RSync>,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        errors::ConnectorError,
    > {
        let response: stripe::RefundResponse =
            res.response
                .parse_struct("Stripe RefundResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(connector_response=?response);
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
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

impl api::UploadFile for Stripe {}

#[async_trait::async_trait]
impl api::FileUpload for Stripe {
    fn validate_file_upload(
        &self,
        purpose: api::FilePurpose,
        file_size: i32,
        file_type: mime::Mime,
    ) -> CustomResult<(), errors::ConnectorError> {
        match purpose {
            api::FilePurpose::DisputeEvidence => {
                let supported_file_types = ["image/jpeg", "image/png", "application/pdf"];
                // 5 Megabytes (MB)
                if file_size > 5000000 {
                    Err(errors::ConnectorError::FileValidationFailed {
                        reason: "file_size exceeded the max file size of 5MB".to_owned(),
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

impl
    services::ConnectorIntegration<
        api::Upload,
        types::UploadFileRequestData,
        types::UploadFileResponse,
    > for Stripe
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
        Ok(format!(
            "{}{}",
            connectors.stripe.base_url_file_upload, "v1/files"
        ))
    }

    fn get_request_form_data(
        &self,
        req: &types::UploadFileRouterData,
    ) -> CustomResult<Option<reqwest::multipart::Form>, errors::ConnectorError> {
        let stripe_req = transformers::construct_file_upload_request(req.clone())?;
        Ok(Some(stripe_req))
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

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::UploadFileRouterData,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<api::Upload, types::UploadFileRequestData, types::UploadFileResponse>,
        errors::ConnectorError,
    > {
        let response: stripe::FileUploadResponse = res
            .response
            .parse_struct("Stripe FileUploadResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(connector_response=?response);
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
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

impl api::RetrieveFile for Stripe {}

impl
    services::ConnectorIntegration<
        api::Retrieve,
        types::RetrieveFileRequestData,
        types::RetrieveFileResponse,
    > for Stripe
{
    fn get_headers(
        &self,
        req: &types::RouterData<
            api::Retrieve,
            types::RetrieveFileRequestData,
            types::RetrieveFileResponse,
        >,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_url(
        &self,
        req: &types::RetrieveFileRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}v1/files/{}/contents",
            connectors.stripe.base_url_file_upload, req.request.provider_file_id
        ))
    }

    fn build_request(
        &self,
        req: &types::RetrieveFileRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RetrieveFileType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RetrieveFileType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::RetrieveFileRouterData,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<
            api::Retrieve,
            types::RetrieveFileRequestData,
            types::RetrieveFileResponse,
        >,
        errors::ConnectorError,
    > {
        let response = res.response;
        Ok(types::RetrieveFileRouterData {
            response: Ok(types::RetrieveFileResponse {
                file_data: response.to_vec(),
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

impl api::SubmitEvidence for Stripe {}

impl
    services::ConnectorIntegration<
        api::Evidence,
        types::SubmitEvidenceRequestData,
        types::SubmitEvidenceResponse,
    > for Stripe
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

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        req: &types::SubmitEvidenceRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}{}",
            self.base_url(connectors),
            "v1/disputes/",
            req.request.connector_dispute_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::SubmitEvidenceRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let stripe_req = stripe::Evidence::try_from(req)?;
        let stripe_req_string = types::RequestBody::log_and_get_request_body(
            &stripe_req,
            utils::Encode::<stripe::Evidence>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(stripe_req_string))
    }

    fn build_request(
        &self,
        req: &types::SubmitEvidenceRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::SubmitEvidenceType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::SubmitEvidenceType::get_headers(
                self, req, connectors,
            )?)
            .body(types::SubmitEvidenceType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::SubmitEvidenceRouterData,
        res: types::Response,
    ) -> CustomResult<types::SubmitEvidenceRouterData, errors::ConnectorError> {
        let response: stripe::DisputeObj = res
            .response
            .parse_struct("Stripe DisputeObj")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(connector_response=?response);
        Ok(types::SubmitEvidenceRouterData {
            response: Ok(types::SubmitEvidenceResponse {
                dispute_status: api_models::enums::DisputeStatus::DisputeChallenged,
                connector_status: Some(response.status),
            }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: stripe::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(error_response=?response);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message.map(|message| {
                response
                    .error
                    .decline_code
                    .map(|decline_code| {
                        format!("message - {}, decline_code - {}", message, decline_code)
                    })
                    .unwrap_or(message)
            }),
        })
    }
}

fn get_signature_elements_from_header(
    headers: &actix_web::http::header::HeaderMap,
) -> CustomResult<HashMap<String, Vec<u8>>, errors::ConnectorError> {
    let security_header = headers
        .get("Stripe-Signature")
        .map(|header_value| {
            header_value
                .to_str()
                .map(String::from)
                .map_err(|_| errors::ConnectorError::WebhookSignatureNotFound)
                .into_report()
        })
        .ok_or(errors::ConnectorError::WebhookSignatureNotFound)
        .into_report()??;

    let props = security_header.split(',').collect::<Vec<&str>>();
    let mut security_header_kvs: HashMap<String, Vec<u8>> = HashMap::with_capacity(props.len());

    for prop_str in &props {
        let (prop_key, prop_value) = prop_str
            .split_once('=')
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)
            .into_report()?;

        security_header_kvs.insert(prop_key.to_string(), prop_value.bytes().collect());
    }

    Ok(security_header_kvs)
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Stripe {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let mut security_header_kvs = get_signature_elements_from_header(request.headers)?;

        let signature = security_header_kvs
            .remove("v1")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)
            .into_report()?;

        hex::decode(signature)
            .into_report()
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let mut security_header_kvs = get_signature_elements_from_header(request.headers)?;

        let timestamp = security_header_kvs
            .remove("t")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)
            .into_report()?;

        Ok(format!(
            "{}.{}",
            String::from_utf8_lossy(&timestamp),
            String::from_utf8_lossy(request.body)
        )
        .into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: stripe::WebhookEvent = request
            .body
            .parse_struct("WebhookEvent")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        Ok(match details.event_data.event_object.object {
            stripe::WebhookEventObjectType::PaymentIntent => {
                match details.event_data.event_object.metadata {
                    // if order_id is present
                    Some(meta_data) => api_models::webhooks::ObjectReferenceId::PaymentId(
                        api_models::payments::PaymentIdType::PaymentAttemptId(meta_data.order_id),
                    ),
                    // else used connector_transaction_id
                    None => api_models::webhooks::ObjectReferenceId::PaymentId(
                        api_models::payments::PaymentIdType::ConnectorTransactionId(
                            details.event_data.event_object.id,
                        ),
                    ),
                }
            }
            stripe::WebhookEventObjectType::Charge => {
                match details.event_data.event_object.metadata {
                    // if order_id is present
                    Some(meta_data) => api_models::webhooks::ObjectReferenceId::PaymentId(
                        api_models::payments::PaymentIdType::PaymentAttemptId(meta_data.order_id),
                    ),
                    // else used connector_transaction_id
                    None => api_models::webhooks::ObjectReferenceId::PaymentId(
                        api_models::payments::PaymentIdType::ConnectorTransactionId(
                            details
                                .event_data
                                .event_object
                                .payment_intent
                                .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                        ),
                    ),
                }
            }
            stripe::WebhookEventObjectType::Dispute => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        details
                            .event_data
                            .event_object
                            .payment_intent
                            .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                    ),
                )
            }
            stripe::WebhookEventObjectType::Source => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::PreprocessingId(
                        details.event_data.event_object.id,
                    ),
                )
            }
            stripe::WebhookEventObjectType::Refund => {
                match details.event_data.event_object.metadata {
                    // if meta_data is present
                    Some(meta_data) => {
                        // Issue: 2076
                        match meta_data.is_refund_id_as_reference {
                            // if the order_id is refund_id
                            Some(_) => api_models::webhooks::ObjectReferenceId::RefundId(
                                api_models::webhooks::RefundIdType::RefundId(meta_data.order_id),
                            ),
                            // if the order_id is payment_id
                            // since payment_id was being passed before the deployment of this pr
                            _ => api_models::webhooks::ObjectReferenceId::RefundId(
                                api_models::webhooks::RefundIdType::ConnectorRefundId(
                                    details.event_data.event_object.id,
                                ),
                            ),
                        }
                    }
                    // else use connector_transaction_id
                    None => api_models::webhooks::ObjectReferenceId::RefundId(
                        api_models::webhooks::RefundIdType::ConnectorRefundId(
                            details.event_data.event_object.id,
                        ),
                    ),
                }
            }
        })
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let details: stripe::WebhookEventTypeBody = request
            .body
            .parse_struct("WebhookEventTypeBody")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        Ok(match details.event_type {
            stripe::WebhookEventType::PaymentIntentFailed => {
                api::IncomingWebhookEvent::PaymentIntentFailure
            }
            stripe::WebhookEventType::PaymentIntentSucceed => {
                api::IncomingWebhookEvent::PaymentIntentSuccess
            }
            stripe::WebhookEventType::PaymentIntentCanceled => {
                api::IncomingWebhookEvent::PaymentIntentCancelled
            }
            stripe::WebhookEventType::ChargeSucceeded => {
                if let Some(stripe::WebhookPaymentMethodDetails {
                    payment_method:
                        stripe::WebhookPaymentMethodType::AchCreditTransfer
                        | stripe::WebhookPaymentMethodType::MultibancoBankTransfers,
                }) = details.event_data.event_object.payment_method_details
                {
                    api::IncomingWebhookEvent::PaymentIntentSuccess
                } else {
                    api::IncomingWebhookEvent::EventNotSupported
                }
            }
            stripe::WebhookEventType::ChargeRefundUpdated => details
                .event_data
                .event_object
                .status
                .map(|status| match status {
                    stripe::WebhookEventStatus::Succeeded => {
                        api::IncomingWebhookEvent::RefundSuccess
                    }
                    stripe::WebhookEventStatus::Failed => api::IncomingWebhookEvent::RefundFailure,
                    _ => api::IncomingWebhookEvent::EventNotSupported,
                })
                .unwrap_or(api::IncomingWebhookEvent::EventNotSupported),
            stripe::WebhookEventType::SourceChargeable => {
                api::IncomingWebhookEvent::SourceChargeable
            }
            stripe::WebhookEventType::DisputeCreated => api::IncomingWebhookEvent::DisputeOpened,
            stripe::WebhookEventType::DisputeClosed => api::IncomingWebhookEvent::DisputeCancelled,
            stripe::WebhookEventType::DisputeUpdated => details
                .event_data
                .event_object
                .status
                .map(Into::into)
                .unwrap_or(api::IncomingWebhookEvent::EventNotSupported),
            stripe::WebhookEventType::PaymentIntentPartiallyFunded => {
                api::IncomingWebhookEvent::PaymentIntentPartiallyFunded
            }
            stripe::WebhookEventType::PaymentIntentRequiresAction => {
                api::IncomingWebhookEvent::PaymentActionRequired
            }
            stripe::WebhookEventType::ChargeDisputeFundsWithdrawn => {
                api::IncomingWebhookEvent::DisputeLost
            }
            stripe::WebhookEventType::ChargeDisputeFundsReinstated => {
                api::IncomingWebhookEvent::DisputeWon
            }
            stripe::WebhookEventType::Unknown
            | stripe::WebhookEventType::ChargeCaptured
            | stripe::WebhookEventType::ChargeExpired
            | stripe::WebhookEventType::ChargeFailed
            | stripe::WebhookEventType::ChargePending
            | stripe::WebhookEventType::ChargeUpdated
            | stripe::WebhookEventType::ChargeRefunded
            | stripe::WebhookEventType::PaymentIntentCreated
            | stripe::WebhookEventType::PaymentIntentProcessing
            | stripe::WebhookEventType::PaymentIntentAmountCapturableUpdated
            | stripe::WebhookEventType::SourceTransactionCreated => {
                api::IncomingWebhookEvent::EventNotSupported
            }
        })
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let details: stripe::WebhookEventObjectResource = request
            .body
            .parse_struct("WebhookEventObjectResource")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(details.data.object)
    }
    fn get_dispute_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::disputes::DisputePayload, errors::ConnectorError> {
        let details: stripe::WebhookEvent = request
            .body
            .parse_struct("WebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(api::disputes::DisputePayload {
            amount: details
                .event_data
                .event_object
                .amount
                .get_required_value("amount")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "amount",
                })?
                .to_string(),
            currency: details.event_data.event_object.currency,
            dispute_stage: api_models::enums::DisputeStage::Dispute,
            connector_dispute_id: details.event_data.event_object.id,
            connector_reason: details.event_data.event_object.reason,
            connector_reason_code: None,
            challenge_required_by: details
                .event_data
                .event_object
                .evidence_details
                .map(|payload| payload.due_by),
            connector_status: details
                .event_data
                .event_object
                .status
                .ok_or(errors::ConnectorError::WebhookResourceObjectNotFound)?
                .to_string(),
            created_at: Some(details.event_data.event_object.created),
            updated_at: None,
        })
    }
}

impl services::ConnectorRedirectResponse for Stripe {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        action: services::PaymentAction,
    ) -> CustomResult<crate::core::payments::CallConnectorAction, errors::ConnectorError> {
        match action {
            services::PaymentAction::PSync | services::PaymentAction::CompleteAuthorize => {
                Ok(payments::CallConnectorAction::Trigger)
            }
        }
    }
}
