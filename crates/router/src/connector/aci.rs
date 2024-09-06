mod result_codes;
pub mod transformers;
use std::marker::PhantomData;

use common_utils::{
    {request::RequestContent,
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
}, types::StringMajorUnit};
use error_stack::{report, ResultExt};
use masking::PeekInterface;

use super::utils::{convert_amount, is_mandate_supported, PaymentsAuthorizeRequestData};
use crate::{
    connector::{aci::transformers as aci, utils::PaymentMethodDataType},
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers,
    services::{
        request::{self, Mask},
        ConnectorValidation,
    },
    types,
    types::api::ConnectorCommon,
    utils::BytesExt,
};

super::macros::create_all_prerequisites2!(
    connector_name: Aci,
    api: [
        (
            flow: Authorize,
            request_body: AciPaymentsRequest,
            response_body: AciPaymentsResponse,
            router_data: types::PaymentsAuthorizeRouterData
        ),
        (flow: Void,request_body: AciCancelRequest, response_body:AciPaymentsResponse, router_data:types::PaymentsCancelRouterData),
        (flow: Execute, request_body:AciRefundRequest, response_body:AciRefundResponse, router_data:types::RefundsRouterData<api::Execute>),
        (flow: PSync, request_body: NoRequestBody, response_body:AciPaymentsResponse, router_data:types::PaymentsSyncRouterData)
    ],
    amount_converters: [
        amount_converter: StringMajorUnit
    ]
);
super::macros::create_module_and_template_for_request_and_response_types!(
    connector_types: {
        (
            path: crate::connector::aci::transformers,
            types: {
                AciPaymentsRequest,
                AciCancelRequest,
                AciPaymentsResponse,
                AciRefundRequest,
                AciRefundResponse
            }
        ),
    },
    domain_types: {
        (
            path: crate::types::api,
            types: {
                Authorize,
                PSync,
                Execute,
                Capture,
                Void
            }
        ),
    }

);

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
        let auth = aci::AciAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: aci::AciErrorResponse = res
            .response
            .parse_struct("AciErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);
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
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Aci {
    fn validate_mandate_payment(
        &self,
        pm_type: Option<types::storage::enums::PaymentMethodType>,
        pm_data: types::domain::payments::PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([PaymentMethodDataType::Card]);
        is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

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
    fn build_request(
        &self,
        _req: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Setup Mandate flow for Aci".to_string()).into())
    }
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
super::macros::impl_templating!(
    connector: Aci,
    // flow: PSync,
    curl_response: AciPaymentsResponse,
    router_data: types::PaymentsSyncRouterData
);
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

    // fn get_content_type(&self) -> &'static str {
    //     self.common_get_content_type()
    // }

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
    macro_connector_implementation!(
        connector_default_implementations: [get_content_type, get_error_response],
        connector: Aci,
        curl_response:AciPaymentsResponse,
        flow_name: PSync,
        resource_common_data: types::PaymentFlowData,
        flow_request: types::PaymentsSyncData,
        flow_response: types::PaymentsResponseData,
    );

    // fn build_request(
    //     &self,
    //     req: &types::PaymentsSyncRouterData,
    //     connectors: &settings::Connectors,
    // ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
    //     Ok(Some(
    //         services::RequestBuilder::new()
    //             .method(services::Method::Get)
    //             .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
    //             .attach_default_headers()
    //             .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
    //             .build(),
    //     ))
    // }

    // fn handle_response(
    //     &self,
    //     data: &types::PaymentsSyncRouterData,
    //     event_builder: Option<&mut ConnectorEvent>,
    //     res: types::Response,
    // ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError>
    // where
    //     types::PaymentsSyncData: Clone,
    //     types::PaymentsResponseData: Clone,
    // {
    //     let response: aci::AciPaymentsResponse =
    //         res.response
    //             .parse_struct("AciPaymentsResponse")
    //             .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
    //     event_builder.map(|i| i.set_response_body(&response));
    //     router_env::logger::info!(connector_response=?response);
    //     types::RouterData::try_from(types::ResponseRouterData {
    //         response,
    //         data: data.clone(),
    //         http_code: res.status_code,
    //     })
    //     .change_context(errors::ConnectorError::ResponseHandlingFailed)
    // }

    // fn get_error_response(
    //     &self,
    //     res: types::Response,
    //     event_builder: Option<&mut ConnectorEvent>,
    // ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
    //     self.build_error_response(res, event_builder)
    // }
}

impl TryFrom<aci::AciPaymentsResponse> for types::PaymentsAuthorizeRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(value: aci::AciPaymentsResponse) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<AciInputData<types::PaymentsAuthorizeRouterData>> for aci::AciPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: AciInputData<types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<AciInputData<types::PaymentsCancelRouterData>> for aci::AciCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(value: AciInputData<types::PaymentsCancelRouterData>) -> Result<Self, Self::Error> {
        todo!()
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

    // fn get_content_type(&self) -> &'static str {
    //     self.common_get_content_type()
    // }

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
    macro_connector_implementation!(
        connector_default_implementations: [get_content_type, get_error_response],
        connector: Aci,
        curl_request: FormUrlEncoded(AciPaymentsRequest),
        curl_response:AciPaymentsResponse,
        flow_name: Authorize,
        resource_common_data: types::PaymentFlowData,
        flow_request: types::PaymentsAuthorizeData,
        flow_response: types::PaymentsResponseData,
    );
    // fn get_error_response(
    //     &self,
    //     res: types::Response,
    //     event_builder: Option<&mut ConnectorEvent>,
    // ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
    //     self.build_error_response(res, event_builder)
    // }
}

super::macros::impl_templating!(
    connector: Aci,
    curl_request: AciPaymentsRequest,
    curl_response: AciPaymentsResponse,
    router_data: types::PaymentsAuthorizeRouterData
);

super::macros::impl_templating!(
    connector: Aci,
    curl_request: AciCancelRequest,
    curl_response: AciPaymentsResponse,
    router_data: types::PaymentsCancelRouterData
);

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

    // fn get_content_type(&self) -> &'static str {
    //     self.common_get_content_type()
    // }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = &req.request.connector_transaction_id;
        Ok(format!("{}v1/payments/{}", self.base_url(connectors), id))
    }

    macro_connector_implementation!(
        connector_default_implementations: [get_content_type,get_error_response],
        connector: Aci,
        curl_request: FormUrlEncoded(AciCancelRequest),
        curl_response:AciPaymentsResponse,
        flow_name: Void,
        resource_common_data: types::PaymentFlowData,
        flow_request: types::PaymentsCancelData,
        flow_response: types::PaymentsResponseData,
    );

    // fn get_request_body(
    //     &self,
    //     req: &types::PaymentsCancelRouterData,
    //     _connectors: &settings::Connectors,
    // ) -> CustomResult<RequestContent, errors::ConnectorError> {
    //     let connector_req = aci::AciCancelRequest::try_from(req)?;
    //     Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    // }
    // fn build_request(
    //     &self,
    //     req: &types::PaymentsCancelRouterData,
    //     connectors: &settings::Connectors,
    // ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
    //     Ok(Some(
    //         services::RequestBuilder::new()
    //             .method(services::Method::Post)
    //             .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
    //             .attach_default_headers()
    //             .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
    //             .set_body(types::PaymentsVoidType::get_request_body(
    //                 self, req, connectors,
    //             )?)
    //             .build(),
    //     ))
    // }

    // fn handle_response(
    //     &self,
    //     data: &types::PaymentsCancelRouterData,
    //     event_builder: Option<&mut ConnectorEvent>,
    //     res: types::Response,
    // ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
    //     let response: aci::AciPaymentsResponse =
    //         res.response
    //             .parse_struct("AciPaymentsResponse")
    //             .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
    //     event_builder.map(|i| i.set_response_body(&response));
    //     router_env::logger::info!(connector_response=?response);
    //     types::RouterData::try_from(types::ResponseRouterData {
    //         response,
    //         data: data.clone(),
    //         http_code: res.status_code,
    //     })
    //     .change_context(errors::ConnectorError::ResponseHandlingFailed)
    // }

    // fn get_error_response(
    //     &self,
    //     res: types::Response,
    //     event_builder: Option<&mut ConnectorEvent>,
    // ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
    //     self.build_error_response(res, event_builder)
    // }
}

impl api::Refund for Aci {}
impl api::RefundExecute for Aci {}
impl api::RefundSync for Aci {}

super::macros::impl_templating!(
    connector: Aci,
    // flow: Execute,
    curl_request: AciRefundRequest,
    curl_response: AciRefundResponse,
    router_data: types::RefundsRouterData<api::Execute>
);
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

    // fn get_content_type(&self) -> &'static str {
    //     self.common_get_content_type()
    // }

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
    macro_connector_implementation!(
        connector_default_implementations: [get_content_type,get_error_response],
        connector: Aci,
        curl_request: FormUrlEncoded(AciRefundRequest),
        curl_response:AciRefundResponse,
        flow_name: Execute,
        resource_common_data: types::PaymentFlowData,
        flow_request: types::RefundsData,
        flow_response: types::RefundsResponseData,
    );

    // fn get_request_body(
    //     &self,
    //     req: &types::RefundsRouterData<api::Execute>,
    //     _connectors: &settings::Connectors,
    // ) -> CustomResult<RequestContent, errors::ConnectorError> {
    //     let connector_router_data = aci::AciRouterData::try_from((
    //         &self.get_currency_unit(),
    //         req.request.currency,
    //         req.request.refund_amount,
    //         req,
    //     ))?;
    //     let connector_req = aci::AciRefundRequest::try_from(&connector_router_data)?;
    //     Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    // }

    // fn build_request(
    //     &self,
    //     req: &types::RefundsRouterData<api::Execute>,
    //     connectors: &settings::Connectors,
    // ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
    //     Ok(Some(
    //         services::RequestBuilder::new()
    //             .method(services::Method::Post)
    //             .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
    //             .attach_default_headers()
    //             .headers(types::RefundExecuteType::get_headers(
    //                 self, req, connectors,
    //             )?)
    //             .set_body(types::RefundExecuteType::get_request_body(
    //                 self, req, connectors,
    //             )?)
    //             .build(),
    //     ))
    // }

    // fn handle_response(
    //     &self,
    //     data: &types::RefundsRouterData<api::Execute>,
    //     event_builder: Option<&mut ConnectorEvent>,
    //     res: types::Response,
    // ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
    //     let response: aci::AciRefundResponse = res
    //         .response
    //         .parse_struct("AciRefundResponse")
    //         .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

    //     event_builder.map(|i| i.set_response_body(&response));
    //     router_env::logger::info!(connector_response=?response);
    //     types::RouterData::try_from(types::ResponseRouterData {
    //         response,
    //         data: data.clone(),
    //         http_code: res.status_code,
    //     })
    //     .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    // }
    // fn get_error_response(
    //     &self,
    //     res: types::Response,
    //     event_builder: Option<&mut ConnectorEvent>,
    // ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
    //     self.build_error_response(res, event_builder)
    // }
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
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
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
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}
