mod transformers;

use std::fmt::Debug;

use bytes::Bytes;
use error_stack::ResultExt;

use crate::{
    configs::settings::Connectors,
    consts,
    utils::{self, BytesExt},
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
};


use transformers as braintree;

use self::braintree::BraintreeAuthType;

#[derive(Debug, Clone)]
pub struct Braintree;

impl api::ConnectorCommon for Braintree {
    fn id(&self) -> &'static str {
        "braintree"
    }

    fn base_url(&self, connectors: Connectors) -> String {
        connectors.braintree.base_url
    }

    fn get_auth_header(&self, auth_type:&types::ConnectorAuthType)-> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
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


#[allow(dead_code)]
type PCapture = dyn services::ConnectorIntegration<
    api::PCapture,
    types::PaymentsRequestCaptureData,
    types::PaymentsResponseData,
>;
impl
    services::ConnectorIntegration<
        api::PCapture,
        types::PaymentsRequestCaptureData,
        types::PaymentsResponseData,
    > for Braintree
{
    // Not Implemented (R)
}


type PSync = dyn services::ConnectorIntegration<
    api::PSync,
    types::PaymentsRequestSyncData,
    types::PaymentsResponseData,
>;
impl
    services::ConnectorIntegration<
        api::PSync,
        types::PaymentsRequestSyncData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::RouterData<
            api::PSync,
            types::PaymentsRequestSyncData,
            types::PaymentsResponseData,
        >,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![
            (headers::CONTENT_TYPE.to_string(), Authorize::get_content_type(self).to_string()),
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
        req: &types::RouterData<
            api::PSync,
            types::PaymentsRequestSyncData,
            types::PaymentsResponseData,
        >,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth_type =
            BraintreeAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!("{}/merchants/{}/transactions/{}", self.base_url(connectors), auth_type.merchant_account, connector_payment_id))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::PSync,
            types::PaymentsRequestSyncData,
            types::PaymentsResponseData,
        >,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&PSync::get_url(self, req, connectors)?)
                .headers(PSync::get_headers(self, req)?)
                .body(PSync::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let _: braintree::ErrorResponse = res
            .parse_struct("Error Response")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            code: consts::NO_ERROR_CODE.to_string(),
            message: consts::NO_ERROR_MESSAGE.to_string(),
            reason: None,
        })
    }

    fn get_request_body(
        &self,
        _req: &types::RouterData<api::PSync, types::PaymentsRequestSyncData, types::PaymentsResponseData>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::PSync, types::PaymentsRequestSyncData, types::PaymentsResponseData>,
        res: Response,
    ) -> CustomResult<types::RouterData<api::PSync, types::PaymentsRequestSyncData, types::PaymentsResponseData>, errors::ConnectorError>
    {
        logger::debug!(payment_sync_response=?res);
        let response: braintree::BraintreePaymentsResponse = res
            .response
            .parse_struct("braintree PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(res=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

}

type Authorize = dyn services::ConnectorIntegration<
    api::Authorize,
    types::PaymentsRequestData,
    types::PaymentsResponseData,
>;
impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsRequestData,
        types::PaymentsResponseData,
    > for Braintree {
    fn get_headers(&self, req: &types::PaymentsRouterData) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        let mut headers = vec![
            (headers::CONTENT_TYPE.to_string(), Authorize::get_content_type(self).to_string()),
            (headers::X_ROUTER.to_string(), "test".to_string()),
            (headers::X_API_VERSION.to_string(), "6".to_string()),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)    }

    fn get_url(&self, req: &types::PaymentsRouterData, connectors: Connectors) -> CustomResult<String,errors::ConnectorError> {
        let auth_type =
            BraintreeAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        Ok(format!("{}merchants/{}/transactions", self.base_url(connectors), auth_type.merchant_account))
    }

    fn build_request(
        &self,
        req: &types::RouterData<
            api::Authorize,
            types::PaymentsRequestData,
            types::PaymentsResponseData,
        >,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                // TODO: [ORCA-346] Requestbuilder needs &str migrate get_url to send &str instead of owned string
                .url(&Authorize::get_url(self, req, connectors)?)
                .headers(Authorize::get_headers(self, req)?)
                .body(Authorize::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_request_body(&self, req: &types::PaymentsRouterData) -> CustomResult<Option<String>,errors::ConnectorError> {
        let braintree_req =
            utils::Encode::<braintree::BraintreePaymentsRequest>::convert_and_encode(req).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(braintree_req))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsRouterData,errors::ConnectorError> {
        let response: braintree::BraintreePaymentsResponse = res.response.parse_struct("Braintree Payments Response").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(braintreepayments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, res: Bytes) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        logger::debug!(braintreepayments_create_response=?res);

        let response: braintree::ErrorResponse = res
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code:  consts::NO_ERROR_CODE.to_string(),
            message: response.api_error_response.message,
            reason: None,
        })
    }
}

#[allow(dead_code)]
type Void = dyn services::ConnectorIntegration<
    api::Void,
    types::PaymentRequestCancelData,
    types::PaymentsResponseData,
>;

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentRequestCancelData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        _req: &types::PaymentRouterCancelData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }

    fn get_content_type(&self) -> &'static str {
        ""
    }

    fn get_url(
        &self,
        _req: &types::PaymentRouterCancelData,
        _connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentRouterCancelData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }
    fn build_request(
        &self,
        _req: &types::PaymentRouterCancelData,
        _connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }

    fn handle_response(
        &self,
        _data: &types::PaymentRouterCancelData,
        _res: Response,
    ) -> CustomResult<types::PaymentRouterCancelData, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }

    fn get_error_response(
        &self,
        _res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("braintree".to_string()).into())
    }
}

impl api::Refund for Braintree {}
impl api::RefundExecute for Braintree {}
impl api::RefundSync for Braintree {}

type Execute = dyn services::ConnectorIntegration<
    api::Execute,
    types::RefundsRequestData,
    types::RefundsResponseData,
>;

impl
    services::ConnectorIntegration<
        api::Execute,
        types::RefundsRequestData,
        types::RefundsResponseData,
    > for Braintree {
    fn get_headers(&self, req: &types::RefundsRouterData<api::Execute>) -> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        let mut headers = vec![
            (headers::CONTENT_TYPE.to_string(), Authorize::get_content_type(self).to_string()),
            (headers::X_ROUTER.to_string(), "test".to_string()),
            (headers::X_API_VERSION.to_string(), "6".to_string()),
            (headers::ACCEPT.to_string(), "application/json".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        _connectors: Connectors,
    ) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(&self, req: &types::RefundsRouterData<api::Execute>) -> CustomResult<Option<String>,errors::ConnectorError> {
        let braintree_req = utils::Encode::<braintree::RefundRequest>::convert_and_url_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(braintree_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>,errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&Execute::get_url(self, req, connectors)?)
            .headers(Execute::get_headers(self, req)?)
            .body(Execute::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>,errors::ConnectorError> {
        logger::debug!(target: "router::connector::braintree", response=?res);
        let response: braintree::RefundResponse = res.response.parse_struct("braintree RefundResponse").change_context(errors::ConnectorError::RequestEncodingFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, _res: Bytes) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        todo!()
    }
}

#[allow(dead_code)]
type RSync = dyn services::ConnectorIntegration<
    api::RSync,
    types::RefundsRequestData,
    types::RefundsResponseData,
>;
impl
    services::ConnectorIntegration<api::RSync, types::RefundsRequestData, types::RefundsResponseData> for Braintree {
    fn get_headers(&self, _req: &types::RouterData<api::RSync, types::RefundsRequestData, types::RefundsResponseData>) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(
        &self,
        _req: &types::RouterData<api::RSync, types::RefundsRequestData, types::RefundsResponseData>,
        _connectors: Connectors,
    ) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn get_error_response(&self, _res: Bytes) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(
        &self,
        _req: &types::RouterData<api::RSync, types::RefundsRequestData, types::RefundsResponseData>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn build_request(
        &self,
        _req: &types::RouterData<api::RSync, types::RefundsRequestData, types::RefundsResponseData>,
        _connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::RSync, types::RefundsRequestData, types::RefundsResponseData>,
        res: Response,
    ) -> CustomResult<types::RouterData<api::RSync, types::RefundsRequestData, types::RefundsResponseData>, errors::ConnectorError>
    {
        logger::debug!(target: "router::connector::braintree", response=?res);
        let response: braintree::RefundResponse = res.response.parse_struct("braintree RefundResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
        todo!()
    }

    fn get_webhook_event_type(
        &self,
        _body: &[u8],
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        todo!()
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        todo!()
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
