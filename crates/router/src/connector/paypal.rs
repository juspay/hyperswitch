pub mod transformers;
use std::fmt::Debug;

use base64::Engine;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use transformers as paypal;

use self::transformers::PaypalMeta;
use crate::{
    configs::settings,
    connector::utils::{to_connector_meta, RefundsRequestData},
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation, PaymentAction,
    },
    types::{
        self,
        api::{self, CompleteAuthorize, ConnectorCommon, ConnectorCommonExt},
        storage::enums as storage_enums,
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Paypal;

impl api::Payment for Paypal {}
impl api::PaymentSession for Paypal {}
impl api::PaymentToken for Paypal {}
impl api::ConnectorAccessToken for Paypal {}
impl api::PreVerify for Paypal {}
impl api::PaymentAuthorize for Paypal {}
impl api::PaymentsCompleteAuthorize for Paypal {}
impl api::PaymentSync for Paypal {}
impl api::PaymentCapture for Paypal {}
impl api::PaymentVoid for Paypal {}
impl api::Refund for Paypal {}
impl api::RefundExecute for Paypal {}
impl api::RefundSync for Paypal {}

impl Paypal {
    pub fn connector_transaction_id(
        &self,
        payment_method: Option<storage_enums::PaymentMethod>,
        connector_meta: &Option<serde_json::Value>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        match payment_method {
            Some(diesel_models::enums::PaymentMethod::Wallet)
            | Some(diesel_models::enums::PaymentMethod::BankRedirect) => {
                let meta: PaypalMeta = to_connector_meta(connector_meta.clone())?;
                Ok(Some(meta.order_id))
            }
            _ => {
                let meta: PaypalMeta = to_connector_meta(connector_meta.clone())?;
                Ok(meta.authorize_id)
            }
        }
    }

    pub fn get_order_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        //Handled error response separately for Orders as the end point is different for Orders - (Authorize) and Payments - (Capture, void, refund, rsync).
        //Error response have different fields for Orders and Payments.
        let response: paypal::PaypalOrderErrorResponse = res
            .response
            .parse_struct("Paypal ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let error_reason = match response.details {
            Some(order_errors) => order_errors
                .iter()
                .map(|error| {
                    let mut reason = format!("description - {}", error.description);
                    if let Some(value) = &error.value {
                        reason.push_str(&format!(", value - {value}"));
                    }
                    if let Some(field) = error
                        .field
                        .as_ref()
                        .and_then(|field| field.split('/').last())
                    {
                        reason.push_str(&format!(", field - {field}"));
                    }
                    reason.push(';');
                    reason
                })
                .collect::<String>(),
            None => consts::NO_ERROR_MESSAGE.to_string(),
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.name,
            message: response.message,
            reason: Some(error_reason),
        })
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Paypal
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let access_token = req
            .access_token
            .clone()
            .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
        let key = &req.attempt_id;

        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", access_token.token.peek()).into_masked(),
            ),
            (
                "Prefer".to_string(),
                "return=representation".to_string().into(),
            ),
            (
                "PayPal-Request-Id".to_string(),
                key.to_string().into_masked(),
            ),
        ])
    }
}

impl ConnectorCommon for Paypal {
    fn id(&self) -> &'static str {
        "paypal"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.paypal.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: paypal::PaypalAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: paypal::PaypalPaymentErrorResponse = res
            .response
            .parse_struct("Paypal ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let error_reason = match response.details {
            Some(error_details) => error_details
                .iter()
                .map(|error| format!("description - {} ; ", error.description))
                .collect::<String>(),
            None => consts::NO_ERROR_MESSAGE.to_string(),
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.name,
            message: response.message,
            reason: Some(error_reason),
        })
    }
}

impl ConnectorValidation for Paypal {
    fn validate_capture_method(
        &self,
        capture_method: enums::CaptureMethod,
    ) -> CustomResult<(), errors::ConnectorError> {
        let unsupported_capture_method = match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => None,
            enums::CaptureMethod::ManualMultiple => Some("manual_multiple"),
            enums::CaptureMethod::Scheduled => Some("schedule"),
        };
        if let Some(capture_method) = unsupported_capture_method {
            Err(errors::ConnectorError::NotImplemented(capture_method.into()).into())
        } else {
            Ok(())
        }
    }
}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Paypal
{
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Paypal
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Paypal
{
    fn get_url(
        &self,
        _req: &types::RefreshTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v1/oauth2/token", self.base_url(connectors)))
    }
    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }
    fn get_headers(
        &self,
        req: &types::RefreshTokenRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: paypal::PaypalAuthType = (&req.connector_auth_type)
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let auth_id = auth
            .key1
            .zip(auth.api_key)
            .map(|(key1, api_key)| format!("{}:{}", key1, api_key));
        let auth_val = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_id.peek()));

        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefreshTokenType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (headers::AUTHORIZATION.to_string(), auth_val.into_masked()),
        ])
    }
    fn get_request_body(
        &self,
        req: &types::RefreshTokenRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = paypal::PaypalAuthUpdateRequest::try_from(req)?;
        let paypal_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<paypal::PaypalAuthUpdateRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(paypal_req))
    }

    fn build_request(
        &self,
        req: &types::RefreshTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let req = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        let response: paypal::PaypalAuthUpdateResponse = res
            .response
            .parse_struct("Paypal PaypalAuthUpdateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

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
        let response: paypal::PaypalAccessTokenErrorResponse = res
            .response
            .parse_struct("Paypal AccessTokenErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error,
            message: response.error_description,
            reason: None,
        })
    }
}

impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Paypal
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Paypal
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
        if req.request.capture_method == Some(enums::CaptureMethod::ManualMultiple) {
            return Err(errors::ConnectorError::NotImplemented(format!(
                "{}{}",
                consts::MANUAL_MULTIPLE_NOT_IMPLEMENTED_ERROR_MESSAGE,
                self.id()
            ))
            .into());
        }
        Ok(format!("{}v2/checkout/orders", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = paypal::PaypalPaymentsRequest::try_from(req)?;
        let paypal_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<paypal::PaypalPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(paypal_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        self.validate_capture_method(req.request.capture_method.unwrap_or_default())?;
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
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
        match data.payment_method {
            diesel_models::enums::PaymentMethod::Wallet
            | diesel_models::enums::PaymentMethod::BankRedirect => {
                let response: paypal::PaypalRedirectResponse = res
                    .response
                    .parse_struct("paypal PaymentsRedirectResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            _ => {
                let response: paypal::PaypalOrdersResponse = res
                    .response
                    .parse_struct("paypal PaymentsOrderResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
        self.get_order_error_response(res)
    }
}

impl
    ConnectorIntegration<
        CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Paypal
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
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let paypal_meta: PaypalMeta = to_connector_meta(req.request.connector_meta.clone())?;
        Ok(format!(
            "{}v2/checkout/orders/{}/capture",
            self.base_url(connectors),
            paypal_meta.order_id
        ))
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
        let response: paypal::PaypalOrdersResponse = res
            .response
            .parse_struct("paypal PaymentsOrderResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Paypal
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
        let paypal_meta: PaypalMeta = to_connector_meta(req.request.connector_meta.clone())?;
        match req.payment_method {
            diesel_models::enums::PaymentMethod::Wallet
            | diesel_models::enums::PaymentMethod::BankRedirect => Ok(format!(
                "{}v2/checkout/orders/{}",
                self.base_url(connectors),
                paypal_meta.order_id
            )),
            _ => {
                let capture_id = req
                    .request
                    .connector_transaction_id
                    .get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
                let psync_url = match paypal_meta.psync_flow {
                    transformers::PaypalPaymentIntent::Authorize => format!(
                        "v2/payments/authorizations/{}",
                        paypal_meta.authorize_id.unwrap_or_default()
                    ),
                    transformers::PaypalPaymentIntent::Capture => {
                        format!("v2/payments/captures/{}", capture_id)
                    }
                };
                Ok(format!("{}{}", self.base_url(connectors), psync_url))
            }
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
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        match data.payment_method {
            diesel_models::enums::PaymentMethod::Wallet
            | diesel_models::enums::PaymentMethod::BankRedirect => {
                let response: paypal::PaypalOrdersResponse = res
                    .response
                    .parse_struct("paypal PaymentsOrderResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            _ => {
                let response: paypal::PaypalPaymentsSyncResponse = res
                    .response
                    .parse_struct("paypal PaymentsSyncResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Paypal
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
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let paypal_meta: PaypalMeta = to_connector_meta(req.request.connector_meta.clone())?;
        let authorize_id = paypal_meta.authorize_id.ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Missing Authorize id".to_string(),
            ),
        )?;
        Ok(format!(
            "{}v2/payments/authorizations/{}/capture",
            self.base_url(connectors),
            authorize_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = paypal::PaypalPaymentsCaptureRequest::try_from(req)?;
        let paypal_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<paypal::PaypalPaymentsCaptureRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(paypal_req))
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
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: paypal::PaymentCaptureResponse = res
            .response
            .parse_struct("Paypal PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Paypal
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
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let paypal_meta: PaypalMeta = to_connector_meta(req.request.connector_meta.clone())?;
        let authorize_id = paypal_meta.authorize_id.ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Missing Authorize id".to_string(),
            ),
        )?;
        Ok(format!(
            "{}v2/payments/authorizations/{}/void",
            self.base_url(connectors),
            authorize_id,
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
            .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: paypal::PaypalPaymentsCancelResponse = res
            .response
            .parse_struct("PaymentCancelResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Paypal {
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
            "{}v2/payments/captures/{}/refund",
            self.base_url(connectors),
            id,
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = paypal::PaypalRefundRequest::try_from(req)?;
        let paypal_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<paypal::PaypalRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(paypal_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
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
        let response: paypal::RefundResponse =
            res.response
                .parse_struct("paypal RefundResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Paypal {
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
            "{}v2/payments/refunds/{}",
            self.base_url(connectors),
            req.request.get_connector_refund_id()?
        ))
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
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: paypal::RefundSyncResponse = res
            .response
            .parse_struct("paypal RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

#[async_trait::async_trait]
impl api::IncomingWebhook for Paypal {
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

impl services::ConnectorRedirectResponse for Paypal {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        _action: PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}
