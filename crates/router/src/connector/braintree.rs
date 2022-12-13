mod transformers;

use std::fmt::Debug;

use bytes::Bytes;
use error_stack::ResultExt;

use self::{braintree::BraintreeAuthType, transformers as braintree};
use crate::{
    configs::settings::Connectors,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger, services,
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Braintree;

impl api::ConnectorCommon for Braintree {
    fn id(&self) -> &'static str {
        "braintree"
    }

    fn base_url(&self, connectors: Connectors) -> String {
        connectors.braintree.base_url
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth: braintree::BraintreeAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key)])
    }
}

impl api::Payment for Braintree {}

impl api::PaymentAuthorize for Braintree {}
impl api::PaymentSync for Braintree {}
impl api::PaymentVoid for Braintree {}
impl api::PaymentCapture for Braintree {}

impl api::PaymentSession for Braintree {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsSessionType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
            (headers::X_API_VERSION.to_string(), "6".to_string()),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(format!(
            "{}/merchants/{}/client_token",
            self.base_url(connectors),
            auth_type.merchant_account,
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsSessionType::get_url(self, req, connectors)?)
                .headers(types::PaymentsSessionType::get_headers(self, req)?)
                .body(types::PaymentsSessionType::get_request_body(self, req)?)
                .build(),
        );

        logger::debug!(session_request=?request);
        Ok(request)
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: braintree::ErrorResponse = res
            .parse_struct("Error Response")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            code: consts::NO_ERROR_CODE.to_string(),
            message: response.api_error_response.message,
            reason: None,
        })
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSessionRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSessionRouterData, errors::ConnectorError> {
        logger::debug!(payment_session_response_braintree=?res);
        let response: braintree::BraintreeSessionTokenResponse = res
            .response
            .parse_struct("braintree SessionTokenReponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::PreVerify for Braintree {}

#[allow(dead_code)]
impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Braintree
{
    // Not Implemented (R)
}

#[allow(dead_code)]
impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Braintree
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsSyncType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
            (headers::X_API_VERSION.to_string(), "6".to_string()),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/merchants/{}/transactions/{}",
            self.base_url(connectors),
            auth_type.merchant_account,
            connector_payment_id
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .headers(types::PaymentsSyncType::get_headers(self, req)?)
                .body(types::PaymentsSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: braintree::ErrorResponse = res
            .parse_struct("Braintree Error Response")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            code: consts::NO_ERROR_CODE.to_string(),
            message: response.api_error_response.message,
            reason: None,
        })
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        logger::debug!(payment_sync_response=?res);
        let response: braintree::BraintreePaymentsResponse = res
            .response
            .parse_struct("Braintree PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
            (headers::X_API_VERSION.to_string(), "6".to_string()),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth_type = BraintreeAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        Ok(format!(
            "{}merchants/{}/transactions",
            self.base_url(connectors),
            auth_type.merchant_account
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .headers(types::PaymentsAuthorizeType::get_headers(self, req)?)
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let braintree_req =
            utils::Encode::<braintree::BraintreePaymentsRequest>::convert_and_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(braintree_req))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: braintree::BraintreePaymentsResponse = res
            .response
            .parse_struct("Braintree PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(braintreepayments_create_response=?response);
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
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        logger::debug!(braintreepayments_create_response=?res);

        let response: braintree::ErrorResponse = res
            .parse_struct("Braintree ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: consts::NO_ERROR_CODE.to_string(),
            message: response.api_error_response.message,
            reason: None,
        })
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
            (headers::X_API_VERSION.to_string(), "6".to_string()),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(format!(
            "{}merchants/{}/transactions/{}/void",
            self.base_url(connectors),
            auth_type.merchant_account,
            req.request.connector_transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Put)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .headers(types::PaymentsVoidType::get_headers(self, req)?)
                .body(types::PaymentsVoidType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: braintree::ErrorResponse = res
            .parse_struct("Braintree ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            code: consts::NO_ERROR_CODE.to_string(),
            message: response.api_error_response.message,
            reason: None,
        })
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        logger::debug!(payment_sync_response=?res);
        let response: braintree::BraintreePaymentsResponse = res
            .response
            .parse_struct("Braintree PaymentsVoidResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::Refund for Braintree {}
impl api::RefundExecute for Braintree {}
impl api::RefundSync for Braintree {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefundExecuteType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
            (headers::X_API_VERSION.to_string(), "6".to_string()),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth_type = BraintreeAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}merchants/{}/transactions/{}",
            self.base_url(connectors),
            auth_type.merchant_account,
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let braintree_req =
            utils::Encode::<braintree::BraintreeRefundRequest>::convert_and_url_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(braintree_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .headers(types::RefundExecuteType::get_headers(self, req)?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        logger::debug!(target: "router::connector::braintree", response=?res);
        let response: braintree::RefundResponse = res
            .response
            .parse_struct("Braintree RefundResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
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
        _res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }
}

#[allow(dead_code)]
impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        _req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }

    fn get_content_type(&self) -> &'static str {
        ""
    }

    fn get_url(
        &self,
        _req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        _connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }

    fn get_error_response(
        &self,
        _res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn build_request(
        &self,
        _req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        _connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        res: Response,
    ) -> CustomResult<
        types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        errors::ConnectorError,
    > {
        logger::debug!(target: "router::connector::braintree", response=?res);
        let response: braintree::RefundResponse = res
            .response
            .parse_struct("Braintree RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Braintree {
    fn get_webhook_object_reference_id(
        &self,
        _body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }

    fn get_webhook_event_type(
        &self,
        _body: &[u8],
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }
}

impl services::ConnectorRedirectResponse for Braintree {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}
