pub mod transformers;
use std::fmt::Debug;

#[cfg(feature = "frm")]
use common_utils::request::RequestContent;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface};
use ring::hmac;
use transformers as riskified;

#[cfg(feature = "frm")]
use super::utils::FrmTransactionRouterDataRequest;
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    headers,
    services::{self, request, ConnectorIntegration, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
    },
};
#[cfg(feature = "frm")]
use crate::{
    events::connector_api_logs::ConnectorEvent,
    types::{api::fraud_check as frm_api, fraud_check as frm_types, ErrorResponse, Response},
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Riskified;

impl Riskified {
    pub fn generate_authorization_signature(
        &self,
        auth: &riskified::RiskifiedAuthType,
        payload: &str,
    ) -> CustomResult<String, errors::ConnectorError> {
        let key = hmac::Key::new(
            hmac::HMAC_SHA256,
            auth.secret_token.clone().expose().as_bytes(),
        );

        let signature_value = hmac::sign(&key, payload.as_bytes());

        let digest = signature_value.as_ref();

        Ok(hex::encode(digest))
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Riskified
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: riskified::RiskifiedAuthType =
            riskified::RiskifiedAuthType::try_from(&req.connector_auth_type)?;

        let riskified_req = self.get_request_body(req, connectors)?;

        let binding = types::RequestBody::get_inner_value(riskified_req);
        let payload = binding.peek();

        let digest = self
            .generate_authorization_signature(&auth, payload)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                "X-RISKIFIED-SHOP-DOMAIN".to_string(),
                auth.domain_name.clone().into(),
            ),
            (
                "X-RISKIFIED-HMAC-SHA256".to_string(),
                request::Mask::into_masked(digest),
            ),
            (
                "Accept".to_string(),
                "application/vnd.riskified.com; version=2".into(),
            ),
        ];

        Ok(header)
    }
}

impl ConnectorCommon for Riskified {
    fn id(&self) -> &'static str {
        "riskified"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }
    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.riskified.base_url.as_ref()
    }

    #[cfg(feature = "frm")]
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: riskified::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            attempt_status: None,
            code: crate::consts::NO_ERROR_CODE.to_string(),
            message: response.error.message.clone(),
            reason: None,
            connector_transaction_id: None,
        })
    }
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        frm_api::Checkout,
        frm_types::FraudCheckCheckoutData,
        frm_types::FraudCheckResponseData,
    > for Riskified
{
    fn get_headers(
        &self,
        req: &frm_types::FrmCheckoutRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &frm_types::FrmCheckoutRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "/decide"))
    }

    fn get_request_body(
        &self,
        req: &frm_types::FrmCheckoutRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = riskified::RiskifiedPaymentsCheckoutRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &frm_types::FrmCheckoutRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&frm_types::FrmCheckoutType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(frm_types::FrmCheckoutType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(frm_types::FrmCheckoutType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &frm_types::FrmCheckoutRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<frm_types::FrmCheckoutRouterData, errors::ConnectorError> {
        let response: riskified::RiskifiedPaymentsResponse = res
            .response
            .parse_struct("RiskifiedPaymentsResponse Checkout")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        <frm_types::FrmCheckoutRouterData>::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::Payment for Riskified {}
impl api::PaymentAuthorize for Riskified {}
impl api::PaymentSync for Riskified {}
impl api::PaymentVoid for Riskified {}
impl api::PaymentCapture for Riskified {}
impl api::MandateSetup for Riskified {}
impl api::ConnectorAccessToken for Riskified {}
impl api::PaymentToken for Riskified {}
impl api::Refund for Riskified {}
impl api::RefundExecute for Riskified {}
impl api::RefundSync for Riskified {}
impl ConnectorValidation for Riskified {}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        frm_api::Sale,
        frm_types::FraudCheckSaleData,
        frm_types::FraudCheckResponseData,
    > for Riskified
{
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        frm_api::Transaction,
        frm_types::FraudCheckTransactionData,
        frm_types::FraudCheckResponseData,
    > for Riskified
{
    fn get_headers(
        &self,
        req: &frm_types::FrmTransactionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &frm_types::FrmTransactionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        match req.is_payment_successful() {
            Some(false) => Ok(format!(
                "{}{}",
                self.base_url(connectors),
                "/checkout_denied"
            )),
            Some(true) => Ok(format!("{}{}", self.base_url(connectors), "/decision")),
            None => Err(errors::ConnectorError::FlowNotSupported {
                flow: "Transaction".to_owned(),
                connector: req.connector.to_string(),
            })?,
        }
    }

    fn get_request_body(
        &self,
        req: &frm_types::FrmTransactionRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        match req.is_payment_successful() {
            Some(false) => {
                let req_obj = riskified::TransactionFailedRequest::try_from(req)?;
                Ok(RequestContent::Json(Box::new(req_obj)))
            }
            Some(true) => {
                let req_obj = riskified::TransactionSuccessRequest::try_from(req)?;
                Ok(RequestContent::Json(Box::new(req_obj)))
            }
            None => Err(errors::ConnectorError::FlowNotSupported {
                flow: "Transaction".to_owned(),
                connector: req.connector.to_owned(),
            })?,
        }
    }

    fn build_request(
        &self,
        req: &frm_types::FrmTransactionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&frm_types::FrmTransactionType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(frm_types::FrmTransactionType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(frm_types::FrmTransactionType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &frm_types::FrmTransactionRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<frm_types::FrmTransactionRouterData, errors::ConnectorError> {
        let response: riskified::RiskifiedTransactionResponse = res
            .response
            .parse_struct("RiskifiedPaymentsResponse Transaction")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        match response {
            riskified::RiskifiedTransactionResponse::FailedResponse(response_data) => {
                <frm_types::FrmTransactionRouterData>::try_from(types::ResponseRouterData {
                    response: response_data,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            riskified::RiskifiedTransactionResponse::SuccessResponse(response_data) => {
                <frm_types::FrmTransactionRouterData>::try_from(types::ResponseRouterData {
                    response: response_data,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
        }
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        frm_api::Fulfillment,
        frm_types::FraudCheckFulfillmentData,
        frm_types::FraudCheckResponseData,
    > for Riskified
{
    fn get_headers(
        &self,
        req: &frm_types::FrmFulfillmentRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &frm_types::FrmFulfillmentRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "/fulfill"))
    }

    fn get_request_body(
        &self,
        req: &frm_types::FrmFulfillmentRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = riskified::RiskifiedFullfillmentRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &frm_types::FrmFulfillmentRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&frm_types::FrmFulfillmentType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(frm_types::FrmFulfillmentType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(frm_types::FrmFulfillmentType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &frm_types::FrmFulfillmentRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<frm_types::FrmFulfillmentRouterData, errors::ConnectorError> {
        let response: riskified::RiskifiedFulfilmentResponse = res
            .response
            .parse_struct("RiskifiedFulfilmentResponse fulfilment")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        frm_types::FrmFulfillmentRouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl
    ConnectorIntegration<
        frm_api::RecordReturn,
        frm_types::FraudCheckRecordReturnData,
        frm_types::FraudCheckResponseData,
    > for Riskified
{
}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Riskified
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Riskified
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Riskified
{
    fn build_request(
        &self,
        _req: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Riskified".to_string())
                .into(),
        )
    }
}

impl api::PaymentSession for Riskified {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Riskified
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Riskified
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Riskified
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Riskified
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Riskified
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Riskified
{
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Riskified
{
}

#[cfg(feature = "frm")]
impl api::FraudCheck for Riskified {}
#[cfg(feature = "frm")]
impl frm_api::FraudCheckSale for Riskified {}
#[cfg(feature = "frm")]
impl frm_api::FraudCheckCheckout for Riskified {}
#[cfg(feature = "frm")]
impl frm_api::FraudCheckTransaction for Riskified {}
#[cfg(feature = "frm")]
impl frm_api::FraudCheckFulfillment for Riskified {}
#[cfg(feature = "frm")]
impl frm_api::FraudCheckRecordReturn for Riskified {}

#[async_trait::async_trait]
impl api::IncomingWebhook for Riskified {
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
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
