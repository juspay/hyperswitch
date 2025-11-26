pub mod transformers;

use std::sync::LazyLock;

use base64::engine::Engine;
use common_enums::enums;
use common_utils::{
    consts::BASE64_ENGINE,
    crypto,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, PaymentMethodToken as PMT, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        CreateConnectorCustomer,
    },
    router_request_types::{
        AccessTokenRequestData, ConnectorCustomerData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, ResponseId, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorCustomerResponseData, ConnectorInfo, PaymentMethodDetails, PaymentsResponseData,
        RefundsResponseData, SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        ConnectorCustomerRouterData, PaymentsAuthorizeRouterData, PaymentsSyncRouterData,
        RefreshTokenRouterData, RefundSyncRouterData, RefundsRouterData, TokenizationRouterData,
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
    types::{self, RefreshTokenType, Response, TokenizationType},
    webhooks,
};
use masking::{ExposeInterface, Mask, Secret};
use ring::hmac;
use transformers as dwolla;
use transformers::extract_token_from_body;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{convert_amount, get_http_header, RefundsRequestData, RouterData as RD},
};

#[derive(Clone)]
pub struct Dwolla {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Dwolla {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

impl api::ConnectorCustomer for Dwolla {}
impl api::Payment for Dwolla {}
impl api::PaymentSession for Dwolla {}
impl api::ConnectorAccessToken for Dwolla {}
impl api::MandateSetup for Dwolla {}
impl api::PaymentAuthorize for Dwolla {}
impl api::PaymentSync for Dwolla {}
impl api::PaymentCapture for Dwolla {}
impl api::PaymentVoid for Dwolla {}
impl api::Refund for Dwolla {}
impl api::RefundExecute for Dwolla {}
impl api::RefundSync for Dwolla {}
impl api::PaymentToken for Dwolla {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Dwolla
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
        let header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.common_get_content_type().to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),
                self.common_get_content_type().to_string().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", access_token.token.expose()).into_masked(),
            ),
            (
                headers::IDEMPOTENCY_KEY.to_string(),
                uuid::Uuid::new_v4().to_string().into(),
            ),
        ];
        Ok(header)
    }
}

impl ConnectorCommon for Dwolla {
    fn id(&self) -> &'static str {
        "dwolla"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/vnd.dwolla.v1.hal+json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.dwolla.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: dwolla::DwollaErrorResponse = res
            .response
            .parse_struct("DwollaErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response
                ._embedded
                .as_ref()
                .and_then(|errors_vec| errors_vec.first())
                .and_then(|details| details.errors.first())
                .and_then(|err_detail| err_detail.message.clone()),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Dwolla {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Dwolla {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Dwolla {
    fn get_url(
        &self,
        _req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/token", self.base_url(connectors)))
    }

    fn get_headers(
        &self,
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = dwolla::DwollaAuthType::try_from(&req.connector_auth_type)?;
        let mut headers = vec![(
            headers::CONTENT_TYPE.to_string(),
            "application/x-www-form-urlencoded".to_string().into(),
        )];

        let auth_str = format!(
            "{}:{}",
            auth.client_id.expose(),
            auth.client_secret.expose()
        );

        let encoded = BASE64_ENGINE.encode(auth_str);
        let auth_header_value = format!("Basic {encoded}");
        headers.push((
            headers::AUTHORIZATION.to_string(),
            auth_header_value.into_masked(),
        ));

        Ok(headers)
    }

    fn build_request(
        &self,
        req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let headers = self.get_headers(req, connectors)?;

        let connector_req = dwolla::DwollaAccessTokenRequest {
            grant_type: "client_credentials".to_string(),
        };
        let body = RequestContent::FormUrlEncoded(Box::new(connector_req));

        let req = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .headers(headers)
                .url(&RefreshTokenType::get_url(self, req, connectors)?)
                .set_body(body)
                .build(),
        );
        Ok(req)
    }

    fn handle_response(
        &self,
        data: &RefreshTokenRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefreshTokenRouterData, errors::ConnectorError> {
        let response: dwolla::DwollaAccessTokenResponse = res
            .response
            .parse_struct("Dwolla DwollaAccessTokenResponse")
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

impl ConnectorIntegration<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>
    for Dwolla
{
    fn get_url(
        &self,
        _req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/customers", self.base_url(connectors)))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_headers(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &ConnectorCustomerRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = dwolla::DwollaCustomerRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = Some(
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
        );
        Ok(request)
    }

    fn handle_response(
        &self,
        data: &ConnectorCustomerRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<ConnectorCustomerRouterData, errors::ConnectorError> {
        let headers = res
            .headers
            .as_ref()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
        let location = get_http_header("Location", headers)
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        let connector_customer_id = location
            .split('/')
            .next_back()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?
            .to_string();

        let response = serde_json::json!({"connector_customer_id": connector_customer_id.clone()});
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(RouterData {
            connector_customer: Some(connector_customer_id.clone()),
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new_with_customer_id(connector_customer_id),
            )),
            ..data.clone()
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

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Dwolla
{
    fn get_headers(
        &self,
        req: &TokenizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &TokenizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let customer_id = req.get_connector_customer_id()?;
        Ok(format!(
            "{}/customers/{}/funding-sources",
            self.base_url(connectors),
            customer_id
        ))
    }

    fn get_request_body(
        &self,
        req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = dwolla::DwollaFundingSourceRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &TokenizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&TokenizationType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(TokenizationType::get_headers(self, req, connectors)?)
                .set_body(TokenizationType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &TokenizationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<TokenizationRouterData, errors::ConnectorError> {
        let token = res
            .headers
            .as_ref()
            .and_then(|headers| get_http_header("Location", headers).ok())
            .and_then(|location| location.rsplit('/').next().map(|s| s.to_string()))
            .ok_or_else(|| report!(errors::ConnectorError::ResponseHandlingFailed))?;

        let response = serde_json::json!({ "payment_token": token });

        if let Some(builder) = event_builder {
            builder.set_response_body(&response);
        }

        router_env::logger::info!(connector_response=?response);

        Ok(RouterData {
            payment_method_token: Some(PMT::Token(token.clone().into())),
            response: Ok(PaymentsResponseData::TokenizationResponse { token }),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        if let Ok(body) = std::str::from_utf8(&res.response) {
            if res.status_code == 400 && body.contains("Duplicate") {
                let token = extract_token_from_body(&res.response);
                let metadata = Some(Secret::new(
                    serde_json::json!({ "payment_method_token": token? }),
                ));
                let response: dwolla::DwollaErrorResponse = res
                    .response
                    .parse_struct("DwollaErrorResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

                event_builder.map(|i| i.set_response_body(&response));

                return Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: response.code,
                    message: response.message,
                    reason: response
                        ._embedded
                        .as_ref()
                        .and_then(|errors_vec| errors_vec.first())
                        .and_then(|details| details.errors.first())
                        .and_then(|err_detail| err_detail.message.clone()),
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: metadata,
                });
            }
        }
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Dwolla {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Dwolla {
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
        Ok(format!("{}/transfers", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_router_data =
            dwolla::DwollaRouterData::try_from((amount, req, self.base_url(connectors)))?;
        let connector_req = dwolla::DwollaPaymentsRequest::try_from(&connector_router_data)?;
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
        let headers = res
            .headers
            .as_ref()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
        let location = get_http_header("Location", headers)
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        let payment_id = location
            .split('/')
            .next_back()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?
            .to_string();
        let response = serde_json::json!({"payment_id : ": payment_id.clone()});

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let connector_metadata = data
            .payment_method_token
            .as_ref()
            .and_then(|token| match token {
                PMT::Token(t) => Some(serde_json::json!({ "payment_token": t.clone().expose() })),
                _ => None,
            });

        Ok(RouterData {
            payment_method_token: data.payment_method_token.clone(),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(payment_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(payment_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            amount_captured: Some(data.request.amount),
            ..data.clone()
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Dwolla {
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
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/transfers/{}",
            self.base_url(connectors),
            connector_payment_id.clone()
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
        let response: dwolla::DwollaPSyncResponse = res
            .response
            .parse_struct("dwolla DwollaPSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Dwolla {
    //Not implemented for Dwolla
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Dwolla {
    //Not implemented for Dwolla
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Dwolla {
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
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/transfers", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount_in_minor_unit = MinorUnit::new(req.request.refund_amount);
        let amount = convert_amount(
            self.amount_converter,
            amount_in_minor_unit,
            req.request.currency,
        )?;
        let connector_router_data =
            dwolla::DwollaRouterData::try_from((amount, req, self.base_url(connectors)))?;
        let connector_req = dwolla::DwollaRefundsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundExecuteType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::RefundExecuteType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let headers = res
            .headers
            .as_ref()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
        let location = get_http_header("Location", headers)
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        let refund_id = location
            .split('/')
            .next_back()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?
            .to_string();

        let response = serde_json::json!({"refund_id : ": refund_id.clone()});

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(RouterData {
            response: Ok(RefundsResponseData {
                connector_refund_id: refund_id.clone(),
                refund_status: enums::RefundStatus::Pending,
            }),
            ..data.clone()
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Dwolla {
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
        let connector_refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}/transfers/{}",
            self.base_url(connectors),
            connector_refund_id.clone()
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
        let response: dwolla::DwollaRSyncResponse = res
            .response
            .parse_struct("dwolla DwollaRSyncResponse")
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
impl webhooks::IncomingWebhook for Dwolla {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let sig = request
            .headers
            .get("X-Request-Signature-SHA-256")
            .and_then(|hv| hv.to_str().ok())
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        hex::decode(sig).map_err(|_| errors::ConnectorError::WebhookSignatureNotFound.into())
    }

    async fn verify_webhook_source(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
        connector_name: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_name,
                connector_webhook_details,
            )
            .await?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;

        let secret_bytes = connector_webhook_secrets.secret.as_ref();
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret_bytes);

        let verify = hmac::verify(&key, request.body, &signature)
            .map(|_| true)
            .map_err(|_| errors::ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(verify)
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: dwolla::DwollaWebhookDetails = request
            .body
            .parse_struct("DwollaWebhookDetails")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        if let Some(correlation_id) = &details.correlation_id {
            if correlation_id.starts_with("refund_") {
                Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                    api_models::webhooks::RefundIdType::ConnectorRefundId(
                        details.resource_id.clone(),
                    ),
                ))
            } else if correlation_id.starts_with("payment_") {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        details.resource_id.clone(),
                    ),
                ))
            } else {
                Err(report!(errors::ConnectorError::WebhookReferenceIdNotFound))
            }
        } else {
            Err(report!(errors::ConnectorError::WebhookReferenceIdNotFound))
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let details: dwolla::DwollaWebhookDetails = request
            .body
            .parse_struct("DwollaWebhookDetails")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        let incoming = api_models::webhooks::IncomingWebhookEvent::try_from(details)?;
        Ok(incoming)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let details: dwolla::DwollaWebhookDetails = request
            .body
            .parse_struct("DwollaWebhookDetails")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(details))
    }
}

static DWOLLA_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::SequentialAutomatic,
    ];

    let mut dwolla_supported_payment_methods = SupportedPaymentMethods::new();

    dwolla_supported_payment_methods.add(
        enums::PaymentMethod::BankDebit,
        enums::PaymentMethodType::Ach,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    dwolla_supported_payment_methods
});

static DWOLLA_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Dwolla",
    description: "Dwolla is a multinational financial technology company offering financial services and software as a service (SaaS)",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Sandbox,
};

static DWOLLA_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 2] =
    [enums::EventClass::Payments, enums::EventClass::Refunds];

impl ConnectorSpecifications for Dwolla {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&DWOLLA_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*DWOLLA_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&DWOLLA_SUPPORTED_WEBHOOK_FLOWS)
    }

    fn should_call_connector_customer(
        &self,
        _payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> bool {
        true
    }
}
