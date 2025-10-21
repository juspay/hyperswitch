pub mod transformers;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts::BASE64_ENGINE,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::ResultExt;
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::{revenue_recovery, router_data_v2::RouterDataV2};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_data_v2::flow_common_types::{SubscriptionCreateData, SubscriptionCustomerData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        revenue_recovery::InvoiceRecordBack,
        subscriptions::{
            GetSubscriptionEstimate, GetSubscriptionPlanPrices, GetSubscriptionPlans,
            SubscriptionCreate,
        },
        CreateConnectorCustomer,
    },
    router_request_types::{
        revenue_recovery::InvoiceRecordBackRequest,
        subscriptions::{
            GetSubscriptionEstimateRequest, GetSubscriptionPlanPricesRequest,
            GetSubscriptionPlansRequest, SubscriptionCreateRequest,
        },
        AccessTokenRequestData, ConnectorCustomerData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            GetSubscriptionEstimateResponse, GetSubscriptionPlanPricesResponse,
            GetSubscriptionPlansResponse, SubscriptionCreateResponse,
        },
        ConnectorInfo, PaymentsResponseData, RefundsResponseData,
    },
    types::{
        ConnectorCustomerRouterData, GetSubscriptionEstimateRouterData,
        GetSubscriptionPlanPricesRouterData, GetSubscriptionPlansRouterData,
        InvoiceRecordBackRouterData, PaymentsAuthorizeRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
        SubscriptionCreateRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self,
        payments::ConnectorCustomer,
        subscriptions_v2::{GetSubscriptionPlanPricesV2, GetSubscriptionPlansV2},
        ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    connector_integration_v2::ConnectorIntegrationV2,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{Mask, PeekInterface, Secret};
use transformers as chargebee;

use crate::{
    connectors::chargebee::transformers::{
        ChargebeeGetPlanPricesResponse, ChargebeeListPlansResponse,
    },
    constants::{self, headers},
    types::ResponseRouterData,
    utils,
};

#[derive(Clone)]
pub struct Chargebee {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Chargebee {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}
impl ConnectorCustomer for Chargebee {}
impl api::Payment for Chargebee {}
impl api::PaymentSession for Chargebee {}
impl api::ConnectorAccessToken for Chargebee {}
impl api::MandateSetup for Chargebee {}
impl api::PaymentAuthorize for Chargebee {}
impl api::PaymentSync for Chargebee {}
impl api::PaymentCapture for Chargebee {}
impl api::PaymentVoid for Chargebee {}
impl api::Refund for Chargebee {}
impl api::RefundExecute for Chargebee {}
impl api::RefundSync for Chargebee {}
impl api::PaymentToken for Chargebee {}
impl api::subscriptions::Subscriptions for Chargebee {}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl api::revenue_recovery::RevenueRecoveryRecordBack for Chargebee {}

impl api::subscriptions::SubscriptionCreate for Chargebee {}

impl ConnectorIntegration<SubscriptionCreate, SubscriptionCreateRequest, SubscriptionCreateResponse>
    for Chargebee
{
    fn get_headers(
        &self,
        req: &SubscriptionCreateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &SubscriptionCreateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let metadata: chargebee::ChargebeeMetadata =
            utils::to_connector_meta_from_secret(req.connector_meta_data.clone())?;

        let site = metadata.site.peek();

        let mut base = self.base_url(connectors).to_string();

        base = base.replace("{{merchant_endpoint_prefix}}", site);
        base = base.replace("$", site);

        if base.contains("{{merchant_endpoint_prefix}}") || base.contains('$') {
            return Err(errors::ConnectorError::InvalidConnectorConfig {
                config: "Chargebee base_url has an unresolved placeholder (expected `$` or `{{merchant_endpoint_prefix}}`).",
            }
            .into());
        }

        if !base.ends_with('/') {
            base.push('/');
        }

        let customer_id = &req.request.customer_id.get_string_repr().to_string();
        Ok(format!(
            "{base}v2/customers/{customer_id}/subscription_for_items"
        ))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &SubscriptionCreateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = chargebee::ChargebeeRouterData::from((MinorUnit::new(0), req));
        let connector_req =
            chargebee::ChargebeeSubscriptionCreateRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &SubscriptionCreateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::SubscriptionCreateType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::SubscriptionCreateType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::SubscriptionCreateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &SubscriptionCreateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SubscriptionCreateRouterData, errors::ConnectorError> {
        let response: chargebee::ChargebeeSubscriptionCreateResponse = res
            .response
            .parse_struct("chargebee ChargebeeSubscriptionCreateResponse")
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

impl
    ConnectorIntegrationV2<
        SubscriptionCreate,
        SubscriptionCreateData,
        SubscriptionCreateRequest,
        SubscriptionCreateResponse,
    > for Chargebee
{
    // Not Implemented (R)
}

impl
    ConnectorIntegrationV2<
        CreateConnectorCustomer,
        SubscriptionCustomerData,
        ConnectorCustomerData,
        PaymentsResponseData,
    > for Chargebee
{
    // Not Implemented (R)
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Chargebee
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Chargebee
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
            self.common_get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Chargebee {
    fn id(&self) -> &'static str {
        "chargebee"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.chargebee.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = chargebee::ChargebeeAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let encoded_api_key = BASE64_ENGINE.encode(auth.full_access_key_v1.peek());
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Basic {encoded_api_key}").into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: chargebee::ChargebeeErrorResponse = res
            .response
            .parse_struct("ChargebeeErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.api_error_code.clone(),
            message: response.api_error_code.clone(),
            reason: Some(response.message),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Chargebee {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Chargebee {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Chargebee {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Chargebee
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Chargebee {
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

        let connector_router_data = chargebee::ChargebeeRouterData::from((amount, req));
        let connector_req = chargebee::ChargebeePaymentsRequest::try_from(&connector_router_data)?;
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
        let response: chargebee::ChargebeePaymentsResponse = res
            .response
            .parse_struct("Chargebee PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Chargebee {
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
        let response: chargebee::ChargebeePaymentsResponse = res
            .response
            .parse_struct("chargebee PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Chargebee {
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
        let response: chargebee::ChargebeePaymentsResponse = res
            .response
            .parse_struct("Chargebee PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Chargebee {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Chargebee {
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

        let connector_router_data = chargebee::ChargebeeRouterData::from((refund_amount, req));
        let connector_req = chargebee::ChargebeeRefundRequest::try_from(&connector_router_data)?;
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
        let response: chargebee::RefundResponse = res
            .response
            .parse_struct("chargebee RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Chargebee {
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
        let response: chargebee::RefundResponse = res
            .response
            .parse_struct("chargebee RefundSyncResponse")
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

impl ConnectorIntegration<InvoiceRecordBack, InvoiceRecordBackRequest, InvoiceRecordBackResponse>
    for Chargebee
{
    fn get_headers(
        &self,
        req: &InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_url(
        &self,
        req: &InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let metadata: chargebee::ChargebeeMetadata =
            utils::to_connector_meta_from_secret(req.connector_meta_data.clone())?;

        let site = metadata.site.peek();

        let mut base = self.base_url(connectors).to_string();

        base = base.replace("{{merchant_endpoint_prefix}}", site);
        base = base.replace("$", site);

        if base.contains("{{merchant_endpoint_prefix}}") || base.contains('$') {
            return Err(errors::ConnectorError::InvalidConnectorConfig {
                config: "Chargebee base_url has an unresolved placeholder (expected `$` or `{{merchant_endpoint_prefix}}`).",
            }
            .into());
        }

        if !base.ends_with('/') {
            base.push('/');
        }

        let invoice_id = req
            .request
            .merchant_reference_id
            .get_string_repr()
            .to_string();
        Ok(format!("{base}v2/invoices/{invoice_id}/record_payment"))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &InvoiceRecordBackRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.amount,
            req.request.currency,
        )?;
        let connector_router_data = chargebee::ChargebeeRouterData::from((amount, req));
        let connector_req =
            chargebee::ChargebeeRecordPaymentRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &InvoiceRecordBackRouterData,
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
                .set_body(types::InvoiceRecordBackType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &InvoiceRecordBackRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<InvoiceRecordBackRouterData, errors::ConnectorError> {
        let response: chargebee::ChargebeeRecordbackResponse = res
            .response
            .parse_struct("chargebee ChargebeeRecordbackResponse")
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

fn get_chargebee_plans_query_params(
    _req: &RouterData<
        GetSubscriptionPlans,
        GetSubscriptionPlansRequest,
        GetSubscriptionPlansResponse,
    >,
) -> CustomResult<String, errors::ConnectorError> {
    // Try to get limit from request, else default to 10
    let limit = _req.request.limit.unwrap_or(10);
    let offset = _req.request.offset.unwrap_or(0);
    let param = format!(
        "?limit={}&offset={}&type[is]={}",
        limit,
        offset,
        constants::PLAN_ITEM_TYPE
    );
    Ok(param)
}

impl api::subscriptions::GetSubscriptionPlansFlow for Chargebee {}
impl api::subscriptions::SubscriptionRecordBackFlow for Chargebee {}

impl
    ConnectorIntegration<
        GetSubscriptionPlans,
        GetSubscriptionPlansRequest,
        GetSubscriptionPlansResponse,
    > for Chargebee
{
    fn get_headers(
        &self,
        req: &GetSubscriptionPlansRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &GetSubscriptionPlansRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let query_params = get_chargebee_plans_query_params(req)?;
        let metadata: chargebee::ChargebeeMetadata =
            utils::to_connector_meta_from_secret(req.connector_meta_data.clone())?;

        let site = metadata.site.peek();

        let mut base = self.base_url(connectors).to_string();

        base = base.replace("{{merchant_endpoint_prefix}}", site);
        base = base.replace("$", site);

        if base.contains("{{merchant_endpoint_prefix}}") || base.contains('$') {
            return Err(errors::ConnectorError::InvalidConnectorConfig {
                config: "Chargebee base_url has an unresolved placeholder (expected `$` or `{{merchant_endpoint_prefix}}`).",
            }
            .into());
        }

        if !base.ends_with('/') {
            base.push('/');
        }

        let url = format!("{base}v2/items{query_params}");
        Ok(url)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn build_request(
        &self,
        req: &GetSubscriptionPlansRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::GetSubscriptionPlansType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::GetSubscriptionPlansType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &GetSubscriptionPlansRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<GetSubscriptionPlansRouterData, errors::ConnectorError> {
        let response: ChargebeeListPlansResponse = res
            .response
            .parse_struct("ChargebeeListPlansResponse")
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

impl GetSubscriptionPlansV2 for Chargebee {}

impl
    ConnectorIntegrationV2<
        GetSubscriptionPlans,
        hyperswitch_domain_models::router_data_v2::flow_common_types::GetSubscriptionPlansData,
        GetSubscriptionPlansRequest,
        GetSubscriptionPlansResponse,
    > for Chargebee
{
    // Not implemented (R)
}

impl ConnectorIntegration<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>
    for Chargebee
{
    fn get_headers(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let metadata: chargebee::ChargebeeMetadata =
            utils::to_connector_meta_from_secret(req.connector_meta_data.clone())?;

        let site = metadata.site.peek();

        let mut base = self.base_url(connectors).to_string();

        base = base.replace("{{merchant_endpoint_prefix}}", site);
        base = base.replace("$", site);

        if base.contains("{{merchant_endpoint_prefix}}") || base.contains('$') {
            return Err(errors::ConnectorError::InvalidConnectorConfig {
                config: "Chargebee base_url has an unresolved placeholder (expected `$` or `{{merchant_endpoint_prefix}}`).",
            }
            .into());
        }

        if !base.ends_with('/') {
            base.push('/');
        }

        let url = format!("{base}v2/customers");
        Ok(url)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &ConnectorCustomerRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = chargebee::ChargebeeRouterData::from((MinorUnit::new(0), req));
        let connector_req =
            chargebee::ChargebeeCustomerCreateRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::ConnectorCustomerType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ConnectorCustomerType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::ConnectorCustomerType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &ConnectorCustomerRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<ConnectorCustomerRouterData, errors::ConnectorError> {
        let response: chargebee::ChargebeeCustomerCreateResponse = res
            .response
            .parse_struct("ChargebeeCustomerCreateResponse")
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

impl api::subscriptions::GetSubscriptionPlanPricesFlow for Chargebee {}

fn get_chargebee_plan_prices_query_params(
    req: &GetSubscriptionPlanPricesRouterData,
) -> CustomResult<String, errors::ConnectorError> {
    let item_id = req.request.plan_price_id.to_string();
    let params = format!("?item_id[is]={item_id}");
    Ok(params)
}

impl
    ConnectorIntegration<
        GetSubscriptionPlanPrices,
        GetSubscriptionPlanPricesRequest,
        GetSubscriptionPlanPricesResponse,
    > for Chargebee
{
    fn get_headers(
        &self,
        req: &GetSubscriptionPlanPricesRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &GetSubscriptionPlanPricesRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let query_params = get_chargebee_plan_prices_query_params(req)?;

        let metadata: chargebee::ChargebeeMetadata =
            utils::to_connector_meta_from_secret(req.connector_meta_data.clone())?;

        let site = metadata.site.peek();

        let mut base = self.base_url(connectors).to_string();

        base = base.replace("{{merchant_endpoint_prefix}}", site);
        base = base.replace("$", site);

        if base.contains("{{merchant_endpoint_prefix}}") || base.contains('$') {
            return Err(errors::ConnectorError::InvalidConnectorConfig {
                config: "Chargebee base_url has an unresolved placeholder (expected `$` or `{{merchant_endpoint_prefix}}`).",
            }
            .into());
        }

        if !base.ends_with('/') {
            base.push('/');
        }

        let url = format!("{base}v2/item_prices{query_params}");
        Ok(url)
    }
    // check if get_content_type is required
    fn build_request(
        &self,
        req: &GetSubscriptionPlanPricesRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::GetSubscriptionPlanPricesType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::GetSubscriptionPlanPricesType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &GetSubscriptionPlanPricesRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<GetSubscriptionPlanPricesRouterData, errors::ConnectorError> {
        let response: ChargebeeGetPlanPricesResponse = res
            .response
            .parse_struct("chargebee ChargebeeGetPlanPricesResponse")
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

impl GetSubscriptionPlanPricesV2 for Chargebee {}

impl
    ConnectorIntegrationV2<
        GetSubscriptionPlanPrices,
        hyperswitch_domain_models::router_data_v2::flow_common_types::GetSubscriptionPlanPricesData,
        GetSubscriptionPlanPricesRequest,
        GetSubscriptionPlanPricesResponse,
    > for Chargebee
{
    // TODO: implement functions when support enabled
}

impl api::subscriptions::GetSubscriptionEstimateFlow for Chargebee {}

impl
    ConnectorIntegration<
        GetSubscriptionEstimate,
        GetSubscriptionEstimateRequest,
        GetSubscriptionEstimateResponse,
    > for Chargebee
{
    fn get_headers(
        &self,
        req: &GetSubscriptionEstimateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_url(
        &self,
        req: &GetSubscriptionEstimateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let metadata: chargebee::ChargebeeMetadata =
            utils::to_connector_meta_from_secret(req.connector_meta_data.clone())?;

        let site = metadata.site.peek();

        let mut base = self.base_url(connectors).to_string();

        base = base.replace("{{merchant_endpoint_prefix}}", site);
        base = base.replace("$", site);

        if base.contains("{{merchant_endpoint_prefix}}") || base.contains('$') {
            return Err(errors::ConnectorError::InvalidConnectorConfig {
                config: "Chargebee base_url has an unresolved placeholder (expected `$` or `{{merchant_endpoint_prefix}}`).",
            }
            .into());
        }

        if !base.ends_with('/') {
            base.push('/');
        }

        Ok(format!("{base}v2/estimates/create_subscription_for_items"))
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_request_body(
        &self,
        req: &GetSubscriptionEstimateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = chargebee::ChargebeeSubscriptionEstimateRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &GetSubscriptionEstimateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::GetSubscriptionEstimateType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::GetSubscriptionEstimateType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::GetSubscriptionEstimateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &GetSubscriptionEstimateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<GetSubscriptionEstimateRouterData, errors::ConnectorError> {
        let response: chargebee::SubscriptionEstimateResponse = res
            .response
            .parse_struct("chargebee SubscriptionEstimateResponse")
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

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Chargebee {
    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let base64_signature = utils::get_header_key_value("authorization", request.headers)?;
        let signature = base64_signature.as_bytes().to_owned();
        Ok(signature)
    }
    async fn verify_webhook_source(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: common_utils::crypto::Encryptable<Secret<serde_json::Value>>,
        connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_label,
                connector_webhook_details,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let password = connector_webhook_secrets
            .additional_secret
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Failed to get additional secrets")?;
        let username = String::from_utf8(connector_webhook_secrets.secret.to_vec())
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        let secret_auth = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!(
                "{}:{}",
                username,
                password.peek()
            ))
        );
        let signature_auth = String::from_utf8(signature.to_vec())
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        Ok(signature_auth == secret_auth)
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook =
            chargebee::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::InvoiceId(
            api_models::webhooks::InvoiceIdType::ConnectorInvoiceId(
                webhook.content.invoice.id.get_string_repr().to_string(),
            ),
        ))
    }
    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook =
            chargebee::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        let subscription_id = webhook.content.invoice.subscription_id;
        Ok(api_models::webhooks::ObjectReferenceId::SubscriptionId(
            subscription_id,
        ))
    }
    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook =
            chargebee::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let event = api_models::webhooks::IncomingWebhookEvent::from(webhook.event_type);
        Ok(event)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook =
            chargebee::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(webhook))
    }
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_attempt_details(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<revenue_recovery::RevenueRecoveryAttemptData, errors::ConnectorError> {
        let webhook =
            transformers::ChargebeeWebhookBody::get_webhook_object_from_body(request.body)?;
        revenue_recovery::RevenueRecoveryAttemptData::try_from(webhook)
    }
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_invoice_details(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<revenue_recovery::RevenueRecoveryInvoiceData, errors::ConnectorError> {
        let webhook =
            transformers::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)?;
        revenue_recovery::RevenueRecoveryInvoiceData::try_from(webhook)
    }

    fn get_subscription_mit_payment_data(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        hyperswitch_domain_models::router_flow_types::SubscriptionMitPaymentData,
        errors::ConnectorError,
    > {
        let webhook_body =
            transformers::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
                .attach_printable("Failed to parse Chargebee invoice webhook body")?;

        let chargebee_mit_data = transformers::ChargebeeMitPaymentData::try_from(webhook_body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
            .attach_printable("Failed to extract MIT payment data from Chargebee webhook")?;

        // Convert Chargebee-specific data to generic domain model
        Ok(
            hyperswitch_domain_models::router_flow_types::SubscriptionMitPaymentData {
                invoice_id: chargebee_mit_data.invoice_id,
                amount_due: chargebee_mit_data.amount_due,
                currency_code: chargebee_mit_data.currency_code,
                status: chargebee_mit_data.status.map(|s| s.into()),
                customer_id: chargebee_mit_data.customer_id,
                subscription_id: chargebee_mit_data.subscription_id,
                first_invoice: chargebee_mit_data.first_invoice,
            },
        )
    }
}

static CHARGEBEE_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Chargebee",
    description: "Chargebee is a Revenue Growth Management (RGM) platform that helps subscription businesses manage subscriptions, billing, revenue recognition, collections, and customer retention, essentially streamlining the entire subscription lifecycle.",
    connector_type: enums::HyperswitchConnectorCategory::RevenueGrowthManagementPlatform,
    integration_status: enums::ConnectorIntegrationStatus::Alpha,
};

static CHARGEBEE_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] = [enums::EventClass::Payments];

impl ConnectorSpecifications for Chargebee {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&CHARGEBEE_CONNECTOR_INFO)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&CHARGEBEE_SUPPORTED_WEBHOOK_FLOWS)
    }
}
