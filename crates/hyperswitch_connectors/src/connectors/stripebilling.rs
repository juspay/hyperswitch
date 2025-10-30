pub mod transformers;

use std::collections::HashMap;

use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
};
use error_stack::{report, ResultExt};
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::revenue_recovery;
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::types as recovery_router_data_types;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        revenue_recovery as recovery_router_flows, subscriptions as subscription_flow_types,
    },
    router_request_types::{
        revenue_recovery as recovery_request_types, subscriptions as subscription_request_types,
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        revenue_recovery as recovery_response_types, subscriptions as subscription_response_types,
        ConnectorInfo, PaymentsResponseData, RefundsResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, subscriptions as subscriptions_api, ConnectorCommon, ConnectorCommonExt,
        ConnectorIntegration, ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{Mask, PeekInterface};
use stripebilling::auth_headers;
use transformers as stripebilling;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Stripebilling {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Stripebilling {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

impl api::Payment for Stripebilling {}
impl api::PaymentSession for Stripebilling {}
impl api::ConnectorAccessToken for Stripebilling {}
impl api::MandateSetup for Stripebilling {}
impl api::PaymentAuthorize for Stripebilling {}
impl api::PaymentSync for Stripebilling {}
impl api::PaymentCapture for Stripebilling {}
impl api::PaymentVoid for Stripebilling {}
impl api::Refund for Stripebilling {}
impl api::RefundExecute for Stripebilling {}
impl api::RefundSync for Stripebilling {}
impl api::PaymentToken for Stripebilling {}
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl api::revenue_recovery::RevenueRecoveryRecordBack for Stripebilling {}
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl api::revenue_recovery::BillingConnectorPaymentsSyncIntegration for Stripebilling {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Stripebilling
{
    // Not Implemented (R)
}

impl subscriptions_api::Subscriptions for Stripebilling {}
impl subscriptions_api::GetSubscriptionPlansFlow for Stripebilling {}
impl subscriptions_api::SubscriptionRecordBackFlow for Stripebilling {}
impl subscriptions_api::SubscriptionCreate for Stripebilling {}
impl
    ConnectorIntegration<
        subscription_flow_types::GetSubscriptionPlans,
        subscription_request_types::GetSubscriptionPlansRequest,
        subscription_response_types::GetSubscriptionPlansResponse,
    > for Stripebilling
{
}
impl subscriptions_api::GetSubscriptionPlanPricesFlow for Stripebilling {}
impl
    ConnectorIntegration<
        subscription_flow_types::GetSubscriptionPlanPrices,
        subscription_request_types::GetSubscriptionPlanPricesRequest,
        subscription_response_types::GetSubscriptionPlanPricesResponse,
    > for Stripebilling
{
}
impl
    ConnectorIntegration<
        subscription_flow_types::SubscriptionCreate,
        subscription_request_types::SubscriptionCreateRequest,
        subscription_response_types::SubscriptionCreateResponse,
    > for Stripebilling
{
}
impl subscriptions_api::GetSubscriptionEstimateFlow for Stripebilling {}
impl
    ConnectorIntegration<
        subscription_flow_types::GetSubscriptionEstimate,
        subscription_request_types::GetSubscriptionEstimateRequest,
        subscription_response_types::GetSubscriptionEstimateResponse,
    > for Stripebilling
{
}

impl subscriptions_api::SubscriptionCancelFlow for Stripebilling {}
impl
    ConnectorIntegration<
        subscription_flow_types::SubscriptionCancel,
        subscription_request_types::SubscriptionCancelRequest,
        subscription_response_types::SubscriptionCancelResponse,
    > for Stripebilling
{
}
impl subscriptions_api::SubscriptionPauseFlow for Stripebilling {}
impl
    ConnectorIntegration<
        subscription_flow_types::SubscriptionPause,
        subscription_request_types::SubscriptionPauseRequest,
        subscription_response_types::SubscriptionPauseResponse,
    > for Stripebilling
{
}
impl subscriptions_api::SubscriptionResumeFlow for Stripebilling {}
impl
    ConnectorIntegration<
        subscription_flow_types::SubscriptionResume,
        subscription_request_types::SubscriptionResumeRequest,
        subscription_response_types::SubscriptionResumeResponse,
    > for Stripebilling
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Stripebilling
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Stripebilling {
    fn id(&self) -> &'static str {
        "stripebilling"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.stripebilling.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = stripebilling::StripebillingAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", auth.api_key.peek()).into_masked(),
            ),
            (
                auth_headers::STRIPE_API_VERSION.to_string(),
                auth_headers::STRIPE_VERSION.to_string().into_masked(),
            ),
        ])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: stripebilling::StripebillingErrorResponse = res
            .response
            .parse_struct("StripebillingErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Stripebilling {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Stripebilling {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Stripebilling {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Stripebilling
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Stripebilling
{
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = stripebilling::StripebillingRouterData::from((amount, req));
        let connector_req =
            stripebilling::StripebillingPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: stripebilling::StripebillingPaymentsResponse = res
            .response
            .parse_struct("Stripebilling PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Stripebilling {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: stripebilling::StripebillingPaymentsResponse = res
            .response
            .parse_struct("stripebilling PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Stripebilling {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsCaptureType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: stripebilling::StripebillingPaymentsResponse = res
            .response
            .parse_struct("Stripebilling PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Stripebilling {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Stripebilling {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data =
            stripebilling::StripebillingRouterData::from((refund_amount, req));
        let connector_req =
            stripebilling::StripebillingRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::RefundExecuteType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: stripebilling::RefundResponse = res
            .response
            .parse_struct("stripebilling RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Stripebilling {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .set_body(types::RefundSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: stripebilling::RefundResponse = res
            .response
            .parse_struct("stripebilling RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
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

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    ConnectorIntegration<
        recovery_router_flows::BillingConnectorPaymentsSync,
        recovery_request_types::BillingConnectorPaymentsSyncRequest,
        recovery_response_types::BillingConnectorPaymentsSyncResponse,
    > for Stripebilling
{
    fn get_headers(
        &self,
        req: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}v1/charges/{}",
            self.base_url(connectors),
            req.request.billing_connector_psync_id
        ))
    }

    fn build_request(
        &self,
        req: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&types::BillingConnectorPaymentsSyncType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::BillingConnectorPaymentsSyncType::get_headers(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        recovery_router_data_types::BillingConnectorPaymentsSyncRouterData,
        errors::ConnectorError,
    > {
        let response: stripebilling::StripebillingRecoveryDetailsData = res
            .response
            .parse_struct::<stripebilling::StripebillingRecoveryDetailsData>(
                "StripebillingRecoveryDetailsData",
            )
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        recovery_router_data_types::BillingConnectorPaymentsSyncRouterData::try_from(
            ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
        )
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "v1")]
impl
    ConnectorIntegration<
        recovery_router_flows::InvoiceRecordBack,
        recovery_request_types::InvoiceRecordBackRequest,
        recovery_response_types::InvoiceRecordBackResponse,
    > for Stripebilling
{
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    ConnectorIntegration<
        recovery_router_flows::InvoiceRecordBack,
        recovery_request_types::InvoiceRecordBackRequest,
        recovery_response_types::InvoiceRecordBackResponse,
    > for Stripebilling
{
    fn get_headers(
        &self,
        req: &recovery_router_data_types::InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &recovery_router_data_types::InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let invoice_id = req
            .request
            .merchant_reference_id
            .get_string_repr()
            .to_string();
        match req.request.attempt_status {
            common_enums::AttemptStatus::Charged => Ok(format!(
                "{}/v1/invoices/{invoice_id}/pay?paid_out_of_band=true",
                self.base_url(connectors),
            )),
            common_enums::AttemptStatus::Failure => Ok(format!(
                "{}/v1/invoices/{invoice_id}/void",
                self.base_url(connectors),
            )),
            _ => Err(errors::ConnectorError::FailedToObtainIntegrationUrl.into()),
        }
    }

    fn build_request(
        &self,
        req: &recovery_router_data_types::InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::InvoiceRecordBackType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::InvoiceRecordBackType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &recovery_router_data_types::InvoiceRecordBackRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<recovery_router_data_types::InvoiceRecordBackRouterData, errors::ConnectorError>
    {
        let response = res
            .response
            .parse_struct::<stripebilling::StripebillingRecordBackResponse>(
                "StripebillingRecordBackResponse",
            )
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        recovery_router_data_types::InvoiceRecordBackRouterData::try_from(ResponseRouterData {
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

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Stripebilling {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn common_utils::crypto::VerifySignature + Send>, errors::ConnectorError>
    {
        Ok(Box::new(common_utils::crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let mut header_hashmap = get_signature_elements_from_header(request.headers)?;
        let signature = header_hashmap
            .remove("v1")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        hex::decode(signature).change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let mut header_hashmap = get_signature_elements_from_header(request.headers)?;
        let timestamp = header_hashmap
            .remove("t")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        Ok(format!(
            "{}.{}",
            String::from_utf8_lossy(&timestamp),
            String::from_utf8_lossy(request.body)
        )
        .into_bytes())
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        //  For Stripe billing, we need an additional call to fetch the required recovery data. So, instead of the Invoice ID, we send the Charge ID.
        let webhook =
            stripebilling::StripebillingWebhookBody::get_webhook_object_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(webhook.data.object.charge),
        ))
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook =
            stripebilling::StripebillingWebhookBody::get_webhook_object_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        let event = match webhook.event_type {
            stripebilling::StripebillingEventType::PaymentSucceeded => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentSuccess
            }
            stripebilling::StripebillingEventType::PaymentFailed => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentFailure
            }
            stripebilling::StripebillingEventType::InvoiceDeleted => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryInvoiceCancel
            }
        };
        Ok(event)
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook = stripebilling::StripebillingInvoiceBody::get_invoice_webhook_data_from_body(
            request.body,
        )
        .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(webhook))
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_attempt_details(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<revenue_recovery::RevenueRecoveryAttemptData, errors::ConnectorError> {
        // since stripe requires an additional call we dont need to implement this function because we get the recovery data from additional call itself
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_invoice_details(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<revenue_recovery::RevenueRecoveryInvoiceData, errors::ConnectorError> {
        let webhook = stripebilling::StripebillingInvoiceBody::get_invoice_webhook_data_from_body(
            request.body,
        )?;
        revenue_recovery::RevenueRecoveryInvoiceData::try_from(webhook)
    }
}

fn get_signature_elements_from_header(
    headers: &actix_web::http::header::HeaderMap,
) -> CustomResult<HashMap<String, Vec<u8>>, errors::ConnectorError> {
    let security_header = headers
        .get("stripe-signature")
        .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
    let security_header_str = security_header
        .to_str()
        .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
    let header_parts = security_header_str.split(',').collect::<Vec<&str>>();
    let mut header_hashmap: HashMap<String, Vec<u8>> = HashMap::with_capacity(header_parts.len());

    for header_part in header_parts {
        let (header_key, header_value) = header_part
            .split_once('=')
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        header_hashmap.insert(header_key.to_string(), header_value.bytes().collect());
    }

    Ok(header_hashmap)
}

static STRIPEBILLING_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Stripebilling",
    description: "Stripe Billing manages subscriptions, recurring payments, and invoicing. It supports trials, usage-based billing, coupons, and automated retries.",
    connector_type: enums::HyperswitchConnectorCategory::RevenueGrowthManagementPlatform,
    integration_status: enums::ConnectorIntegrationStatus::Beta,
};

static STRIPEBILLING_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] =
    [enums::EventClass::Payments];

impl ConnectorSpecifications for Stripebilling {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&STRIPEBILLING_CONNECTOR_INFO)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&STRIPEBILLING_SUPPORTED_WEBHOOK_FLOWS)
    }
}
