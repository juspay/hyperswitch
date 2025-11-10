mod requests;
mod responses;
pub mod transformers;

use common_enums::enums;
use common_utils::{
    crypto,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt, ValueExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, CreateConnectorCustomer, PSync, PaymentMethodToken, Session,
            SetupMandate, Void,
        },
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, ConnectorCustomerData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        ConnectorCustomerRouterData, PaymentsAuthorizeRouterData, PaymentsCancelRouterData,
        PaymentsCaptureRouterData, PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, RefreshTokenType, Response},
    webhooks,
};
use lazy_static::lazy_static;
use masking::{ExposeInterface, Mask, PeekInterface};
use requests::{
    FacilitapayAuthRequest, FacilitapayCustomerRequest, FacilitapayPaymentsRequest,
    FacilitapayRouterData,
};
use responses::{
    FacilitapayAuthResponse, FacilitapayCustomerResponse, FacilitapayPaymentsResponse,
    FacilitapayRefundResponse, FacilitapayWebhookEventType,
};
use transformers::parse_facilitapay_error_response;

use crate::{
    connectors::facilitapay::responses::FacilitapayVoidResponse,
    constants::headers,
    types::{RefreshTokenRouterData, ResponseRouterData},
    utils::{self, RefundsRequestData},
};

#[derive(Clone)]
pub struct Facilitapay {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Facilitapay {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

impl api::ConnectorCustomer for Facilitapay {}
impl api::Payment for Facilitapay {}
impl api::PaymentSession for Facilitapay {}
impl api::ConnectorAccessToken for Facilitapay {}
impl api::MandateSetup for Facilitapay {}
impl api::PaymentAuthorize for Facilitapay {}
impl api::PaymentSync for Facilitapay {}
impl api::PaymentCapture for Facilitapay {}
impl api::PaymentVoid for Facilitapay {}
impl api::Refund for Facilitapay {}
impl api::RefundExecute for Facilitapay {}
impl api::RefundSync for Facilitapay {}
impl api::PaymentToken for Facilitapay {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Facilitapay
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Facilitapay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let access_token = req
            .access_token
            .clone()
            .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;

        let headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", access_token.token.clone().peek()).into_masked(),
            ),
        ];

        Ok(headers)
    }
}

impl ConnectorCommon for Facilitapay {
    fn id(&self) -> &'static str {
        "facilitapay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.facilitapay.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        parse_facilitapay_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>
    for Facilitapay
{
    fn get_headers(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/subject/people", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &ConnectorCustomerRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = FacilitapayCustomerRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
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
    ) -> CustomResult<ConnectorCustomerRouterData, errors::ConnectorError>
    where
        ConnectorCustomerRouterData: Clone,
    {
        let response: FacilitapayCustomerResponse = res
            .response
            .parse_struct("FacilitapayCustomerResponse")
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

impl ConnectorValidation for Facilitapay {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Facilitapay {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Facilitapay {
    fn get_headers(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/sign_in", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = FacilitapayAuthRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .attach_default_headers()
                .headers(RefreshTokenType::get_headers(self, req, connectors)?)
                .url(&RefreshTokenType::get_url(self, req, connectors)?)
                .set_body(RefreshTokenType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefreshTokenRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefreshTokenRouterData, errors::ConnectorError> {
        let response: FacilitapayAuthResponse = res
            .response
            .parse_struct("FacilitapayAuthResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Facilitapay
{
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented(
            "Setup Mandate flow for Facilitapay".to_string(),
        )
        .into())
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Facilitapay {
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
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/transactions", self.base_url(connectors)))
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

        let connector_router_data = FacilitapayRouterData::from((amount, req));
        let connector_req = FacilitapayPaymentsRequest::try_from(&connector_router_data)?;

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
        let response: FacilitapayPaymentsResponse = res
            .response
            .parse_struct("FacilitapayPaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Facilitapay {
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
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_transaction_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/transactions/{}",
            self.base_url(connectors),
            connector_transaction_id
        ))
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
        let response: FacilitapayPaymentsResponse = res
            .response
            .parse_struct("FacilitapayPaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Facilitapay {
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
        let response: FacilitapayPaymentsResponse = res
            .response
            .parse_struct("FacilitapayPaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Facilitapay {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/transactions/{}/refund",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: FacilitapayVoidResponse = res
            .response
            .parse_struct("FacilitapayCancelResponse")
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Facilitapay {
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
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/transactions/{}/refund_received_transaction",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        // Validate that this is a full refund
        if req.request.payment_amount != req.request.refund_amount {
            return Err(errors::ConnectorError::NotSupported {
                message: "Partial refund not supported by Facilitapay".to_string(),
                connector: "Facilitapay",
            }
            .into());
        }

        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(
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
        let response: FacilitapayRefundResponse = res
            .response
            .parse_struct("FacilitapayRefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Facilitapay {
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
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}/transactions/{}/refund_received_transaction/{}",
            self.base_url(connectors),
            req.request.connector_transaction_id,
            refund_id
        ))
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
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: FacilitapayRefundResponse = res
            .response
            .parse_struct("FacilitapayRefundSyncResponse")
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
impl webhooks::IncomingWebhook for Facilitapay {
    async fn verify_webhook_source(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<masking::Secret<serde_json::Value>>,
        _connector_name: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let webhook_body: responses::FacilitapayWebhookNotification = request
            .body
            .parse_struct("FacilitapayWebhookNotification")
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let connector_webhook_secrets = match connector_webhook_details {
            Some(secret_value) => {
                let secret = secret_value
                    .parse_value::<api_models::admin::MerchantConnectorWebhookDetails>(
                        "MerchantConnectorWebhookDetails",
                    )
                    .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
                secret.merchant_secret.expose()
            }
            None => "default_secret".to_string(),
        };

        // FacilitaPay uses a simple 4-digit secret for verification
        Ok(webhook_body.notification.secret.peek() == &connector_webhook_secrets)
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook_body: responses::FacilitapayWebhookNotification = request
            .body
            .parse_struct("FacilitapayWebhookNotification")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        // Extract transaction ID from the webhook data
        let transaction_id = match &webhook_body.notification.data {
            responses::FacilitapayWebhookData::Transaction { transaction_id }
            | responses::FacilitapayWebhookData::CardPayment { transaction_id, .. } => {
                transaction_id.clone()
            }
            responses::FacilitapayWebhookData::Exchange {
                transaction_ids, ..
            }
            | responses::FacilitapayWebhookData::Wire {
                transaction_ids, ..
            }
            | responses::FacilitapayWebhookData::WireError {
                transaction_ids, ..
            } => transaction_ids
                .first()
                .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?
                .clone(),
        };

        // For refund webhooks, Facilitapay sends the original payment transaction ID
        // not the refund transaction ID
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(transaction_id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook_body: responses::FacilitapayWebhookNotification = request
            .body
            .parse_struct("FacilitapayWebhookNotification")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        // Note: For "identified" events, we need additional logic to determine if it's cross-currency
        // Since we don't have access to the payment data here, we'll default to Success for now
        // The actual status determination happens in the webhook processing flow
        let event = match webhook_body.notification.event_type {
            FacilitapayWebhookEventType::ExchangeCreated => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing
            }
            FacilitapayWebhookEventType::Identified
            | FacilitapayWebhookEventType::PaymentApproved
            | FacilitapayWebhookEventType::WireCreated => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
            }
            FacilitapayWebhookEventType::PaymentExpired
            | FacilitapayWebhookEventType::PaymentFailed => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure
            }
            FacilitapayWebhookEventType::PaymentRefunded => {
                api_models::webhooks::IncomingWebhookEvent::RefundSuccess
            }
            FacilitapayWebhookEventType::WireWaitingCorrection => {
                api_models::webhooks::IncomingWebhookEvent::PaymentActionRequired
            }
        };

        Ok(event)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook_body: responses::FacilitapayWebhookNotification = request
            .body
            .parse_struct("FacilitapayWebhookNotification")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(Box::new(webhook_body))
    }
}

lazy_static! {
    static ref FACILITAPAY_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        display_name:
            "Facilitapay",
        description:
            "FacilitaPay, the best and most reliable payment provider for your international business.",
        connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
        integration_status: enums::ConnectorIntegrationStatus::Sandbox,
    };
    static ref FACILITAPAY_SUPPORTED_PAYMENT_METHODS: SupportedPaymentMethods = {
        let facilitapay_supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let mut facilitapay_supported_payment_methods = SupportedPaymentMethods::new();

        facilitapay_supported_payment_methods.add(
            enums::PaymentMethod::BankTransfer,
            enums::PaymentMethodType::Pix,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: facilitapay_supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        facilitapay_supported_payment_methods
    };
    static ref FACILITAPAY_SUPPORTED_WEBHOOK_FLOWS: Vec<enums::EventClass> = vec![
        enums::EventClass::Payments,
    ];
}

impl ConnectorSpecifications for Facilitapay {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&*FACILITAPAY_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*FACILITAPAY_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&*FACILITAPAY_SUPPORTED_WEBHOOK_FLOWS)
    }

    fn should_call_connector_customer(
        &self,
        _payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> bool {
        true
    }
}
