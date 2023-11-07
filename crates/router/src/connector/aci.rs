mod result_codes;
pub mod transformers;
use std::fmt::Debug;

use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use transformers as aci;

use super::utils::PaymentsAuthorizeRequestData;
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon},
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Aci;

impl ConnectorCommon for Aci {
    fn id(&self) -> &'static str {
        "aci"
    }
    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }
    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.aci.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: aci::AciAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: aci::AciErrorResponse = res
            .response
            .parse_struct("AciErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.result.code,
            message: response.result.description,
            reason: response.result.parameter_errors.map(|errors| {
                errors
                    .into_iter()
                    .map(|error_description| {
                        format!(
                            "Field is {} and the message is {}",
                            error_description.name, error_description.message
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("; ")
            }),
        })
    }
}

impl ConnectorValidation for Aci {}

impl api::Payment for Aci {}

impl api::PaymentAuthorize for Aci {}
impl api::PaymentSync for Aci {}
impl api::PaymentVoid for Aci {}
impl api::PaymentCapture for Aci {}
impl api::PaymentSession for Aci {}
impl api::ConnectorAccessToken for Aci {}
impl api::PaymentToken for Aci {}

impl
    services::ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Aci
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Aci
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Aci
{
    // Not Implemented (R)
}

impl api::MandateSetup for Aci {}

impl
    services::ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Aci
{
    // Issue: #173
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Aci
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Aci
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
        let auth = aci::AciAuthType::try_from(&req.connector_auth_type)?;
        Ok(format!(
            "{}{}{}{}{}",
            self.base_url(connectors),
            "v1/payments/",
            req.request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
            "?entityId=",
            auth.entity_id.peek()
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
        types::PaymentsSyncData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: aci::AciPaymentsResponse =
            res.response
                .parse_struct("AciPaymentsResponse")
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
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Aci
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
        match req.request.connector_mandate_id() {
            Some(mandate_id) => Ok(format!(
                "{}v1/registrations/{}/payments",
                self.base_url(connectors),
                mandate_id
            )),
            _ => Ok(format!("{}{}", self.base_url(connectors), "v1/payments")),
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        // encode only for for urlencoded things.
        let connector_router_data = aci::AciRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = aci::AciPaymentsRequest::try_from(&connector_router_data)?;
        let aci_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<aci::AciPaymentsRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(aci_req))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
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
        let response: aci::AciPaymentsResponse =
            res.response
                .parse_struct("AciPaymentsResponse")
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
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Aci
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
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
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = &req.request.connector_transaction_id;
        Ok(format!("{}v1/payments/{}", self.base_url(connectors), id))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = aci::AciCancelRequest::try_from(req)?;
        let aci_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<aci::AciCancelRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(aci_req))
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

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: aci::AciPaymentsResponse =
            res.response
                .parse_struct("AciPaymentsResponse")
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
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Aci {}
impl api::RefundExecute for Aci {}
impl api::RefundSync for Aci {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Aci
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
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}v1/payments/{}",
            self.base_url(connectors),
            connector_payment_id,
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = aci::AciRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req = aci::AciRefundRequest::try_from(&connector_router_data)?;
        let body = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<aci::AciRefundRequest>::url_encode,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(body))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundExecuteType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::RefundExecuteType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: aci::AciRefundResponse = res
            .response
            .parse_struct("AciRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }
    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Aci
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Aci {
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
