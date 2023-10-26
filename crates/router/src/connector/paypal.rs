pub mod transformers;
use std::fmt::{Debug, Write};

use base64::Engine;
use common_utils::ext_traits::ByteSliceExt;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use transformers as paypal;

use self::transformers::{PaypalAuthResponse, PaypalMeta, PaypalWebhookEventType};
use super::utils::PaymentsCompleteAuthorizeRequestData;
use crate::{
    configs::settings,
    connector::{
        utils as connector_utils,
        utils::{to_connector_meta, RefundsRequestData},
    },
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
        api::{self, CompleteAuthorize, ConnectorCommon, ConnectorCommonExt, VerifyWebhookSource},
        transformers::ForeignFrom,
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
impl api::MandateSetup for Paypal {}
impl api::PaymentAuthorize for Paypal {}
impl api::PaymentsCompleteAuthorize for Paypal {}
impl api::PaymentSync for Paypal {}
impl api::PaymentCapture for Paypal {}
impl api::PaymentVoid for Paypal {}
impl api::Refund for Paypal {}
impl api::RefundExecute for Paypal {}
impl api::RefundSync for Paypal {}
impl api::ConnectorVerifyWebhookSource for Paypal {}

impl Paypal {
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

        let error_reason = response.details.map(|order_errors| {
            order_errors
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
                .collect::<String>()
        });
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.name,
            message: response.message.clone(),
            reason: error_reason.or(Some(response.message)),
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

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
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

        let error_reason = response
            .details
            .map(|error_details| {
                error_details
                    .iter()
                    .try_fold(String::new(), |mut acc, error| {
                        if let Some(description) = &error.description {
                            write!(acc, "description - {} ;", description)
                                .into_report()
                                .change_context(
                                    errors::ConnectorError::ResponseDeserializationFailed,
                                )
                                .attach_printable("Failed to concatenate error details")
                                .map(|_| acc)
                        } else {
                            Ok(acc)
                        }
                    })
            })
            .transpose()?;
        let reason = match error_reason {
            Some(err_reason) => err_reason
                .is_empty()
                .then(|| response.message.to_owned())
                .or(Some(err_reason)),
            None => Some(response.message.to_owned()),
        };

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.name,
            message: response.message.clone(),
            reason,
        })
    }
}

impl ConnectorValidation for Paypal {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }

    fn validate_if_surcharge_implemented(&self) -> CustomResult<(), errors::ConnectorError> {
        Ok(())
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
            message: response.error_description.clone(),
            reason: Some(response.error_description),
        })
    }
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Paypal
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
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v2/checkout/orders", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = paypal::PaypalRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request
                .surcharge_details
                .as_ref()
                .map_or(req.request.amount, |surcharge| surcharge.final_amount),
            req,
        ))?;
        let req_obj = paypal::PaypalPaymentsRequest::try_from(&connector_router_data)?;
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
        let response: PaypalAuthResponse =
            res.response
                .parse_struct("paypal PaypalAuthResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        match response {
            PaypalAuthResponse::PaypalOrdersResponse(response) => {
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            PaypalAuthResponse::PaypalRedirectResponse(response) => {
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            PaypalAuthResponse::PaypalThreeDsResponse(response) => {
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
        let complete_authorize_url = if req.request.is_auto_capture()? {
            "capture".to_string()
        } else {
            "authorize".to_string()
        };
        Ok(format!(
            "{}v2/checkout/orders/{}/{complete_authorize_url}",
            self.base_url(connectors),
            req.request
                .connector_transaction_id
                .clone()
                .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?
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
            .parse_struct("paypal PaypalOrdersResponse")
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
                req.request
                    .connector_transaction_id
                    .get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
            )),
            _ => {
                let psync_url = match paypal_meta.psync_flow {
                    transformers::PaypalPaymentIntent::Authorize => {
                        let authorize_id = paypal_meta.authorize_id.ok_or(
                            errors::ConnectorError::RequestEncodingFailedWithReason(
                                "Missing Authorize id".to_string(),
                            ),
                        )?;
                        format!("v2/payments/authorizations/{authorize_id}",)
                    }
                    transformers::PaypalPaymentIntent::Capture => {
                        let capture_id = paypal_meta.capture_id.ok_or(
                            errors::ConnectorError::RequestEncodingFailedWithReason(
                                "Missing Capture id".to_string(),
                            ),
                        )?;
                        format!("v2/payments/captures/{capture_id}")
                    }
                    // only set when payment is done through card 3DS
                    //because no authorize or capture id is generated during payment authorize call for card 3DS
                    transformers::PaypalPaymentIntent::Authenticate => {
                        format!(
                            "v2/checkout/orders/{}",
                            req.request
                                .connector_transaction_id
                                .get_connector_transaction_id()
                                .change_context(
                                    errors::ConnectorError::MissingConnectorTransactionID
                                )?
                        )
                    }
                };
                Ok(format!("{}{psync_url}", self.base_url(connectors)))
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
        let response: paypal::PaypalSyncResponse = res
            .response
            .parse_struct("paypal SyncResponse")
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
        let connector_router_data = paypal::PaypalRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let connector_req = paypal::PaypalPaymentsCaptureRequest::try_from(&connector_router_data)?;
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
                .body(types::PaymentsCaptureType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: paypal::PaypalCaptureResponse = res
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
        let paypal_meta: PaypalMeta = to_connector_meta(req.request.connector_metadata.clone())?;
        let capture_id = paypal_meta.capture_id.ok_or(
            errors::ConnectorError::RequestEncodingFailedWithReason(
                "Missing Capture id".to_string(),
            ),
        )?;
        Ok(format!(
            "{}v2/payments/captures/{}/refund",
            self.base_url(connectors),
            capture_id,
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = paypal::PaypalRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let req_obj = paypal::PaypalRefundRequest::try_from(&connector_router_data)?;
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

impl
    ConnectorIntegration<
        VerifyWebhookSource,
        types::VerifyWebhookSourceRequestData,
        types::VerifyWebhookSourceResponseData,
    > for Paypal
{
    fn get_headers(
        &self,
        req: &types::RouterData<
            VerifyWebhookSource,
            types::VerifyWebhookSourceRequestData,
            types::VerifyWebhookSourceResponseData,
        >,
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
                types::VerifyWebhookSourceType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (headers::AUTHORIZATION.to_string(), auth_val.into_masked()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RouterData<
            VerifyWebhookSource,
            types::VerifyWebhookSourceRequestData,
            types::VerifyWebhookSourceResponseData,
        >,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}v1/notifications/verify-webhook-signature",
            self.base_url(connectors)
        ))
    }

    fn build_request(
        &self,
        req: &types::VerifyWebhookSourceRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::VerifyWebhookSourceType::get_url(
                self, req, connectors,
            )?)
            .headers(types::VerifyWebhookSourceType::get_headers(
                self, req, connectors,
            )?)
            .body(types::VerifyWebhookSourceType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    fn get_request_body(
        &self,
        req: &types::RouterData<
            VerifyWebhookSource,
            types::VerifyWebhookSourceRequestData,
            types::VerifyWebhookSourceResponseData,
        >,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = paypal::PaypalSourceVerificationRequest::try_from(&req.request)?;
        let paypal_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<paypal::PaypalSourceVerificationRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(paypal_req))
    }

    fn handle_response(
        &self,
        data: &types::VerifyWebhookSourceRouterData,
        res: Response,
    ) -> CustomResult<types::VerifyWebhookSourceRouterData, errors::ConnectorError> {
        let response: paypal::PaypalSourceVerificationResponse = res
            .response
            .parse_struct("paypal PaypalSourceVerificationResponse")
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
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let payload: paypal::PaypalWebhooksBody =
            request
                .body
                .parse_struct("PaypalWebhooksBody")
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        match payload.resource {
            paypal::PaypalResource::PaypalCardWebhooks(resource) => {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        resource.supplementary_data.related_ids.order_id,
                    ),
                ))
            }
            paypal::PaypalResource::PaypalRedirectsWebhooks(resource) => {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::PaymentAttemptId(
                        resource
                            .purchase_units
                            .first()
                            .and_then(|unit| unit.invoice_id.clone().or(unit.reference_id.clone()))
                            .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                    ),
                ))
            }
            paypal::PaypalResource::PaypalRefundWebhooks(resource) => {
                Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                    api_models::webhooks::RefundIdType::ConnectorRefundId(resource.id),
                ))
            }
            paypal::PaypalResource::PaypalDisputeWebhooks(resource) => {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::PaymentAttemptId(
                        resource
                            .dispute_transactions
                            .first()
                            .map(|transaction| transaction.reference_id.clone())
                            .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                    ),
                ))
            }
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let payload: paypal::PaypalWebooksEventType = request
            .body
            .parse_struct("PaypalWebooksEventType")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let outcome = match payload.event_type {
            PaypalWebhookEventType::CustomerDisputeCreated
            | PaypalWebhookEventType::CustomerDisputeResolved
            | PaypalWebhookEventType::CustomerDisputedUpdated
            | PaypalWebhookEventType::RiskDisputeCreated => Some(
                request
                    .body
                    .parse_struct::<paypal::DisputeOutcome>("PaypalWebooksEventType")
                    .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?
                    .outcome_code,
            ),
            PaypalWebhookEventType::PaymentAuthorizationCreated
            | PaypalWebhookEventType::PaymentAuthorizationVoided
            | PaypalWebhookEventType::PaymentCaptureDeclined
            | PaypalWebhookEventType::PaymentCaptureCompleted
            | PaypalWebhookEventType::PaymentCapturePending
            | PaypalWebhookEventType::PaymentCaptureRefunded
            | PaypalWebhookEventType::CheckoutOrderApproved
            | PaypalWebhookEventType::CheckoutOrderCompleted
            | PaypalWebhookEventType::CheckoutOrderProcessed
            | PaypalWebhookEventType::Unknown => None,
        };

        Ok(api::IncomingWebhookEvent::foreign_from((
            payload.event_type,
            outcome,
        )))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let details: paypal::PaypalWebhooksBody =
            request
                .body
                .parse_struct("PaypalWebhooksBody")
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        let sync_payload = match details.resource {
            paypal::PaypalResource::PaypalCardWebhooks(resource) => serde_json::to_value(
                paypal::PaypalPaymentsSyncResponse::try_from((*resource, details.event_type))?,
            )
            .into_report()
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?,
            paypal::PaypalResource::PaypalRedirectsWebhooks(resource) => serde_json::to_value(
                paypal::PaypalOrdersResponse::try_from((*resource, details.event_type))?,
            )
            .into_report()
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?,
            paypal::PaypalResource::PaypalRefundWebhooks(resource) => serde_json::to_value(
                paypal::RefundSyncResponse::try_from((*resource, details.event_type))?,
            )
            .into_report()
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?,
            paypal::PaypalResource::PaypalDisputeWebhooks(_) => serde_json::to_value(details)
                .into_report()
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?,
        };
        Ok(sync_payload)
    }

    fn get_dispute_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::disputes::DisputePayload, errors::ConnectorError> {
        let payload: paypal::PaypalDisputeWebhooks = request
            .body
            .parse_struct("PaypalDisputeWebhooks")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(api::disputes::DisputePayload {
            amount: connector_utils::to_currency_lower_unit(
                payload.dispute_amount.value,
                payload.dispute_amount.currency_code,
            )?,
            currency: payload.dispute_amount.currency_code.to_string(),
            dispute_stage: api_models::enums::DisputeStage::from(
                payload.dispute_life_cycle_stage.clone(),
            ),
            connector_status: payload.status.to_string(),
            connector_dispute_id: payload.dispute_id,
            connector_reason: payload.reason,
            connector_reason_code: payload.external_reason_code,
            challenge_required_by: payload.seller_response_due_date,
            created_at: payload.create_time,
            updated_at: payload.update_time,
        })
    }
}

impl services::ConnectorRedirectResponse for Paypal {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        action: PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        match action {
            services::PaymentAction::PSync | services::PaymentAction::CompleteAuthorize => {
                Ok(payments::CallConnectorAction::Trigger)
            }
        }
    }
}
