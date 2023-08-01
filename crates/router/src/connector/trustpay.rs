pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_utils::{crypto, errors::ReportSwitchExt, ext_traits::ByteSliceExt};
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use transformers as trustpay;

use super::utils::collect_and_sort_values_by_removing_signature;
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
        let response: trustpay::TrustpayErrorResponse = res
            .response
            .parse_struct("trustpay ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let default_error = trustpay::Errors {
            code: 0,
            description: consts::NO_ERROR_CODE.to_string(),
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status.to_string(),
            message: format!(
                "{:?}",
                response
                    .errors
                    .as_ref()
                    .unwrap_or(&vec![])
                    .first()
                    .unwrap_or(&default_error)
            ),
            reason: response.errors.map(|errors| format!("{:?}", errors)),
        })
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

impl api::PreVerify for Trustpay {}
impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Trustpay
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
            message: response
                .result_info
                .additional_info
                .clone()
                .unwrap_or_default(),
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
            message: response.payment_description.clone(),
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
        let create_intent_req = trustpay::TrustpayCreateIntentRequest::try_from(req)?;
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
        let connector_req = trustpay::TrustpayPaymentsRequest::try_from(req)?;
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
        let connector_req = trustpay::TrustpayRefundRequest::try_from(req)?;
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
        _secret: &[u8],
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
        query_params: &str,
        _json_payload: Option<serde_json::Value>,
        _action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        let query =
            serde_urlencoded::from_str::<transformers::TrustpayRedirectResponse>(query_params)
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        crate::logger::debug!(trustpay_redirect_response=?query);
        Ok(query.status.map_or(
            payments::CallConnectorAction::Trigger,
            |status| match status.as_str() {
                "SuccessOk" => payments::CallConnectorAction::StatusUpdate {
                    status: diesel_models::enums::AttemptStatus::Charged,
                    error_code: None,
                    error_message: None,
                },
                _ => payments::CallConnectorAction::Trigger,
            },
        ))
    }
}
