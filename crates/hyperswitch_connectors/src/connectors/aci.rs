mod aci_result_codes;
pub mod transformers;

use std::sync::LazyLock;

use api_models::webhooks::IncomingWebhookEvent;
use common_enums::enums;
use common_utils::{
    crypto,
    errors::{CryptoError, CustomResult},
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
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
        PaymentsSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorIntegration, ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        PaymentsAuthorizeType, PaymentsCaptureType, PaymentsSyncType, PaymentsVoidType,
        RefundExecuteType, Response,
    },
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails, WebhookContext},
};
use hyperswitch_masking::{ExposeInterface, Mask, PeekInterface};
use ring::aead::{self, UnboundKey};
use transformers as aci;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{convert_amount, PaymentsAuthorizeRequestData},
};

#[derive(Clone)]
pub struct Aci {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Aci {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

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

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.aci.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = aci::AciAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.api_key.peek()).into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: aci::AciErrorResponse = res
            .response
            .parse_struct("AciErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        Ok(ErrorResponse {
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
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
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

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Aci
{
    fn build_request(
        &self,
        _req: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Payment method tokenization not supported".to_string(),
            connector: "ACI",
        }
        .into())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Aci {
    fn build_request(
        &self,
        _req: &RouterData<Session, PaymentsSessionData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Payment sessions not supported".to_string(),
            connector: "ACI",
        }
        .into())
    }
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Aci {
    fn build_request(
        &self,
        _req: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Access token authentication not supported".to_string(),
            connector: "ACI",
        }
        .into())
    }
}

impl api::MandateSetup for Aci {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Aci {
    fn get_headers(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
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
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v1/registrations", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = aci::AciMandateRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&self.get_url(req, connectors)?)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        errors::ConnectorError,
    > {
        let response: aci::AciMandateResponse = res
            .response
            .parse_struct("AciMandateResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Aci {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PaymentsCaptureType::get_content_type(self)
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
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}{}",
            self.base_url(connectors),
            "v1/payments/",
            req.request.connector_transaction_id,
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
        let connector_router_data = aci::AciRouterData::from((amount, req));
        let connector_req = aci::AciCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsCaptureType::get_headers(self, req, connectors)?)
                .set_body(PaymentsCaptureType::get_request_body(
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
        let response: aci::AciCaptureResponse = res
            .response
            .parse_struct("AciCaptureResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Aci {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PaymentsSyncType::get_content_type(self).to_string().into(),
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
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
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
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError>
    where
        PaymentsSyncData: Clone,
        PaymentsResponseData: Clone,
    {
        let response: aci::AciPaymentsResponse =
            res.response
                .parse_struct("AciPaymentsResponse")
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

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Aci {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PaymentsAuthorizeType::get_content_type(self)
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
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
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
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = aci::AciRouterData::from((amount, req));
        let connector_req = aci::AciPaymentsRequest::try_from(&connector_router_data)?;

        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
                .set_body(PaymentsAuthorizeType::get_request_body(
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
        let response: aci::AciPaymentsResponse =
            res.response
                .parse_struct("AciPaymentsResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Aci {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            PaymentsAuthorizeType::get_content_type(self)
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
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = &req.request.connector_transaction_id;
        Ok(format!("{}v1/payments/{}", self.base_url(connectors), id))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = aci::AciCancelRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(PaymentsVoidType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: aci::AciPaymentsResponse =
            res.response
                .parse_struct("AciPaymentsResponse")
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

impl api::Refund for Aci {}
impl api::RefundExecute for Aci {}
impl api::RefundSync for Aci {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Aci {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            RefundExecuteType::get_content_type(self).to_string().into(),
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
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
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
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data = aci::AciRouterData::from((amount, req));
        let connector_req = aci::AciRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(RefundExecuteType::get_headers(self, req, connectors)?)
                .set_body(RefundExecuteType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: aci::AciRefundResponse = res
            .response
            .parse_struct("AciRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Aci {}

/// Decrypts an AES-256-GCM encrypted payload where the IV, auth tag, and ciphertext
/// are provided separately as hex strings. This is specifically tailored for ACI webhooks.
///
/// # Arguments
/// * `hex_key`: The encryption key as a hex string (must decode to 32 bytes).
/// * `hex_iv`: The initialization vector (nonce) as a hex string (must decode to 12 bytes).
/// * `hex_auth_tag`: The authentication tag as a hex string (must decode to 16 bytes).
/// * `hex_encrypted_body`: The encrypted payload as a hex string.
fn decrypt_aci_webhook_payload(
    hex_key: &str,
    hex_iv: &str,
    hex_auth_tag: &str,
    hex_encrypted_body: &str,
) -> CustomResult<Vec<u8>, CryptoError> {
    let (less_safe_key, nonce_arr) = aci_webhook_key_and_nonce(hex_key, hex_iv)?;
    let auth_tag_bytes = decode_aci_webhook_auth_tag(hex_auth_tag)?;
    let encrypted_body_bytes = hex::decode(hex_encrypted_body)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex encrypted body")?;
    let nonce = aead::Nonce::assume_unique_for_key(nonce_arr);

    let mut ciphertext_and_tag = encrypted_body_bytes;
    ciphertext_and_tag.extend_from_slice(&auth_tag_bytes);

    less_safe_key
        .open_in_place(nonce, aead::Aad::empty(), &mut ciphertext_and_tag)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decrypt payload using LessSafeKey")?;

    let original_ciphertext_len = ciphertext_and_tag.len() - auth_tag_bytes.len();
    ciphertext_and_tag.truncate(original_ciphertext_len);

    Ok(ciphertext_and_tag)
}

/// Parses the hex-encoded AES-256-GCM key and IV used by ACI webhooks.
fn aci_webhook_key_and_nonce(
    hex_key: &str,
    hex_iv: &str,
) -> CustomResult<(aead::LessSafeKey, [u8; aead::NONCE_LEN]), CryptoError> {
    let key_bytes = hex::decode(hex_key)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex key")?;
    let iv_bytes = hex::decode(hex_iv)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex IV")?;
    if key_bytes.len() != 32 {
        return Err(CryptoError::InvalidKeyLength)
            .attach_printable("Key must be 32 bytes for AES-256-GCM");
    }

    let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to create unbound key")?;
    let nonce_arr: [u8; aead::NONCE_LEN] = iv_bytes
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::InvalidIvLength)
        .attach_printable_lazy(|| {
            format!(
                "IV length is {} but expected {}",
                iv_bytes.len(),
                aead::NONCE_LEN
            )
        })?;

    Ok((aead::LessSafeKey::new(unbound_key), nonce_arr))
}

/// Decodes the hex-encoded AES-256-GCM authentication tag sent by ACI.
fn decode_aci_webhook_auth_tag(hex_auth_tag: &str) -> CustomResult<Vec<u8>, CryptoError> {
    let auth_tag_bytes = hex::decode(hex_auth_tag)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex auth tag")?;
    if auth_tag_bytes.len() != aead::AES_256_GCM.tag_len() {
        return Err(CryptoError::InvalidTagLength)
            .attach_printable("Auth tag must be 16 bytes for AES-256-GCM");
    }
    Ok(auth_tag_bytes)
}

/// Checks that `hex_auth_tag` is the AES-256-GCM tag of `plaintext` under this key and IV.
///
/// AES-GCM is deterministic for a given key and IV, so encrypting the decrypted body again
/// reproduces the ciphertext ACI sent. Opening that ciphertext with the received tag then
/// authenticates it, and `ring` compares the tag in constant time. If `plaintext` is anything
/// other than the body ACI encrypted (for example the raw ciphertext, when a webhook was not
/// decoded first), the tag does not match and this returns `false`.
fn verify_aci_webhook_auth_tag(
    hex_key: &str,
    hex_iv: &str,
    hex_auth_tag: &str,
    plaintext: &[u8],
) -> CustomResult<bool, CryptoError> {
    let (less_safe_key, nonce_arr) = aci_webhook_key_and_nonce(hex_key, hex_iv)?;
    let auth_tag_bytes = decode_aci_webhook_auth_tag(hex_auth_tag)?;

    let mut ciphertext_and_tag = plaintext.to_vec();
    // Only the ciphertext is needed: the received tag, not this recomputed one, is what
    // `open_in_place` checks below.
    let _recomputed_tag = less_safe_key
        .seal_in_place_separate_tag(
            aead::Nonce::assume_unique_for_key(nonce_arr),
            aead::Aad::empty(),
            &mut ciphertext_and_tag,
        )
        .change_context(CryptoError::EncodingFailed)
        .attach_printable("Failed to re-encrypt ACI webhook body")?;
    ciphertext_and_tag.extend_from_slice(&auth_tag_bytes);

    Ok(less_safe_key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce_arr),
            aead::Aad::empty(),
            &mut ciphertext_and_tag,
        )
        .is_ok())
}

/// Body decoding algorithm for ACI webhooks. ACI encrypts the notification body with
/// AES-256-GCM and sends the IV and authentication tag as hex strings in the
/// `X-Initialization-Vector` and `X-Authentication-Tag` headers.
struct AciWebhookBodyDecryption {
    iv_hex: String,
    auth_tag_hex: String,
}

impl crypto::DecodeMessage for AciWebhookBodyDecryption {
    fn decode_message(
        &self,
        secret: &[u8],
        msg: hyperswitch_masking::Secret<Vec<u8>, common_utils::pii::EncryptionStrategy>,
    ) -> CustomResult<Vec<u8>, CryptoError> {
        let hex_key = std::str::from_utf8(secret)
            .change_context(CryptoError::DecodingFailed)
            .attach_printable("ACI webhook secret is not a valid UTF-8 string")?;
        let encrypted_body_hex = String::from_utf8(msg.expose())
            .change_context(CryptoError::DecodingFailed)
            .attach_printable("ACI webhook body is not a valid UTF-8 string")?;

        decrypt_aci_webhook_payload(
            hex_key,
            &self.iv_hex,
            &self.auth_tag_hex,
            &encrypted_body_hex,
        )
    }
}

/// Source verification algorithm for ACI webhooks. The signature is the
/// `X-Authentication-Tag` header and the message is the decrypted body. See
/// [`verify_aci_webhook_auth_tag`].
struct AciWebhookAuthTagVerification {
    iv_hex: String,
}

impl crypto::VerifySignature for AciWebhookAuthTagVerification {
    fn verify_signature(
        &self,
        secret: &[u8],
        signature: &[u8],
        msg: &[u8],
    ) -> CustomResult<bool, CryptoError> {
        let hex_key = std::str::from_utf8(secret)
            .change_context(CryptoError::DecodingFailed)
            .attach_printable("ACI webhook secret is not a valid UTF-8 string")?;
        let auth_tag_hex = std::str::from_utf8(signature)
            .change_context(CryptoError::DecodingFailed)
            .attach_printable("ACI webhook authentication tag is not a valid UTF-8 string")?;

        verify_aci_webhook_auth_tag(hex_key, &self.iv_hex, auth_tag_hex, msg)
    }
}

fn get_aci_webhook_header(
    request: &IncomingWebhookRequestDetails<'_>,
    header_name: &'static str,
) -> CustomResult<String, errors::ConnectorError> {
    request
        .headers
        .get(header_name)
        .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)
        .attach_printable_lazy(|| format!("Missing {header_name} header"))?
        .to_str()
        .map(ToString::to_string)
        .map_err(|_| errors::ConnectorError::WebhookBodyDecodingFailed)
        .attach_printable_lazy(|| format!("Invalid {header_name} header value (not UTF-8)"))
}

#[async_trait::async_trait]
impl IncomingWebhook for Aci {
    fn get_webhook_body_decoding_algorithm(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::DecodeMessage + Send>, errors::ConnectorError> {
        Ok(Box::new(AciWebhookBodyDecryption {
            iv_hex: get_aci_webhook_header(request, "X-Initialization-Vector")?,
            auth_tag_hex: get_aci_webhook_header(request, "X-Authentication-Tag")?,
        }))
    }

    fn get_webhook_source_verification_algorithm(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(AciWebhookAuthTagVerification {
            iv_hex: get_aci_webhook_header(request, "X-Initialization-Vector")?,
        }))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(get_aci_webhook_header(request, "X-Authentication-Tag")?.into_bytes())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        // `decode_webhook_body` runs before source verification and replaces the body with the
        // decrypted plaintext, which `AciWebhookAuthTagVerification` re-encrypts to check the tag.
        Ok(request.body.to_vec())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let aci_notification: aci::AciWebhookNotification =
            serde_json::from_slice(request.body)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)
                .attach_printable("Failed to deserialize ACI webhook notification for ID extraction (expected decrypted payload)")?;

        let id_value_str = aci_notification
            .payload
            .get("id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| {
                report!(errors::ConnectorError::WebhookResourceObjectNotFound)
                    .attach_printable("Missing 'id' in webhook payload for ID extraction")
            })?;

        let payment_type_str = aci_notification
            .payload
            .get("paymentType")
            .and_then(|pt| pt.as_str());

        if payment_type_str.is_some_and(|pt| pt.to_uppercase() == "RF") {
            Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::ConnectorRefundId(id_value_str.to_string()),
            ))
        } else {
            Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    id_value_str.to_string(),
                ),
            ))
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _context: Option<&WebhookContext>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        let aci_notification: aci::AciWebhookNotification =
            serde_json::from_slice(request.body)
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)
                 .attach_printable("Failed to deserialize ACI webhook notification for event type (expected decrypted payload)")?;

        match aci_notification.event_type {
            aci::AciWebhookEventType::Payment => {
                let payment_payload: aci::AciPaymentWebhookPayload =
                    serde_json::from_value(aci_notification.payload)
                        .change_context(errors::ConnectorError::WebhookEventTypeNotFound)
                        .attach_printable("Could not deserialize ACI payment webhook payload for event type determination")?;

                let code = &payment_payload.result.code;
                if aci_result_codes::SUCCESSFUL_CODES.contains(&code.as_str()) {
                    if payment_payload.payment_type.to_uppercase() == "RF" {
                        Ok(IncomingWebhookEvent::RefundSuccess)
                    } else {
                        Ok(IncomingWebhookEvent::PaymentIntentSuccess)
                    }
                } else if aci_result_codes::PENDING_CODES.contains(&code.as_str()) {
                    if payment_payload.payment_type.to_uppercase() == "RF" {
                        Ok(IncomingWebhookEvent::EventNotSupported)
                    } else {
                        Ok(IncomingWebhookEvent::PaymentIntentProcessing)
                    }
                } else if aci_result_codes::FAILURE_CODES.contains(&code.as_str()) {
                    if payment_payload.payment_type.to_uppercase() == "RF" {
                        Ok(IncomingWebhookEvent::RefundFailure)
                    } else {
                        Ok(IncomingWebhookEvent::PaymentIntentFailure)
                    }
                } else {
                    Ok(IncomingWebhookEvent::EventNotSupported)
                }
            }
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        let aci_notification: aci::AciWebhookNotification =
            serde_json::from_slice(request.body)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)
                .attach_printable("Failed to deserialize ACI webhook notification for resource object (expected decrypted payload)")?;

        match aci_notification.event_type {
            aci::AciWebhookEventType::Payment => {
                let payment_payload: aci::AciPaymentWebhookPayload =
                    serde_json::from_value(aci_notification.payload)
                        .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)
                        .attach_printable("Failed to deserialize ACI payment webhook payload")?;
                Ok(Box::new(payment_payload))
            }
        }
    }
}

static ACI_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::Manual,
    ];

    let supported_card_networks = vec![
        common_enums::CardNetwork::Visa,
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::AmericanExpress,
        common_enums::CardNetwork::JCB,
        common_enums::CardNetwork::DinersClub,
        common_enums::CardNetwork::Discover,
        common_enums::CardNetwork::UnionPay,
        common_enums::CardNetwork::Maestro,
    ];

    let mut aci_supported_payment_methods = SupportedPaymentMethods::new();

    aci_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::MbWay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    aci_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::AliPay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    aci_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_networks.clone(),
                    }
                }),
            ),
        },
    );

    aci_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Debit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_networks.clone(),
                    }
                }),
            ),
        },
    );

    aci_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Eps,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    aci_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Eft,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    aci_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Ideal,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    aci_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Giropay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    aci_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Sofort,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    aci_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Interac,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    aci_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Przelewy24,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    aci_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Trustly,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    aci_supported_payment_methods.add(
        enums::PaymentMethod::PayLater,
        enums::PaymentMethodType::Klarna,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    aci_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::ApplePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    aci_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::GooglePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    aci_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::SamsungPay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods,
            specific_features: None,
        },
    );

    aci_supported_payment_methods
});

static ACI_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "ACI",
    description:
        "ACI Payments delivers secure, real-time electronic payment solutions for businesses, banks, and governments, enabling seamless transactions across channels.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Sandbox,
};

static ACI_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Aci {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&ACI_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*ACI_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&ACI_SUPPORTED_WEBHOOK_FLOWS)
    }
}

#[cfg(test)]
mod tests {
    use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
    use common_utils::{
        crypto::Encryptable,
        errors::{CryptoError, CustomResult},
        id_type::MerchantId,
        pii::SecretSerdeValue,
    };
    use error_stack::ResultExt;
    use hyperswitch_interfaces::{
        errors::ConnectorError,
        webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
    };
    use hyperswitch_masking::Secret;
    use ring::aead;

    use super::Aci;

    const WEBHOOK_SECRET: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    const IV_HEX: &str = "0a0b0c0d0e0f101112131415";

    fn webhook_payload() -> Vec<u8> {
        serde_json::json!({
            "type": "PAYMENT",
            "payload": {
                "id": "8ac7a4a18d2f4d6f018d30a2c2b54f7e",
                "paymentType": "DB",
                "paymentBrand": "VISA",
                "amount": "10.00",
                "currency": "EUR",
                "result": {
                    "code": "000.100.110",
                    "description": "Request successfully processed in 'Merchant in Integrator Test Mode'"
                },
                "timestamp": "2026-09-23 10:00:00+0000",
                "ndc": "8a8294174b7ecb28014b9699220015ca_7a9ec8f6e3d64f1e9a1e1f0c0f0e0d0c"
            }
        })
        .to_string()
        .into_bytes()
    }

    /// Encrypts `plaintext` the way ACI does: AES-256-GCM, returning the hex-encoded
    /// ciphertext (request body) and the hex-encoded authentication tag (header).
    fn encrypt_like_aci(plaintext: &[u8]) -> (String, String) {
        let key_bytes = hex::decode(WEBHOOK_SECRET).unwrap();
        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes).unwrap();
        let key = aead::LessSafeKey::new(unbound_key);
        let nonce_bytes: [u8; aead::NONCE_LEN] = hex::decode(IV_HEX).unwrap().try_into().unwrap();
        let mut in_out = plaintext.to_vec();
        let tag = key
            .seal_in_place_separate_tag(
                aead::Nonce::assume_unique_for_key(nonce_bytes),
                aead::Aad::empty(),
                &mut in_out,
            )
            .unwrap();
        (hex::encode_upper(in_out), hex::encode_upper(tag.as_ref()))
    }

    fn headers(auth_tag_hex: &str) -> actix_web::http::header::HeaderMap {
        let mut headers = actix_web::http::header::HeaderMap::new();
        headers.insert(
            actix_web::http::header::HeaderName::from_static("x-initialization-vector"),
            actix_web::http::header::HeaderValue::from_static(IV_HEX),
        );
        headers.insert(
            actix_web::http::header::HeaderName::from_static("x-authentication-tag"),
            actix_web::http::header::HeaderValue::from_str(auth_tag_hex).unwrap(),
        );
        headers
    }

    fn request<'a>(
        headers: &'a actix_web::http::header::HeaderMap,
        body: &'a [u8],
    ) -> IncomingWebhookRequestDetails<'a> {
        IncomingWebhookRequestDetails {
            method: http::Method::POST,
            uri: http::Uri::from_static("/webhooks"),
            headers,
            body,
            query_params: String::new(),
        }
    }

    fn decode_body(
        request: &IncomingWebhookRequestDetails<'_>,
        secret: &[u8],
    ) -> CustomResult<Vec<u8>, CryptoError> {
        Aci::new()
            .get_webhook_body_decoding_algorithm(request)
            .change_context(CryptoError::DecodingFailed)?
            .decode_message(secret, Secret::new(request.body.to_vec()))
    }

    fn webhook_details(secret: &str) -> Option<SecretSerdeValue> {
        Some(Secret::new(
            serde_json::json!({ "merchant_secret": secret }),
        ))
    }

    /// Runs ACI's `verify_webhook_source`, the same entry point the webhook flows call.
    fn verify_source(
        request: &IncomingWebhookRequestDetails<'_>,
        connector_webhook_details: Option<SecretSerdeValue>,
    ) -> CustomResult<bool, ConnectorError> {
        actix_web::rt::System::new().block_on(Aci::new().verify_webhook_source(
            request,
            &MerchantId::default(),
            connector_webhook_details,
            Encryptable::new(
                Secret::new(serde_json::Value::Null),
                Secret::new(Vec::new()),
            ),
            "aci",
        ))
    }

    #[test]
    fn decrypts_encrypted_webhook_body_before_parsing() {
        let plaintext = webhook_payload();
        let (encrypted_body, auth_tag) = encrypt_like_aci(&plaintext);
        let headers = headers(&auth_tag);
        let encrypted_request = request(&headers, encrypted_body.as_bytes());

        let decoded_body = decode_body(&encrypted_request, WEBHOOK_SECRET.as_bytes()).unwrap();
        assert_eq!(decoded_body, plaintext);

        let decoded_request = request(&headers, &decoded_body);
        let reference = Aci::new()
            .get_webhook_object_reference_id(&decoded_request)
            .unwrap();
        assert!(matches!(
            reference,
            ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(ref id)
            ) if id == "8ac7a4a18d2f4d6f018d30a2c2b54f7e"
        ));
        assert_eq!(
            Aci::new()
                .get_webhook_event_type(&decoded_request, None)
                .unwrap(),
            IncomingWebhookEvent::PaymentIntentSuccess
        );
        assert!(Aci::new()
            .get_webhook_resource_object(&decoded_request)
            .is_ok());
    }

    #[test]
    fn rejects_webhook_body_with_mismatched_authentication_tag() {
        let (encrypted_body, _) = encrypt_like_aci(&webhook_payload());
        let headers = headers("00000000000000000000000000000000");
        let encrypted_request = request(&headers, encrypted_body.as_bytes());

        assert!(decode_body(&encrypted_request, WEBHOOK_SECRET.as_bytes()).is_err());
    }

    #[test]
    fn rejects_webhook_body_encrypted_with_another_secret() {
        let (encrypted_body, auth_tag) = encrypt_like_aci(&webhook_payload());
        let headers = headers(&auth_tag);
        let encrypted_request = request(&headers, encrypted_body.as_bytes());

        let other_secret = b"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert!(decode_body(&encrypted_request, other_secret).is_err());
    }

    /// A merchant without a configured webhook secret gets the literal `"default_secret"`,
    /// which is not a valid hex key, so the webhook must be rejected.
    #[test]
    fn rejects_webhook_when_merchant_has_no_webhook_secret_configured() {
        let (encrypted_body, auth_tag) = encrypt_like_aci(&webhook_payload());
        let headers = headers(&auth_tag);
        let encrypted_request = request(&headers, encrypted_body.as_bytes());

        assert!(decode_body(&encrypted_request, b"default_secret").is_err());
    }

    #[test]
    fn rejects_webhook_without_initialization_vector_header() {
        let (encrypted_body, _) = encrypt_like_aci(&webhook_payload());
        let headers = actix_web::http::header::HeaderMap::new();
        let encrypted_request = request(&headers, encrypted_body.as_bytes());

        assert!(Aci::new()
            .get_webhook_body_decoding_algorithm(&encrypted_request)
            .is_err());
    }

    #[test]
    fn verifies_source_of_decoded_webhook_body() {
        let plaintext = webhook_payload();
        let (encrypted_body, auth_tag) = encrypt_like_aci(&plaintext);
        let headers = headers(&auth_tag);
        let encrypted_request = request(&headers, encrypted_body.as_bytes());
        let decoded_body = decode_body(&encrypted_request, WEBHOOK_SECRET.as_bytes()).unwrap();
        let decoded_request = request(&headers, &decoded_body);

        assert!(verify_source(&decoded_request, webhook_details(WEBHOOK_SECRET)).unwrap());
    }

    /// Source verification must not depend on the body having been decoded first: if a flow
    /// ever verifies the raw ciphertext, the webhook must not be marked as verified.
    #[test]
    fn does_not_verify_source_of_webhook_body_that_was_not_decoded() {
        let (encrypted_body, auth_tag) = encrypt_like_aci(&webhook_payload());
        let headers = headers(&auth_tag);
        let encrypted_request = request(&headers, encrypted_body.as_bytes());

        assert!(!verify_source(&encrypted_request, webhook_details(WEBHOOK_SECRET)).unwrap());
    }

    #[test]
    fn does_not_verify_source_of_forged_webhook_body() {
        let (_, auth_tag) = encrypt_like_aci(&webhook_payload());
        let headers = headers(&auth_tag);
        let forged_body = br#"{"type":"PAYMENT","payload":{"id":"forged"}}"#;
        let forged_request = request(&headers, forged_body);

        assert!(!verify_source(&forged_request, webhook_details(WEBHOOK_SECRET)).unwrap());
    }

    #[test]
    fn does_not_verify_source_with_another_secret() {
        let plaintext = webhook_payload();
        let (_, auth_tag) = encrypt_like_aci(&plaintext);
        let headers = headers(&auth_tag);
        let decoded_request = request(&headers, &plaintext);
        let other_secret = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

        assert!(!verify_source(&decoded_request, webhook_details(other_secret)).unwrap());
    }

    /// Without a configured secret the key is the literal `"default_secret"`, which is not a
    /// valid hex key. The gateway treats this error as "not verified".
    #[test]
    fn does_not_verify_source_when_merchant_has_no_webhook_secret_configured() {
        let plaintext = webhook_payload();
        let (_, auth_tag) = encrypt_like_aci(&plaintext);
        let headers = headers(&auth_tag);
        let decoded_request = request(&headers, &plaintext);

        let result = verify_source(&decoded_request, None);
        assert!(matches!(
            result,
            Err(ref error)
                if matches!(error.current_context(), ConnectorError::WebhookSourceVerificationFailed)
        ));
    }
}
