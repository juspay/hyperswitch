pub mod transformers;

use std::sync::LazyLock;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts, errors,
    events::connector_api_logs::ConnectorEvent,
    types, webhooks,
};
use masking::{ExposeInterface, Mask, PeekInterface};
use ring::hmac;
use time::OffsetDateTime;
use transformers as fiserv;
use uuid::Uuid;

use crate::{
    constants::headers, types::ResponseRouterData, utils as connector_utils, utils::convert_amount,
};

#[derive(Clone)]
pub struct Fiserv {
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Fiserv {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
    pub fn generate_authorization_signature(
        &self,
        auth: fiserv::FiservAuthType,
        request_id: &str,
        payload: &str,
        timestamp: i128,
    ) -> CustomResult<String, errors::ConnectorError> {
        let fiserv::FiservAuthType {
            api_key,
            api_secret,
            ..
        } = auth;
        let raw_signature = format!("{}{request_id}{timestamp}{payload}", api_key.peek());

        let key = hmac::Key::new(hmac::HMAC_SHA256, api_secret.expose().as_bytes());
        let signature_value = common_utils::consts::BASE64_ENGINE
            .encode(hmac::sign(&key, raw_signature.as_bytes()).as_ref());
        Ok(signature_value)
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Fiserv
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
        let auth: fiserv::FiservAuthType =
            fiserv::FiservAuthType::try_from(&req.connector_auth_type)?;
        let mut auth_header = self.get_auth_header(&req.connector_auth_type)?;

        let fiserv_req = self.get_request_body(req, connectors)?;

        let client_request_id = Uuid::new_v4().to_string();
        let hmac = self
            .generate_authorization_signature(
                auth,
                &client_request_id,
                fiserv_req.get_inner_value().peek(),
                timestamp,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            ("Client-Request-Id".to_string(), client_request_id.into()),
            ("Auth-Token-Type".to_string(), "HMAC".to_string().into()),
            (headers::TIMESTAMP.to_string(), timestamp.to_string().into()),
            (headers::AUTHORIZATION.to_string(), hmac.into_masked()),
        ];
        headers.append(&mut auth_header);
        Ok(headers)
    }
}

impl ConnectorCommon for Fiserv {
    fn id(&self) -> &'static str {
        "fiserv"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.fiserv.base_url.as_ref()
    }
    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = fiserv::FiservAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::API_KEY.to_string(),
            auth.api_key.into_masked(),
        )])
    }
    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: fiserv::ErrorResponse = res
            .response
            .parse_struct("Fiserv ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error_details_opt = response.error.as_ref().and_then(|v| v.first());

        let (code, message, reason) = if let Some(first_error) = error_details_opt {
            let code = first_error
                .code
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string());

            let message = first_error
                .message
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string());

            let reason = first_error.additional_info.clone();

            (code, message, reason)
        } else {
            (
                consts::NO_ERROR_CODE.to_string(),
                consts::NO_ERROR_MESSAGE.to_string(),
                None,
            )
        };

        Ok(ErrorResponse {
            code,
            message,
            reason,
            status_code: res.status_code,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl api::ConnectorAccessToken for Fiserv {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Fiserv {
    // Not Implemented (R)
}

impl ConnectorValidation for Fiserv {}

impl api::Payment for Fiserv {}

impl api::PaymentToken for Fiserv {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Fiserv
{
    // Not Implemented (R)
}

impl api::MandateSetup for Fiserv {}

#[allow(dead_code)]
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Fiserv {
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Fiserv".to_string())
                .into(),
        )
    }
}

impl api::PaymentVoid for Fiserv {}

#[allow(dead_code)]
impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Fiserv {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            //The docs has this url wrong, cancels is the working endpoint
            "{}ch/payments/v1/cancels",
            connectors.fiserv.base_url
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = fiserv::FiservCancelRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsVoidType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        );

        Ok(request)
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: fiserv::FiservPaymentsResponse = res
            .response
            .parse_struct("Fiserv PaymentResponse")
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
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::PaymentSync for Fiserv {}

#[allow(dead_code)]
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Fiserv {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ch/payments/v1/transaction-inquiry",
            connectors.fiserv.base_url
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = fiserv::FiservSyncRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        );
        Ok(request)
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: fiserv::FiservSyncResponse = res
            .response
            .parse_struct("Fiserv PaymentSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let p_sync_response = response.sync_responses.first().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "P_Sync_Responses[0]",
            },
        )?;

        let (approved_amount, currency) = match &p_sync_response {
            fiserv::FiservPaymentsResponse::Charges(resp) => (
                &resp.payment_receipt.approved_amount.total,
                &resp.payment_receipt.approved_amount.currency,
            ),
            fiserv::FiservPaymentsResponse::Checkout(resp) => (
                &resp.payment_receipt.approved_amount.total,
                &resp.payment_receipt.approved_amount.currency,
            ),
        };

        let response_integrity_object = connector_utils::get_sync_integrity_object(
            self.amount_converter,
            *approved_amount,
            currency.to_string().clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed);

        new_router_data.map(|mut router_data| {
            router_data.request.integrity_object = Some(response_integrity_object);
            router_data
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::PaymentCapture for Fiserv {}
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Fiserv {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount_to_capture = convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
        let router_obj = fiserv::FiservRouterData::try_from((amount_to_capture, req))?;
        let connector_req = fiserv::FiservCaptureRequest::try_from(&router_obj)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = Some(
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
        );
        Ok(request)
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: fiserv::FiservPaymentsResponse = res
            .response
            .parse_struct("Fiserv Payment Response")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let (approved_amount, currency) = match &response {
            fiserv::FiservPaymentsResponse::Charges(resp) => (
                &resp.payment_receipt.approved_amount.total,
                &resp.payment_receipt.approved_amount.currency,
            ),
            fiserv::FiservPaymentsResponse::Checkout(resp) => (
                &resp.payment_receipt.approved_amount.total,
                &resp.payment_receipt.approved_amount.currency,
            ),
        };

        let response_integrity_object = connector_utils::get_capture_integrity_object(
            self.amount_converter,
            Some(*approved_amount),
            currency.to_string().clone(),
        )?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed);

        new_router_data.map(|mut router_data| {
            router_data.request.integrity_object = Some(response_integrity_object);
            router_data
        })
    }

    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ch/payments/v1/charges",
            connectors.fiserv.base_url
        ))
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::PaymentSession for Fiserv {}

#[allow(dead_code)]
impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Fiserv {}

impl api::PaymentAuthorize for Fiserv {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Fiserv {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let url = match &req.request.payment_method_data {
            PaymentMethodData::Wallet(WalletData::PaypalRedirect(_)) => {
                format!("{}ch/checkouts/v1/orders", connectors.fiserv.base_url)
            }
            _ => format!("{}ch/payments/v1/charges", connectors.fiserv.base_url),
        };

        Ok(url)
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let router_obj = fiserv::FiservRouterData::try_from((amount, req))?;
        let connector_req = fiserv::FiservPaymentsRequest::try_from(&router_obj)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = Some(
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
        );

        Ok(request)
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: fiserv::FiservPaymentsResponse = res
            .response
            .parse_struct("Fiserv PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let (approved_amount, currency) = match &response {
            fiserv::FiservPaymentsResponse::Charges(resp) => (
                &resp.payment_receipt.approved_amount.total,
                &resp.payment_receipt.approved_amount.currency,
            ),
            fiserv::FiservPaymentsResponse::Checkout(resp) => (
                &resp.payment_receipt.approved_amount.total,
                &resp.payment_receipt.approved_amount.currency,
            ),
        };

        let response_integrity_object = connector_utils::get_authorise_integrity_object(
            self.amount_converter,
            *approved_amount,
            currency.to_string().clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed);

        new_router_data.map(|mut router_data| {
            router_data.request.integrity_object = Some(response_integrity_object);
            router_data
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::Refund for Fiserv {}
impl api::RefundExecute for Fiserv {}
impl api::RefundSync for Fiserv {}

#[allow(dead_code)]
impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Fiserv {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        "application/json"
    }
    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ch/payments/v1/refunds",
            connectors.fiserv.base_url
        ))
    }
    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let router_obj = fiserv::FiservRouterData::try_from((refund_amount, req))?;
        let connector_req = fiserv::FiservRefundRequest::try_from(&router_obj)?;
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
        res: types::Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        router_env::logger::debug!(target: "router::connector::fiserv", response=?res);
        let response: fiserv::RefundResponse =
            res.response
                .parse_struct("fiserv RefundResponse")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let response_integrity_object = connector_utils::get_refund_integrity_object(
            self.amount_converter,
            response.payment_receipt.approved_amount.total,
            response
                .payment_receipt
                .approved_amount
                .currency
                .to_string()
                .clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed);

        new_router_data.map(|mut router_data| {
            router_data.request.integrity_object = Some(response_integrity_object);
            router_data
        })
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[allow(dead_code)]
impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Fiserv {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        _req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ch/payments/v1/transaction-inquiry",
            connectors.fiserv.base_url
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = fiserv::FiservSyncRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .set_body(types::RefundSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        );
        Ok(request)
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        router_env::logger::debug!(target: "router::connector::fiserv", response=?res);

        let response: fiserv::FiservSyncResponse = res
            .response
            .parse_struct("Fiserv Refund Response")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let r_sync_response = response.sync_responses.first().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "R_Sync_Responses[0]",
            },
        )?;

        let (approved_amount, currency) = match &r_sync_response {
            fiserv::FiservPaymentsResponse::Charges(resp) => (
                &resp.payment_receipt.approved_amount.total,
                &resp.payment_receipt.approved_amount.currency,
            ),
            fiserv::FiservPaymentsResponse::Checkout(resp) => (
                &resp.payment_receipt.approved_amount.total,
                &resp.payment_receipt.approved_amount.currency,
            ),
        };

        let response_integrity_object = connector_utils::get_refund_integrity_object(
            self.amount_converter,
            *approved_amount,
            currency.to_string().clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed);

        new_router_data.map(|mut router_data| {
            router_data.request.integrity_object = Some(response_integrity_object);
            router_data
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Fiserv {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

static FISERV_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::SequentialAutomatic,
        enums::CaptureMethod::Manual,
    ];

    let supported_card_network = vec![
        common_enums::CardNetwork::Visa,
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::AmericanExpress,
        common_enums::CardNetwork::JCB,
        common_enums::CardNetwork::Discover,
        common_enums::CardNetwork::UnionPay,
        common_enums::CardNetwork::Interac,
    ];

    let mut fiserv_supported_payment_methods = SupportedPaymentMethods::new();

    fiserv_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::NotSupported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    }
                }),
            ),
        },
    );

    fiserv_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Debit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::NotSupported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    }
                }),
            ),
        },
    );

    fiserv_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::GooglePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    fiserv_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::Paypal,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    fiserv_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::ApplePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    fiserv_supported_payment_methods
});

static FISERV_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Fiserv",
    description:
        "Fiserv is a global fintech and payments company with solutions for banking, global commerce, merchant acquiring, billing and payments, and point-of-sale ",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Sandbox,
};

static FISERV_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Fiserv {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&FISERV_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*FISERV_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&FISERV_SUPPORTED_WEBHOOK_FLOWS)
    }
}
