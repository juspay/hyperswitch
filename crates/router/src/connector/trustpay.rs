mod transformers;

use std::fmt::Debug;
use base64::Engine;
use error_stack::{ResultExt, IntoReport};

use crate::{
    configs::settings,
    utils::{self, BytesExt},
    consts,
    core::{
        errors::{self, CustomResult}, payments,
    },
    headers, logger, services::{self, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse, Response,
    }
};


use transformers as trustpay;

#[derive(Debug, Clone)]
pub struct Trustpay;

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

    fn get_auth_header(&self, auth_type:&types::ConnectorAuthType)-> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        let auth: trustpay::TrustpayAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::X_API_KEY.to_string(), auth.api_key)])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        logger::debug!(trustpay_error_response=?res);
        let response: trustpay::TrustpayErrorResponse = res
            .response
            .parse_struct("trustpay ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let default_error = trustpay::Errors {
            code: 0,
            description: "".to_string()
        };
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status.to_string(),
            message: format!("{:?}", response.errors.first().unwrap_or_else(|| &default_error)),
            reason: None,
        })
    }
}

impl api::Payment for Trustpay {}

impl api::PreVerify for Trustpay {}
impl
    ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Trustpay
{
}

impl api::PaymentVoid for Trustpay {}

impl
    ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Trustpay
{}

impl api::ConnectorAccessToken for Trustpay {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Trustpay
{
    fn get_url(
        &self,
        _req: &types::RefreshTokenRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok("https://aapi.trustpay.eu/api/oauth2/token".to_owned())
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_headers(
        &self,
        req: &types::RefreshTokenRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth: trustpay::TrustpayAuthType = (&req.connector_auth_type)
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_value = format!("Basic {}", consts::BASE64_ENGINE.encode(format!("{}:{}", auth.project_id, auth.secret_key)));
        print!(">>>auth_value{}", auth_value);
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefreshTokenType::get_content_type(self).to_string(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                auth_value,
            ),
        ])
    }

    fn get_request_body(
        &self,
        req: &types::RefreshTokenRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let trustpay_req = utils::Encode::<trustpay::TrustpayAuthUpdateRequest>::convert_and_url_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        logger::debug!(trustpay_access_token_request=?trustpay_req);
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
                .headers(types::RefreshTokenType::get_headers(self, req, connectors)?)
                .url(&types::RefreshTokenType::get_url(self, req, connectors)?)
                .body(types::RefreshTokenType::get_request_body(self, req)?)
                .build(),
        );
        logger::debug!(trustpay_access_token_request=?req);
        Ok(req)
    }

    fn handle_response(
        &self,
        data: &types::RefreshTokenRouterData,
        res: Response,
    ) -> CustomResult<types::RefreshTokenRouterData, errors::ConnectorError> {
        logger::debug!(access_token_response=?res);
        let response: trustpay::TrustpayAuthUpdateResponse = res
            .response
            .parse_struct("trustpay TrustpayAuthUpdateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        logger::debug!(access_token_error_response=?res);
        let response: trustpay::TrustpayAccessTokenErrorResponse = res
            .response
            .parse_struct("Trustpay AccessTokenErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.result_info.result_code.to_string(),
            message: response.result_info.additional_info.unwrap_or_default(),
            reason: None,
        })
    }
}

impl api::PaymentSync for Trustpay {}
impl
    ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Trustpay
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        match req.payment_method {
            storage_models:: enums:: PaymentMethod::BankRedirect => {
                let access_token = req
                    .access_token
                    .clone()
                    .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
                let header = vec![
                    (
                        headers::CONTENT_TYPE.to_string(),
                        "application/json".to_owned(),
                    ),
                    (
                        headers::AUTHORIZATION.to_string(),
                        format!("Bearer {}", access_token.token),
                    ),
                ];
                Ok(header)
            }
            _ => {
                let mut header = vec![
                    (
                        headers::CONTENT_TYPE.to_string(),
                        types::PaymentsAuthorizeType::get_content_type(self).to_string(),
                    ),
                ];
                let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
                header.append(&mut api_key);
                Ok(header)
            }
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
        let id = req.request.connector_transaction_id.clone();
        match req.payment_method {
            storage_models:: enums:: PaymentMethod::BankRedirect => {
                Ok(format!(
                    "{}/{}",
                    "https://aapi.trustpay.eu/api/Payments/Payment",
                    id.get_connector_transaction_id()
                        .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
                ))
            },
            _ => Ok(format!(
                    "{}{}/{}",
                    self.base_url(connectors),
                    "api/v1/instance",
                    id.get_connector_transaction_id()
                        .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
                ))
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
        logger::debug!(payment_sync_response=?res);
        let response: trustpay:: TrustpayPaymentsResponse = res
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
impl
    ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Trustpay
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        todo!()
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
        let response: trustpay::TrustpayPaymentsResponse = res
            .response
            .parse_struct("Trustpay PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(trustpaypayments_create_response=?response);
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Trustpay {}

impl
    ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Trustpay
{
    //TODO: implement sessions flow
}

impl api::PaymentAuthorize for Trustpay {}

impl
    ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Trustpay {
    fn get_headers(&self, req: &types::PaymentsAuthorizeRouterData, _connectors: &settings::Connectors,) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        match req.request.payment_method_data {
            api_models::payments::PaymentMethodData::BankRedirect{..} => {
                let access_token = req
                    .access_token
                    .clone()
                    .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
                let header = vec![
                    (
                        headers::CONTENT_TYPE.to_string(),
                        "application/json".to_owned(),
                    ),
                    (
                        headers::AUTHORIZATION.to_string(),
                        format!("Bearer {}", access_token.token),
                    ),
                ];
                Ok(header)
            }
            _ => {
                let mut header = vec![
                    (
                        headers::CONTENT_TYPE.to_string(),
                        types::PaymentsAuthorizeType::get_content_type(self).to_string(),
                    ),
                ];
                let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
                header.append(&mut api_key);
                Ok(header)
            }
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, req: &types::PaymentsAuthorizeRouterData, connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        match req.request.payment_method_data {
            api_models::payments::PaymentMethodData::BankRedirect{..} => {
                Ok("https://aapi.trustpay.eu/api/Payments/Payment".to_owned())
            }
            _ => Ok(format!(
                    "{}{}",
                    self.base_url(connectors),
                    "api/v1/purchase"
                ))
        }
    }

    fn get_request_body(&self, req: &types::PaymentsAuthorizeRouterData) -> CustomResult<Option<String>,errors::ConnectorError> {
        let trustpay_req = trustpay::TrustpayPaymentsRequest::try_from(req)?;
        let trustpay_req_string = match req.request.payment_method_data {
            api_models::payments::PaymentMethodData::BankRedirect{..} => {
                utils::Encode::<trustpay::TrustpayPaymentsRequest>::encode_to_string_of_json(&trustpay_req)
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?
            },
            _ => utils::Encode::<trustpay::TrustpayPaymentsRequest>::encode(&trustpay_req)
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?,
        };
        print!(">>>req{}", trustpay_req_string);
        logger::debug!(trustpay_payment_logs=?trustpay_req_string);
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
    ) -> CustomResult<types::PaymentsAuthorizeRouterData,errors::ConnectorError> {
        let response: trustpay::TrustpayPaymentsResponse = res.response.parse_struct("PaymentIntentResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(trustpaypayments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Trustpay {}
impl api::RefundExecute for Trustpay {}
impl api::RefundSync for Trustpay {}

impl
    ConnectorIntegration<
        api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > for Trustpay {
    fn get_headers(&self, req: &types::RefundsRouterData<api::Execute>, _connectors: &settings::Connectors,) -> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefundExecuteType::get_content_type(self).to_string(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, req: &types::RefundsRouterData<api::Execute>, connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        match req.payment_method {
            storage_models:: enums:: PaymentMethod::BankRedirect => Ok(format!(
                "{}{}{}",
                "https://aapi.trustpay.eu/api/Payments/Payment/",
                req.request.connector_transaction_id,
                "/Refund"
            )),
            _ => Ok(format!(
                "{}{}",
                self.base_url(connectors),
                "api/v1/Reverse"
            )),
        }
    }

    fn get_request_body(&self, req: &types::RefundsRouterData<api::Execute>) -> CustomResult<Option<String>,errors::ConnectorError> {
        let trustpay_req = trustpay::TrustpayRefundRequestWrapper::try_from(req)?;
        let trustpay_req_string = match req.payment_method {
            storage_models:: enums:: PaymentMethod::BankRedirect => {
                utils::Encode::<trustpay::TrustpayRefundRequestBankRedirect>::encode_to_string_of_json(&trustpay_req.bank_refunds)
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?
            },
            _ => utils::Encode::<trustpay::TrustpayRefundRequestCards>::encode(&trustpay_req.card_refunds)
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?,
        };
        print!(">>>req{:?}", trustpay_req_string);
        Ok(Some(trustpay_req_string))
    }

    fn build_request(&self, req: &types::RefundsRouterData<api::Execute>, connectors: &settings::Connectors,) -> CustomResult<Option<services::Request>,errors::ConnectorError> {
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
    ) -> CustomResult<types::RefundsRouterData<api::Execute>,errors::ConnectorError> {
        logger::debug!(target: "router::connector::trustpay", response=?res);
        let response: trustpay::RefundResponse = res.response.parse_struct("trustpay RefundResponse").change_context(errors::ConnectorError::RequestEncodingFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Trustpay {
    fn get_headers(&self, req: &types::RefundSyncRouterData, _connectors: &settings::Connectors,) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, req: &types::RefundSyncRouterData,connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        let id = req.request.connector_refund_id.to_owned().ok_or_else(|| {
            errors::ConnectorError::MissingConnectorRefundID})?;
        Ok(format!(
            "{}{}/{}",
            self.base_url(connectors),
            "api/v1/instance",
            id
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
    ) -> CustomResult<types::RefundSyncRouterData,errors::ConnectorError,> {
        logger::debug!(target: "router::connector::trustpay", response=?res);
        let response: trustpay::RefundResponse = res.response.parse_struct("trustpay RefundResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Trustpay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}

impl services::ConnectorRedirectResponse for Trustpay {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}