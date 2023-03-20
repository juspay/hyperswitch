mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_utils::{crypto, errors::ReportSwitchExt, ext_traits::ByteSliceExt};
use error_stack::{IntoReport, ResultExt};
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
    services::{self, ConnectorIntegration},
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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        match req.payment_method {
            storage_models::enums::PaymentMethod::BankRedirect => {
                let token = req
                    .access_token
                    .clone()
                    .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
                Ok(vec![
                    (
                        headers::CONTENT_TYPE.to_string(),
                        "application/json".to_owned(),
                    ),
                    (
                        headers::AUTHORIZATION.to_string(),
                        format!("Bearer {}", token.token),
                    ),
                ])
            }
            _ => {
                let mut header = vec![(
                    headers::CONTENT_TYPE.to_string(),
                    self.get_content_type().to_string(),
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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth = trustpay::TrustpayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::X_API_KEY.to_string(), auth.api_key)])
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
            message: format!("{:?}", response.errors.first().unwrap_or(&default_error)),
            reason: None,
        })
    }
}

impl api::Payment for Trustpay {}

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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth = trustpay::TrustpayAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_value = format!(
            "Basic {}",
            consts::BASE64_ENGINE.encode(format!("{}:{}", auth.project_id, auth.secret_key))
        );
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefreshTokenType::get_content_type(self).to_string(),
            ),
            (headers::AUTHORIZATION.to_string(), auth_value),
        ])
    }

    fn get_request_body(
        &self,
        req: &types::RefreshTokenRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let trustpay_req =
            utils::Encode::<trustpay::TrustpayAuthUpdateRequest>::convert_and_url_encode(req)
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
            message: response.result_info.additional_info.unwrap_or_default(),
            reason: None,
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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
            storage_models::enums::PaymentMethod::BankRedirect => Ok(format!(
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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
            storage_models::enums::PaymentMethod::BankRedirect => Ok(format!(
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
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let trustpay_req = trustpay::TrustpayPaymentsRequest::try_from(req)?;
        let trustpay_req_string = match req.payment_method {
            storage_models::enums::PaymentMethod::BankRedirect => {
                utils::Encode::<trustpay::PaymentRequestBankRedirect>::encode_to_string_of_json(
                    &trustpay_req,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
            }
            _ => utils::Encode::<trustpay::PaymentRequestCards>::url_encode(&trustpay_req)
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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
            storage_models::enums::PaymentMethod::BankRedirect => Ok(format!(
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
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let trustpay_req = trustpay::TrustpayRefundRequest::try_from(req)?;
        let trustpay_req_string = match req.payment_method {
            storage_models::enums::PaymentMethod::BankRedirect => utils::Encode::<
                trustpay::TrustpayRefundRequestBankRedirect,
            >::encode_to_string_of_json(
                &trustpay_req
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?,
            _ => utils::Encode::<trustpay::TrustpayRefundRequestCards>::url_encode(&trustpay_req)
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
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
            .ok_or_else(|| errors::ConnectorError::MissingConnectorRefundID)?;
        match req.payment_method {
            storage_models::enums::PaymentMethod::BankRedirect => Ok(format!(
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
                    api_models::payments::PaymentIdType::PaymentIntentId(
                        details.payment_information.references.merchant_reference,
                    ),
                ))
            }
            trustpay::CreditDebitIndicator::Dbit => {
                if details.payment_information.status == trustpay::WebhookStatus::Chargebacked {
                    Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                        api_models::payments::PaymentIdType::PaymentIntentId(
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
            _ => Err(errors::ConnectorError::WebhookEventTypeNotFound).into_report()?,
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

    async fn get_webhook_source_verification_merchant_secret(
        &self,
        db: &dyn crate::db::StorageInterface,
        merchant_id: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let key = format!("whsec_verification_{}_{}", self.id(), merchant_id);
        let secret = db
            .get_key(&key)
            .await
            .change_context(errors::ConnectorError::WebhookVerificationSecretNotFound)?;
        Ok(secret)
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
                "SuccessOk" => payments::CallConnectorAction::StatusUpdate(
                    storage_models::enums::AttemptStatus::Charged,
                ),
                _ => payments::CallConnectorAction::Trigger,
            },
        ))
    }
}
