pub mod transformers;

use std::sync::LazyLock;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    crypto::{self, SignMessage},
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt, ValueExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
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
        SupportedPaymentMethods,
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
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{ExposeInterface, Mask, PeekInterface, Secret};
use transformers as payjustnow;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self},
};

#[derive(Clone)]
pub struct Payjustnow {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Payjustnow {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl api::Payment for Payjustnow {}
impl api::PaymentSession for Payjustnow {}
impl api::ConnectorAccessToken for Payjustnow {}
impl api::MandateSetup for Payjustnow {}
impl api::PaymentAuthorize for Payjustnow {}
impl api::PaymentSync for Payjustnow {}
impl api::PaymentCapture for Payjustnow {}
impl api::PaymentVoid for Payjustnow {}
impl api::Refund for Payjustnow {}
impl api::RefundExecute for Payjustnow {}
impl api::RefundSync for Payjustnow {}
impl api::PaymentToken for Payjustnow {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Payjustnow
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Payjustnow
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let request_body = Self::get_request_body(self, req, connectors)?;

        let request_body_string =
            String::from_utf8(request_body.get_inner_value().peek().as_bytes().to_vec())
                .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;
        let request_body_string_without_whitespace =
            request_body_string.replace(char::is_whitespace, "");

        let auth = payjustnow::PayjustnowAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let signature = crypto::HmacSha256::sign_message(
            &crypto::HmacSha256,
            auth.signing_key.expose().as_bytes(),
            request_body_string_without_whitespace.as_bytes(),
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let signature_base64 = base64::engine::general_purpose::STANDARD.encode(signature);

        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::USER_AGENT.to_string(),
                hyperswitch_interfaces::consts::USER_AGENT
                    .to_string()
                    .into(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        headers.push(("X-Signature".to_string(), signature_base64.into_masked()));

        Ok(headers)
    }
}

impl ConnectorCommon for Payjustnow {
    fn id(&self) -> &'static str {
        "payjustnow"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.payjustnow.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = payjustnow::PayjustnowAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            "X-Merchant-Account-ID".to_string(),
            auth.merchant_account_id.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: payjustnow::PayjustnowErrorResponse = res
            .response
            .parse_struct("PayjustnowErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let message = match response {
            payjustnow::PayjustnowErrorResponse::Structured(error) => error.message,
            payjustnow::PayjustnowErrorResponse::Message(message) => message,
        };

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
            message,
            reason: None,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Payjustnow {
    fn validate_mandate_payment(
        &self,
        _pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        match pm_data {
            PaymentMethodData::Card(_) => Err(errors::ConnectorError::NotImplemented(
                "validate_mandate_payment does not support cards".to_string(),
            )
            .into()),
            _ => Ok(()),
        }
    }

    fn validate_psync_reference_id(
        &self,
        data: &PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        if data.encoded_data.is_some()
            || data
                .connector_transaction_id
                .get_connector_transaction_id()
                .is_ok()
        {
            return Ok(());
        }

        Err(errors::ConnectorError::MissingConnectorTransactionID.into())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Payjustnow {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Payjustnow {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Payjustnow
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Payjustnow {
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
        Ok(format!("{}/create", self.base_url(connectors)))
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

        let connector_router_data = payjustnow::PayjustnowRouterData::from((amount, req));
        let connector_req =
            payjustnow::PayjustnowPaymentsRequest::try_from(&connector_router_data)?;

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
                .headers(self.get_headers(req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: payjustnow::PayjustnowPaymentsResponse = res
            .response
            .parse_struct("Payjustnow PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Payjustnow {
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
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/getstatus", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = payjustnow::PayjustnowSyncRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: payjustnow::PayjustnowSyncResponse = res
            .response
            .parse_struct("payjustnow PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Payjustnow {
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
        let response: payjustnow::PayjustnowPaymentsResponse = res
            .response
            .parse_struct("Payjustnow PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Payjustnow {
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
        _req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/cancel", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = payjustnow::PayjustnowCancelRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: payjustnow::PayjustnowSyncResponse = res
            .response
            .parse_struct("payjustnow PaymentsCancelResponse")
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Payjustnow {
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
        Ok(format!("{}/refund", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = payjustnow::PayjustnowRefundRequest::try_from(req)?;
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
            .headers(self.get_headers(req, connectors)?)
            .set_body(self.get_request_body(req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: payjustnow::PayjustnowRefundResponse = res
            .response
            .parse_struct("payjustnow RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Payjustnow {
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
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/getstatus", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = payjustnow::PayjustnowSyncRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: payjustnow::PayjustnowRsyncResponse = res
            .response
            .parse_struct("payjustnow RefundSyncResponse")
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
impl webhooks::IncomingWebhook for Payjustnow {
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: payjustnow::PayjustnowWebhookDetails = request
            .body
            .parse_struct("PayjustnowWebhookDetails")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(details.checkout_token),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let details: payjustnow::PayjustnowWebhookDetails = request
            .body
            .parse_struct("PayjustnowWebhookDetails")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let event_type = match details.checkout_payment_status {
            payjustnow::PayjustnowWebhookStatus::PaidPendingCallback => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
            }
        };
        Ok(event_type)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let details: payjustnow::PayjustnowWebhookDetails = request
            .body
            .parse_struct("PayjustnowWebhookDetails")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(details))
    }

    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let signature = request
            .headers
            .get("x-signature")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?
            .to_str()
            .map_err(|_| errors::ConnectorError::WebhookSignatureNotFound)?
            .to_string();
        let decoded_signature = base64::engine::general_purpose::STANDARD
            .decode(signature)
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
        Ok(decoded_signature)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let message = request
            .body
            .iter()
            .filter(|&b| !b.is_ascii_whitespace())
            .copied()
            .collect::<Vec<u8>>();

        Ok(message)
    }

    async fn verify_webhook_source(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
        _connector_name: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_auth_type: ConnectorAuthType = connector_account_details
            .parse_value("ConnectorAuthType")
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth = payjustnow::PayjustnowAuthType::try_from(&connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let signing_key = auth.signing_key.expose().into_bytes();
        let webhook_secret = api_models::webhooks::ConnectorWebhookSecrets {
            secret: signing_key.clone(),
            additional_secret: None,
        };
        let signature = self.get_webhook_source_verification_signature(request, &webhook_secret)?;
        let message =
            self.get_webhook_source_verification_message(request, merchant_id, &webhook_secret)?;
        let algorithm = self.get_webhook_source_verification_algorithm(request)?;
        algorithm
            .verify_signature(&signing_key, &signature, &message)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    }
}

use hyperswitch_domain_models::router_response_types::SupportedPaymentMethodsExt;
static PAYJUSTNOW_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let mut payjustnow_supported_payment_methods = SupportedPaymentMethods::new();

        payjustnow_supported_payment_methods.add(
            enums::PaymentMethod::PayLater,
            enums::PaymentMethodType::Payjustnow,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        payjustnow_supported_payment_methods
    });

static PAYJUSTNOW_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Payjustnow",
    description: "PayJustNow is a South African payment connector that enables customers to split online purchases into three interest-free monthly installments.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Sandbox,
};

static PAYJUSTNOW_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] = [enums::EventClass::Payments];

impl ConnectorSpecifications for Payjustnow {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&PAYJUSTNOW_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*PAYJUSTNOW_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&PAYJUSTNOW_SUPPORTED_WEBHOOK_FLOWS)
    }
}
