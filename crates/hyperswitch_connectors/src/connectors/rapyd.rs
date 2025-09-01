pub mod transformers;
use std::sync::LazyLock;

use api_models::webhooks::IncomingWebhookEvent;
use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts::BASE64_ENGINE_URL_SAFE,
    crypto, date_time,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt, Encode, StringExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{
        AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector, StringMinorUnit,
        StringMinorUnitForConnector,
    },
};
use error_stack::{Report, ResultExt};
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
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorIntegration, ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    disputes::DisputePayload,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::{ExposeInterface, Mask, PeekInterface, Secret};
use rand::distributions::{Alphanumeric, DistString};
use ring::hmac;
use router_env::logger;
use transformers as rapyd;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, convert_amount, get_header_key_value},
};

#[derive(Clone)]
pub struct Rapyd {
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
    amount_converter_webhooks: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}
impl Rapyd {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &FloatMajorUnitForConnector,
            amount_converter_webhooks: &StringMinorUnitForConnector,
        }
    }
}
impl Rapyd {
    pub fn generate_signature(
        &self,
        auth: &rapyd::RapydAuthType,
        http_method: &str,
        url_path: &str,
        body: &str,
        timestamp: i64,
        salt: &str,
    ) -> CustomResult<String, errors::ConnectorError> {
        let rapyd::RapydAuthType {
            access_key,
            secret_key,
        } = auth;
        let to_sign = format!(
            "{http_method}{url_path}{salt}{timestamp}{}{}{body}",
            access_key.peek(),
            secret_key.peek()
        );
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret_key.peek().as_bytes());
        let tag = hmac::sign(&key, to_sign.as_bytes());
        let hmac_sign = hex::encode(tag);
        let signature_value = BASE64_ENGINE_URL_SAFE.encode(hmac_sign);
        Ok(signature_value)
    }
}

impl ConnectorCommon for Rapyd {
    fn id(&self) -> &'static str {
        "rapyd"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.rapyd.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: Result<
            rapyd::RapydPaymentsResponse,
            Report<common_utils::errors::ParsingError>,
        > = res.response.parse_struct("Rapyd ErrorResponse");

        match response {
            Ok(response_data) => {
                event_builder.map(|i| i.set_error_response_body(&response_data));
                router_env::logger::info!(connector_response=?response_data);
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: response_data.status.error_code,
                    message: response_data.status.status.unwrap_or_default(),
                    reason: response_data.status.message,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Err(error_msg) => {
                event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
                logger::error!(deserialization_error =? error_msg);
                utils::handle_json_response_deserialization_failure(res, "rapyd")
            }
        }
    }
}

impl ConnectorValidation for Rapyd {}

impl api::ConnectorAccessToken for Rapyd {}

impl api::PaymentToken for Rapyd {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Rapyd
{
    // Not Implemented (R)
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Rapyd {}

impl api::PaymentAuthorize for Rapyd {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Rapyd {
    fn get_headers(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v1/payments", self.base_url(connectors)))
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
        let connector_router_data = rapyd::RapydRouterData::from((amount, req));
        let connector_req = rapyd::RapydPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let body = types::PaymentsAuthorizeType::get_request_body(self, req, connectors)?;
        let req_body = body.get_inner_value().expose();
        let signature =
            self.generate_signature(&auth, "post", "/v1/payments", &req_body, timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key.into_masked()),
            ("salt".to_string(), salt.into_masked()),
            ("timestamp".to_string(), timestamp.to_string().into()),
            ("signature".to_string(), signature.into_masked()),
        ];
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PaymentsAuthorizeType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::PaymentsAuthorizeType::get_headers(
                self, req, connectors,
            )?)
            .headers(headers)
            .set_body(types::PaymentsAuthorizeType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd PaymentResponse")
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

impl api::Payment for Rapyd {}

impl api::MandateSetup for Rapyd {}
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Rapyd {
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Rapyd".to_string())
                .into(),
        )
    }
}

impl api::PaymentVoid for Rapyd {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Rapyd {
    fn get_headers(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsVoidType::get_content_type(self)
                .to_string()
                .into(),
        )])
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
            "{}/v1/payments/{}",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let url_path = format!("/v1/payments/{}", req.request.connector_transaction_id);
        let signature =
            self.generate_signature(&auth, "delete", &url_path, "", timestamp, &salt)?;

        let headers = vec![
            ("access_key".to_string(), auth.access_key.into_masked()),
            ("salt".to_string(), salt.into_masked()),
            ("timestamp".to_string(), timestamp.to_string().into()),
            ("signature".to_string(), signature.into_masked()),
        ];
        let request = RequestBuilder::new()
            .method(Method::Delete)
            .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
            .headers(headers)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd PaymentResponse")
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

impl api::PaymentSync for Rapyd {}
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Rapyd {
    fn get_headers(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsSyncType::get_content_type(self)
                .to_string()
                .into(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}/v1/payments/{}",
            self.base_url(connectors),
            id.get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let response_id = req.request.connector_transaction_id.clone();
        let url_path = format!(
            "/v1/payments/{}",
            response_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
        );
        let signature = self.generate_signature(&auth, "get", &url_path, "", timestamp, &salt)?;

        let headers = vec![
            ("access_key".to_string(), auth.access_key.into_masked()),
            ("salt".to_string(), salt.into_masked()),
            ("timestamp".to_string(), timestamp.to_string().into()),
            ("signature".to_string(), signature.into_masked()),
        ];
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
            .headers(headers)
            .build();
        Ok(Some(request))
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("Rapyd PaymentResponse")
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
}

impl api::PaymentCapture for Rapyd {}
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Rapyd {
    fn get_headers(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsCaptureType::get_content_type(self)
                .to_string()
                .into(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
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
        let connector_router_data = rapyd::RapydRouterData::from((amount, req));
        let connector_req = rapyd::CaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let url_path = format!(
            "/v1/payments/{}/capture",
            req.request.connector_transaction_id
        );
        let body = types::PaymentsCaptureType::get_request_body(self, req, connectors)?;
        let req_body = body.get_inner_value().expose();
        let signature =
            self.generate_signature(&auth, "post", &url_path, &req_body, timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key.into_masked()),
            ("salt".to_string(), salt.into_masked()),
            ("timestamp".to_string(), timestamp.to_string().into()),
            ("signature".to_string(), signature.into_masked()),
        ];
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsCaptureType::get_headers(
                self, req, connectors,
            )?)
            .headers(headers)
            .set_body(types::PaymentsCaptureType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: rapyd::RapydPaymentsResponse = res
            .response
            .parse_struct("RapydPaymentResponse")
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

    fn get_url(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v1/payments/{}/capture",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::PaymentSession for Rapyd {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Rapyd {
    //TODO: implement sessions flow
}

impl api::Refund for Rapyd {}
impl api::RefundExecute for Rapyd {}
impl api::RefundSync for Rapyd {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Rapyd {
    fn get_headers(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::RefundExecuteType::get_content_type(self)
                .to_string()
                .into(),
        )])
    }

    fn get_content_type(&self) -> &'static str {
        ConnectorCommon::common_get_content_type(self)
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v1/refunds", self.base_url(connectors)))
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
        let connector_router_data = rapyd::RapydRouterData::from((amount, req));
        let connector_req = rapyd::RapydRefundRequest::try_from(&connector_router_data)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let timestamp = date_time::now_unix_timestamp();
        let salt = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);

        let body = types::RefundExecuteType::get_request_body(self, req, connectors)?;
        let req_body = body.get_inner_value().expose();
        let auth: rapyd::RapydAuthType = rapyd::RapydAuthType::try_from(&req.connector_auth_type)?;
        let signature =
            self.generate_signature(&auth, "post", "/v1/refunds", &req_body, timestamp, &salt)?;
        let headers = vec![
            ("access_key".to_string(), auth.access_key.into_masked()),
            ("salt".to_string(), salt.into_masked()),
            ("timestamp".to_string(), timestamp.to_string().into()),
            ("signature".to_string(), signature.into_masked()),
        ];
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(headers)
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
        let response: rapyd::RefundResponse = res
            .response
            .parse_struct("rapyd RefundResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Rapyd {
    // default implementation of build_request method will be executed
    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: rapyd::RefundResponse = res
            .response
            .parse_struct("rapyd RefundResponse")
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
}

#[async_trait::async_trait]
impl IncomingWebhook for Rapyd {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let base64_signature = get_header_key_value("signature", request.headers)?;
        let signature = BASE64_ENGINE_URL_SAFE
            .decode(base64_signature.as_bytes())
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(signature)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let host = get_header_key_value("host", request.headers)?;
        let connector = self.id();
        let url_path = format!(
            "https://{host}/webhooks/{}/{connector}",
            merchant_id.get_string_repr()
        );
        let salt = get_header_key_value("salt", request.headers)?;
        let timestamp = get_header_key_value("timestamp", request.headers)?;
        let stringify_auth = String::from_utf8(connector_webhook_secrets.secret.to_vec())
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        let auth: transformers::RapydAuthType = stringify_auth
            .parse_struct("RapydAuthType")
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let access_key = auth.access_key;
        let secret_key = auth.secret_key;
        let body_string = String::from_utf8(request.body.to_vec())
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert body to UTF-8")?;
        let to_sign = format!(
            "{url_path}{salt}{timestamp}{}{}{body_string}",
            access_key.peek(),
            secret_key.peek()
        );

        Ok(to_sign.into_bytes())
    }

    async fn verify_webhook_source(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
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
        let message = self
            .get_webhook_source_verification_message(
                request,
                merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let stringify_auth = String::from_utf8(connector_webhook_secrets.secret.to_vec())
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        let auth: transformers::RapydAuthType = stringify_auth
            .parse_struct("RapydAuthType")
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let secret_key = auth.secret_key;
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret_key.peek().as_bytes());
        let tag = hmac::sign(&key, &message);
        let hmac_sign = hex::encode(tag);
        Ok(hmac_sign.as_bytes().eq(&signature))
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook: transformers::RapydIncomingWebhook = request
            .body
            .parse_struct("RapydIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(match webhook.data {
            transformers::WebhookData::Payment(payment_data) => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(payment_data.id),
                )
            }
            transformers::WebhookData::Refund(refund_data) => {
                api_models::webhooks::ObjectReferenceId::RefundId(
                    api_models::webhooks::RefundIdType::ConnectorRefundId(refund_data.id),
                )
            }
            transformers::WebhookData::Dispute(dispute_data) => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        dispute_data.original_transaction_id,
                    ),
                )
            }
        })
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        let webhook: transformers::RapydIncomingWebhook = request
            .body
            .parse_struct("RapydIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(match webhook.webhook_type {
            rapyd::RapydWebhookObjectEventType::PaymentCompleted
            | rapyd::RapydWebhookObjectEventType::PaymentCaptured => {
                IncomingWebhookEvent::PaymentIntentSuccess
            }
            rapyd::RapydWebhookObjectEventType::PaymentFailed => {
                IncomingWebhookEvent::PaymentIntentFailure
            }
            rapyd::RapydWebhookObjectEventType::PaymentRefundFailed
            | rapyd::RapydWebhookObjectEventType::PaymentRefundRejected => {
                IncomingWebhookEvent::RefundFailure
            }
            rapyd::RapydWebhookObjectEventType::RefundCompleted => {
                IncomingWebhookEvent::RefundSuccess
            }
            rapyd::RapydWebhookObjectEventType::PaymentDisputeCreated => {
                IncomingWebhookEvent::DisputeOpened
            }
            rapyd::RapydWebhookObjectEventType::Unknown => IncomingWebhookEvent::EventNotSupported,
            rapyd::RapydWebhookObjectEventType::PaymentDisputeUpdated => match webhook.data {
                rapyd::WebhookData::Dispute(data) => IncomingWebhookEvent::from(data.status),
                _ => IncomingWebhookEvent::EventNotSupported,
            },
        })
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook: transformers::RapydIncomingWebhook = request
            .body
            .parse_struct("RapydIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let res_json = match webhook.data {
            transformers::WebhookData::Payment(payment_data) => {
                let rapyd_response: transformers::RapydPaymentsResponse = payment_data.into();

                rapyd_response
                    .encode_to_value()
                    .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?
            }
            transformers::WebhookData::Refund(refund_data) => refund_data
                .encode_to_value()
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?,
            transformers::WebhookData::Dispute(dispute_data) => dispute_data
                .encode_to_value()
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?,
        };
        Ok(Box::new(res_json))
    }

    fn get_dispute_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<DisputePayload, errors::ConnectorError> {
        let webhook: transformers::RapydIncomingWebhook = request
            .body
            .parse_struct("RapydIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let webhook_dispute_data = match webhook.data {
            transformers::WebhookData::Dispute(dispute_data) => Ok(dispute_data),
            _ => Err(errors::ConnectorError::WebhookBodyDecodingFailed),
        }?;
        Ok(DisputePayload {
            amount: convert_amount(
                self.amount_converter_webhooks,
                webhook_dispute_data.amount,
                webhook_dispute_data.currency,
            )?,
            currency: webhook_dispute_data.currency,
            dispute_stage: api_models::enums::DisputeStage::Dispute,
            connector_dispute_id: webhook_dispute_data.token,
            connector_reason: Some(webhook_dispute_data.dispute_reason_description),
            connector_reason_code: None,
            challenge_required_by: webhook_dispute_data.due_date,
            connector_status: webhook_dispute_data.status.to_string(),
            created_at: webhook_dispute_data.created_at,
            updated_at: webhook_dispute_data.updated_at,
        })
    }
}

static RAPYD_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::Manual,
        enums::CaptureMethod::SequentialAutomatic,
    ];

    let supported_card_network = vec![
        common_enums::CardNetwork::AmericanExpress,
        common_enums::CardNetwork::Visa,
        common_enums::CardNetwork::JCB,
        common_enums::CardNetwork::DinersClub,
        common_enums::CardNetwork::UnionPay,
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::Discover,
    ];

    let mut rapyd_supported_payment_methods = SupportedPaymentMethods::new();

    rapyd_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::ApplePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    rapyd_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::GooglePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    rapyd_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    }
                }),
            ),
        },
    );

    rapyd_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Debit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    }
                }),
            ),
        },
    );

    rapyd_supported_payment_methods
});

static RAPYD_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Rapyd",
    description: "Rapyd is a fintech company that enables businesses to collect payments in local currencies across the globe ",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Sandbox,
};

static RAPYD_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 3] = [
    enums::EventClass::Payments,
    enums::EventClass::Refunds,
    enums::EventClass::Disputes,
];

impl ConnectorSpecifications for Rapyd {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&RAPYD_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*RAPYD_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&RAPYD_SUPPORTED_WEBHOOK_FLOWS)
    }
}
