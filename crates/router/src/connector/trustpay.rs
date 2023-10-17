pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_utils::{crypto, errors::ReportSwitchExt, ext_traits::ByteSliceExt};
use error_stack::{IntoReport, Report, ResultExt};
use masking::PeekInterface;
use transformers as trustpay;

use super::utils::{
    collect_and_sort_values_by_removing_signature, get_error_code_error_message_based_on_priority,
    ConnectorErrorType, ConnectorErrorTypeMapping, PaymentsPreProcessingData,
};
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
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Trustpay;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Trustpay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        match req.payment_method {
            diesel_models::enums::PaymentMethod::BankRedirect => {
                let token = req
                    .access_token
                    .clone()
                    .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
                Ok(vec![
                    (
                        headers::CONTENT_TYPE.to_string(),
                        "application/json".to_owned().into(),
                    ),
                    (
                        headers::AUTHORIZATION.to_string(),
                        format!("Bearer {}", token.token.peek()).into_masked(),
                    ),
                ])
            }
            _ => {
                let mut header = vec![(
                    headers::CONTENT_TYPE.to_string(),
                    self.get_content_type().to_string().into(),
                )];
                let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
                header.append(&mut api_key);
                Ok(header)
            }
        }
    }
}

impl ConnectorCommon for Trustpay {
    fn id(&self) -> &'static str {
        "trustpay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.trustpay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = trustpay::TrustpayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::X_API_KEY.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: Result<
            trustpay::TrustpayErrorResponse,
            Report<common_utils::errors::ParsingError>,
        > = res.response.parse_struct("trustpay ErrorResponse");

        match response {
            Ok(response_data) => {
                let error_list = response_data.errors.clone().unwrap_or_default();
                let option_error_code_message = get_error_code_error_message_based_on_priority(
                    self.clone(),
                    error_list.into_iter().map(|errors| errors.into()).collect(),
                );
                let reason = response_data.errors.map(|errors| {
                    errors
                        .iter()
                        .map(|error| error.description.clone())
                        .collect::<Vec<String>>()
                        .join(" & ")
                });
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: option_error_code_message
                        .clone()
                        .map(|error_code_message| error_code_message.error_code)
                        .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                    // message vary for the same code, so relying on code alone as it is unique
                    message: option_error_code_message
                        .map(|error_code_message| error_code_message.error_code)
                        .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    reason: reason.or(response_data.description),
                })
            }
            Err(error_msg) => {
                logger::error!(deserialization_error =? error_msg);
                utils::handle_json_response_deserialization_failure(res, "trustpay".to_owned())
            }
        }
    }
}

impl ConnectorValidation for Trustpay {
    fn validate_if_surcharge_implemented(&self) -> CustomResult<(), errors::ConnectorError> {
        Ok(())
    }
}

impl api::Payment for Trustpay {}

impl api::PaymentToken for Trustpay {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Trustpay
{
    // Not Implemented (R)
}

impl api::MandateSetup for Trustpay {}
impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Trustpay
{
}

impl api::PaymentVoid for Trustpay {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Trustpay
{
}

impl api::ConnectorAccessToken for Trustpay {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Trustpay
{
    fn get_url(
        &self,
        _req: &types::RefreshTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            connectors.trustpay.base_url_bank_redirects, "api/oauth2/token"
        ))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_headers(
        &self,
        req: &types::RefreshTokenRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = trustpay::TrustpayAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_value = auth
            .project_id
            .zip(auth.secret_key)
            .map(|(project_id, secret_key)| {
                format!(
                    "Basic {}",
                    consts::BASE64_ENGINE.encode(format!("{}:{}", project_id, secret_key))
                )
            });
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefreshTokenType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (headers::AUTHORIZATION.to_string(), auth_value.into_masked()),
        ])
    }

    fn get_request_body(
        &self,
        req: &types::RefreshTokenRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = trustpay::TrustpayAuthUpdateRequest::try_from(req)?;
        let trustpay_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<trustpay::TrustpayAuthUpdateRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(trustpay_req))
    }

    fn build_request(
        &self,
        req: &types::RefreshTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let req = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .attach_default_headers()
                .headers(types::RefreshTokenType::get_headers(self, req, connectors)?)
                .url(&types::RefreshTokenType::get_url(self, req, connectors)?)
                .body(types::RefreshTokenType::get_request_body(self, req)?)
                .build(),
        );
        Ok(req)
    }

    fn handle_response(
        &self,
        data: &types::RefreshTokenRouterData,
        res: Response,
    ) -> CustomResult<types::RefreshTokenRouterData, errors::ConnectorError> {
        let response: trustpay::TrustpayAuthUpdateResponse = res
            .response
            .parse_struct("trustpay TrustpayAuthUpdateResponse")
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: trustpay::TrustpayAccessTokenErrorResponse = res
            .response
            .parse_struct("Trustpay AccessTokenErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.result_info.result_code.to_string(),
            // message vary for the same code, so relying on code alone as it is unique
            message: response.result_info.result_code.to_string(),
            reason: response.result_info.additional_info,
        })
    }
}

impl api::PaymentSync for Trustpay {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Trustpay
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
        let id = req.request.connector_transaction_id.clone();
        match req.payment_method {
            diesel_models::enums::PaymentMethod::BankRedirect => Ok(format!(
                "{}{}/{}",
                connectors.trustpay.base_url_bank_redirects,
                "api/Payments/Payment",
                id.get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
            )),
            _ => Ok(format!(
                "{}{}/{}",
                self.base_url(connectors),
                "api/v1/instance",
                id.get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
            )),
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
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: trustpay::TrustPayTransactionStatusErrorResponse = res
            .response
            .parse_struct("trustpay transaction status ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status.to_string(),
            // message vary for the same code, so relying on code alone as it is unique
            message: response.status.to_string(),
            reason: Some(response.payment_description),
        })
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: trustpay::TrustpayPaymentsResponse = res
            .response
            .parse_struct("trustpay PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::PaymentCapture for Trustpay {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Trustpay
{
}

impl api::PaymentsPreProcessing for Trustpay {}

impl
    ConnectorIntegration<
        api::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
    > for Trustpay
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
        Ok(format!("{}{}", self.base_url(connectors), "api/v1/intent"))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let currency = req.request.get_currency()?;
        let amount = req
            .request
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.final_amount)
            .unwrap_or(req.request.get_amount()?);
        let connector_router_data = trustpay::TrustpayRouterData::try_from((
            &self.get_currency_unit(),
            currency,
            amount,
            req,
        ))?;
        let create_intent_req =
            trustpay::TrustpayCreateIntentRequest::try_from(&connector_router_data)?;
        let trustpay_req = types::RequestBody::log_and_get_request_body(
            &create_intent_req,
            utils::Encode::<trustpay::TrustpayCreateIntentRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(trustpay_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let req = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .attach_default_headers()
                .headers(types::PaymentsPreProcessingType::get_headers(
                    self, req, connectors,
                )?)
                .url(&types::PaymentsPreProcessingType::get_url(
                    self, req, connectors,
                )?)
                .body(types::PaymentsPreProcessingType::get_request_body(
                    self, req,
                )?)
                .build(),
        );
        Ok(req)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsPreProcessingRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsPreProcessingRouterData, errors::ConnectorError> {
        let response: trustpay::TrustpayCreateIntentResponse = res
            .response
            .parse_struct("TrustpayCreateIntentResponse")
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Trustpay {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Trustpay
{
}

impl api::PaymentAuthorize for Trustpay {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Trustpay
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
        match req.payment_method {
            diesel_models::enums::PaymentMethod::BankRedirect => Ok(format!(
                "{}{}",
                connectors.trustpay.base_url_bank_redirects, "api/Payments/Payment"
            )),
            _ => Ok(format!(
                "{}{}",
                self.base_url(connectors),
                "api/v1/purchase"
            )),
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let amount = req
            .request
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.final_amount)
            .unwrap_or(req.request.amount);
        let connector_router_data = trustpay::TrustpayRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            amount,
            req,
        ))?;
        let connector_req = trustpay::TrustpayPaymentsRequest::try_from(&connector_router_data)?;
        let trustpay_req_string = match req.payment_method {
            diesel_models::enums::PaymentMethod::BankRedirect => {
                types::RequestBody::log_and_get_request_body(
                    &connector_req,
                    utils::Encode::<trustpay::PaymentRequestBankRedirect>::encode_to_string_of_json,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
            }
            _ => types::RequestBody::log_and_get_request_body(
                &connector_req,
                utils::Encode::<trustpay::PaymentRequestCards>::url_encode,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?,
        };
        Ok(Some(trustpay_req_string))
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
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: trustpay::TrustpayPaymentsResponse = res
            .response
            .parse_struct("trustpay PaymentsResponse")
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Trustpay {}
impl api::RefundExecute for Trustpay {}
impl api::RefundSync for Trustpay {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Trustpay
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
        match req.payment_method {
            diesel_models::enums::PaymentMethod::BankRedirect => Ok(format!(
                "{}{}{}{}",
                connectors.trustpay.base_url_bank_redirects,
                "api/Payments/Payment/",
                req.request.connector_transaction_id,
                "/Refund"
            )),
            _ => Ok(format!("{}{}", self.base_url(connectors), "api/v1/Reverse")),
        }
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = trustpay::TrustpayRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req = trustpay::TrustpayRefundRequest::try_from(&connector_router_data)?;
        let trustpay_req_string = match req.payment_method {
            diesel_models::enums::PaymentMethod::BankRedirect => {
                types::RequestBody::log_and_get_request_body(
                    &connector_req,
                    utils::Encode::<trustpay::TrustpayRefundRequestBankRedirect>::encode_to_string_of_json,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
            }
            _ =>
                types::RequestBody::log_and_get_request_body(
                    &connector_req,
                    utils::Encode::<trustpay::TrustpayRefundRequestCards>::url_encode,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
        };
        Ok(Some(trustpay_req_string))
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
        let response: trustpay::RefundResponse = res
            .response
            .parse_struct("trustpay RefundResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
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

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Trustpay {
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
        let id = req
            .request
            .connector_refund_id
            .to_owned()
            .ok_or(errors::ConnectorError::MissingConnectorRefundID)?;
        match req.payment_method {
            diesel_models::enums::PaymentMethod::BankRedirect => Ok(format!(
                "{}{}/{}",
                connectors.trustpay.base_url_bank_redirects, "api/Payments/Payment", id
            )),
            _ => Ok(format!(
                "{}{}/{}",
                self.base_url(connectors),
                "api/v1/instance",
                id
            )),
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
        let response: trustpay::RefundResponse = res
            .response
            .parse_struct("trustpay RefundResponse")
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Trustpay {
    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: trustpay::TrustpayWebhookResponse = request
            .body
            .parse_struct("TrustpayWebhookResponse")
            .switch()?;
        match details.payment_information.credit_debit_indicator {
            trustpay::CreditDebitIndicator::Crdt => {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::PaymentAttemptId(
                        details.payment_information.references.merchant_reference,
                    ),
                ))
            }
            trustpay::CreditDebitIndicator::Dbit => {
                if details.payment_information.status == trustpay::WebhookStatus::Chargebacked {
                    Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                        api_models::payments::PaymentIdType::PaymentAttemptId(
                            details.payment_information.references.merchant_reference,
                        ),
                    ))
                } else {
                    Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                        api_models::webhooks::RefundIdType::RefundId(
                            details.payment_information.references.merchant_reference,
                        ),
                    ))
                }
            }
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let response: trustpay::TrustpayWebhookResponse = request
            .body
            .parse_struct("TrustpayWebhookResponse")
            .switch()?;
        match (
            response.payment_information.credit_debit_indicator,
            response.payment_information.status,
        ) {
            (trustpay::CreditDebitIndicator::Crdt, trustpay::WebhookStatus::Paid) => {
                Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess)
            }
            (trustpay::CreditDebitIndicator::Crdt, trustpay::WebhookStatus::Rejected) => {
                Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure)
            }
            (trustpay::CreditDebitIndicator::Dbit, trustpay::WebhookStatus::Paid) => {
                Ok(api_models::webhooks::IncomingWebhookEvent::RefundSuccess)
            }
            (trustpay::CreditDebitIndicator::Dbit, trustpay::WebhookStatus::Refunded) => {
                Ok(api_models::webhooks::IncomingWebhookEvent::RefundSuccess)
            }
            (trustpay::CreditDebitIndicator::Dbit, trustpay::WebhookStatus::Rejected) => {
                Ok(api_models::webhooks::IncomingWebhookEvent::RefundFailure)
            }
            (trustpay::CreditDebitIndicator::Dbit, trustpay::WebhookStatus::Chargebacked) => {
                Ok(api_models::webhooks::IncomingWebhookEvent::DisputeLost)
            }

            (
                trustpay::CreditDebitIndicator::Dbit | trustpay::CreditDebitIndicator::Crdt,
                trustpay::WebhookStatus::Unknown,
            ) => Ok(api::IncomingWebhookEvent::EventNotSupported),
            (trustpay::CreditDebitIndicator::Crdt, trustpay::WebhookStatus::Refunded) => {
                Ok(api::IncomingWebhookEvent::EventNotSupported)
            }
            (trustpay::CreditDebitIndicator::Crdt, trustpay::WebhookStatus::Chargebacked) => {
                Ok(api::IncomingWebhookEvent::EventNotSupported)
            }
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let details: trustpay::TrustpayWebhookResponse = request
            .body
            .parse_struct("TrustpayWebhookResponse")
            .switch()?;
        let res_json = utils::Encode::<trustpay::WebhookPaymentInformation>::encode_to_value(
            &details.payment_information,
        )
        .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(res_json)
    }

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
        let response: trustpay::TrustpayWebhookResponse = request
            .body
            .parse_struct("TrustpayWebhookResponse")
            .switch()?;
        hex::decode(response.signature)
            .into_report()
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let trustpay_response: trustpay::TrustpayWebhookResponse = request
            .body
            .parse_struct("TrustpayWebhookResponse")
            .switch()?;
        let response: serde_json::Value = request.body.parse_struct("Webhook Value").switch()?;
        let values =
            collect_and_sort_values_by_removing_signature(&response, &trustpay_response.signature);
        let payload = values.join("/");
        Ok(payload.into_bytes())
    }

    fn get_dispute_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::disputes::DisputePayload, errors::ConnectorError> {
        let trustpay_response: trustpay::TrustpayWebhookResponse = request
            .body
            .parse_struct("TrustpayWebhookResponse")
            .switch()?;
        let payment_info = trustpay_response.payment_information;
        let reason = payment_info.status_reason_information.unwrap_or_default();
        Ok(api::disputes::DisputePayload {
            amount: payment_info.amount.amount.to_string(),
            currency: payment_info.amount.currency,
            dispute_stage: api_models::enums::DisputeStage::Dispute,
            connector_dispute_id: payment_info.references.payment_id,
            connector_reason: reason.reason.reject_reason,
            connector_reason_code: Some(reason.reason.code),
            challenge_required_by: None,
            connector_status: payment_info.status.to_string(),
            created_at: None,
            updated_at: None,
        })
    }
}

impl services::ConnectorRedirectResponse for Trustpay {
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

impl ConnectorErrorTypeMapping for Trustpay {
    fn get_connector_error_type(
        &self,
        error_code: String,
        error_message: String,
    ) -> ConnectorErrorType {
        match (error_code.as_str(), error_message.as_str()) {
            // 2xx card api error codes and messages mapping
            ("100.100.600", "Empty CVV for VISA, MASTER not allowed") => ConnectorErrorType::UserError,
            ("100.350.100", "Referenced session is rejected (no action possible)") => ConnectorErrorType::TechnicalError,
            ("100.380.401", "User authentication failed") => ConnectorErrorType::UserError,
            ("100.380.501", "Risk management transaction timeout") => ConnectorErrorType::TechnicalError,
            ("100.390.103", "PARes validation failed - problem with signature") => ConnectorErrorType::TechnicalError,
            ("100.390.111", "Communication error to VISA/Mastercard Directory Server") => ConnectorErrorType::TechnicalError,
            ("100.390.112", "Technical error in 3D system") => ConnectorErrorType::TechnicalError,
            ("100.390.115", "Authentication failed due to invalid message format") => ConnectorErrorType::TechnicalError,
            ("100.390.118", "Authentication failed due to suspected fraud") => ConnectorErrorType::UserError,
            ("100.400.304", "Invalid input data") => ConnectorErrorType::UserError,
            ("200.300.404", "Invalid or missing parameter") => ConnectorErrorType::UserError,
            ("300.100.100", "Transaction declined (additional customer authentication required)") => ConnectorErrorType::UserError,
            ("400.001.301", "Card not enrolled in 3DS") => ConnectorErrorType::UserError,
            ("400.001.600", "Authentication error") => ConnectorErrorType::UserError,
            ("400.001.601", "Transaction declined (auth. declined)") => ConnectorErrorType::UserError,
            ("400.001.602", "Invalid transaction") => ConnectorErrorType::UserError,
            ("400.001.603", "Invalid transaction") => ConnectorErrorType::UserError,
            ("700.400.200", "Cannot refund (refund volume exceeded or tx reversed or invalid workflow)") => ConnectorErrorType::BusinessError,
            ("700.500.001", "Referenced session contains too many transactions") => ConnectorErrorType::TechnicalError,
            ("700.500.003", "Test accounts not allowed in production") => ConnectorErrorType::UserError,
            ("800.100.151", "Transaction declined (invalid card)") => ConnectorErrorType::UserError,
            ("800.100.152", "Transaction declined by authorization system") => ConnectorErrorType::UserError,
            ("800.100.153", "Transaction declined (invalid CVV)") => ConnectorErrorType::UserError,
            ("800.100.155", "Transaction declined (amount exceeds credit)") => ConnectorErrorType::UserError,
            ("800.100.157", "Transaction declined (wrong expiry date)") => ConnectorErrorType::UserError,
            ("800.100.162", "Transaction declined (limit exceeded)") => ConnectorErrorType::BusinessError,
            ("800.100.163", "Transaction declined (maximum transaction frequency exceeded)") => ConnectorErrorType::BusinessError,
            ("800.100.168", "Transaction declined (restricted card)") => ConnectorErrorType::UserError,
            ("800.100.170", "Transaction declined (transaction not permitted)") => ConnectorErrorType::UserError,
            ("800.100.172", "Transaction declined (account blocked)") => ConnectorErrorType::BusinessError,
            ("800.100.190", "Transaction declined (invalid configuration data)") => ConnectorErrorType::BusinessError,
            ("800.120.100", "Rejected by throttling") => ConnectorErrorType::TechnicalError,
            ("800.300.401", "Bin blacklisted") => ConnectorErrorType::BusinessError,
            ("800.700.100", "Transaction for the same session is currently being processed, please try again later") => ConnectorErrorType::TechnicalError,
            ("900.100.300", "Timeout, uncertain result") => ConnectorErrorType::TechnicalError,
            // 4xx error codes for cards api are unique and messages vary, so we are relying only on error code to decide an error type
            ("4" | "5" | "6" | "7" | "8" | "9" | "10" | "11" | "12" | "13" | "14" | "15" | "16" | "17" | "18" | "19" | "26" | "34" | "39" | "48" | "52" | "85" | "86", _) => ConnectorErrorType::UserError,
            ("21" | "22" | "23" | "30" | "31" | "32" | "35" | "37" | "40" | "41" | "45" | "46" | "49" | "50" | "56" | "60" | "67" | "81" | "82" | "83" | "84" | "87", _) => ConnectorErrorType::BusinessError,
            ("59", _) => ConnectorErrorType::TechnicalError,
            ("1", _) => ConnectorErrorType::UnknownError,
            // Error codes for bank redirects api are unique and messages vary, so we are relying only on error code to decide an error type
            ("1112008" | "1132000" | "1152000", _) => ConnectorErrorType::UserError,
            ("1112009" | "1122006" | "1132001" | "1132002" | "1132003" | "1132004" | "1132005" | "1132006" | "1132008" | "1132009" | "1132010" | "1132011" | "1132012" | "1132013" | "1133000" | "1133001" | "1133002" | "1133003" | "1133004", _) => ConnectorErrorType::BusinessError,
            ("1132014", _) => ConnectorErrorType::TechnicalError,
            ("1132007", _) => ConnectorErrorType::UnknownError,
            _ => ConnectorErrorType::UnknownError,
        }
    }
}
