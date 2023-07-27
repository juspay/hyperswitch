mod transformers;

use std::fmt::Debug;

use common_utils::{crypto, ext_traits::ByteSliceExt};
use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use transformers as payme;

use crate::{
    configs::settings,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, routes,
    services::{self, request, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Payme;

impl api::Payment for Payme {}
impl api::PaymentSession for Payme {}
impl api::ConnectorAccessToken for Payme {}
impl api::PreVerify for Payme {}
impl api::PaymentAuthorize for Payme {}
impl api::PaymentSync for Payme {}
impl api::PaymentCapture for Payme {}
impl api::PaymentVoid for Payme {}
impl api::Refund for Payme {}
impl api::RefundExecute for Payme {}
impl api::RefundSync for Payme {}
impl api::PaymentToken for Payme {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Payme
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Payme
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            Self::get_content_type(self).to_string().into(),
        )];
        Ok(header)
    }
}

impl ConnectorCommon for Payme {
    fn id(&self) -> &'static str {
        "payme"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }
    fn validate_auth_type(
        &self,
        val: &types::ConnectorAuthType,
    ) -> Result<(), error_stack::Report<errors::ConnectorError>> {
        payme::PaymeAuthType::try_from(val)?;
        Ok(())
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.payme.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: payme::PaymeErrorResponse =
            res.response
                .parse_struct("PaymeErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
        })
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Payme
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Payme
{
}

impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Payme
{
}

impl
    ConnectorIntegration<
        api::InitPayment,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Payme
{
    fn get_headers(
        &self,
        req: &types::PaymentsInitRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RouterData<
            api::InitPayment,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/generate-sale", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsInitRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = payme::GenerateSaleRequest::try_from(req)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::GenerateSaleRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsInitRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsInitType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsInitType::get_headers(self, req, connectors)?)
                .body(types::PaymentsInitType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsInitRouterData,
        res: Response,
    ) -> CustomResult<
        types::RouterData<
            api::InitPayment,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        errors::ConnectorError,
    >
    where
        api::InitPayment: Clone,
        types::PaymentsAuthorizeData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: payme::GenerateSaleResponse = res
            .response
            .parse_struct("Payme GenerateSaleResponse")
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
impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Payme
{
    async fn execute_pretasks(
        &self,
        router_data: &mut types::PaymentsAuthorizeRouterData,
        app_state: &routes::AppState,
    ) -> CustomResult<(), errors::ConnectorError> {
        if router_data.request.mandate_id.is_none() {
            let integ: Box<
                &(dyn ConnectorIntegration<
                    api::InitPayment,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                > + Send
                      + Sync
                      + 'static),
            > = Box::new(&Self);
            let init_data = &types::PaymentsInitRouterData::from((
                &router_data.to_owned(),
                router_data.request.clone(),
            ));
            let init_res = services::execute_connector_processing_step(
                app_state,
                integ,
                init_data,
                payments::CallConnectorAction::Trigger,
                None,
            )
            .await?;
            router_data.request.related_transaction_id = init_res.request.related_transaction_id;
        }
        Ok(())
    }

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
        if req.request.mandate_id.is_some() {
            // For recurring mandate payments
            Ok(format!("{}api/generate-sale", self.base_url(connectors)))
        } else {
            // For Normal & first mandate payments
            Ok(format!("{}api/pay-sale", self.base_url(connectors)))
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = payme::PaymePaymentRequest::try_from(req)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::PayRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
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
        let response: payme::PaymePaySaleResponse = res
            .response
            .parse_struct("Payme PaymentsAuthorizeResponse")
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
    for Payme
{
    fn build_request(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Payment Sync".to_string(),
            connector: "Payme".to_string(),
        }
        .into())
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        res: Response,
    ) -> CustomResult<
        types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        errors::ConnectorError,
    >
    where
        api::PSync: Clone,
        types::PaymentsSyncData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: payme::PaymePaySaleResponse = res
            .response
            .parse_struct("Payme PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Payme
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
        _req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/capture-sale", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = payme::PaymentCaptureRequest::try_from(req)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::PaymentCaptureRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
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
                .attach_default_headers()
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
        let response: payme::PaymePaySaleResponse = res
            .response
            .parse_struct("Payme PaymentsCaptureResponse")
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
    for Payme
{
    fn build_request(
        &self,
        _req: &types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Void".to_string(),
            connector: "Payme".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Payme {
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
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/refund-sale", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = payme::PaymeRefundRequest::try_from(req)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::PaymeRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
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
        let response: payme::RefundResponse = res
            .response
            .parse_struct("payme RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from((
            &data.request,
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Payme {
    fn build_request(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Refund Sync".to_string(),
            connector: "Payme".to_string(),
        }
        .into())
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Payme {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::Md5))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let resource: payme::WebhookEventDataResourceSignature = request
            .body
            .parse_struct("WebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(resource.payme_signature.expose().into_bytes())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        secret: &[u8],
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let resource: payme::WebhookEventDataResource =
            request
                .body
                .parse_struct("WebhookEvent")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(format!(
            "{}{}{}",
            String::from_utf8_lossy(secret),
            resource.payme_transaction_id,
            resource.payme_sale_id
        )
        .as_bytes()
        .to_vec())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let resource: payme::WebhookEventDataResource =
            request
                .body
                .parse_struct("WebhookEvent")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let id = match resource.notify_type {
            transformers::NotifyType::SaleComplete
            | transformers::NotifyType::SaleAuthorized
            | transformers::NotifyType::SaleFailure => api::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(resource.payme_sale_id),
            ),
            transformers::NotifyType::Refund => api::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::ConnectorRefundId(resource.payme_sale_id),
            ),
            transformers::NotifyType::SaleChargeback
            | transformers::NotifyType::SaleChargebackRefund => {
                api::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        resource.payme_sale_id,
                    ),
                )
            }
        };
        Ok(id)
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let resource: payme::WebhookEventDataResourceEvent = request
            .body
            .parse_struct("WebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(api::IncomingWebhookEvent::from(resource.notify_type))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let resource: payme::WebhookEventDataResource =
            request
                .body
                .parse_struct("WebhookEvent")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let sale_response = payme::PaymePaySaleResponse::try_from(resource)?;

        let res_json = serde_json::to_value(sale_response)
            .into_report()
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(res_json)
    }
}
