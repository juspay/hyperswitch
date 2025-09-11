pub mod transformers;

use std::{fmt::Debug, sync::LazyLock};

use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts, crypto,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, OptionExt},
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
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
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation, PaymentCapture, PaymentSync,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        PaymentsAuthorizeType, PaymentsCaptureType, PaymentsSyncType, PaymentsVoidType,
        RefundExecuteType, RefundSyncType, Response,
    },
    webhooks::{self, IncomingWebhookFlowError},
};
use masking::{ExposeInterface, Mask, PeekInterface};
use ring::hmac;
use router_env::logger;
use time::{format_description, OffsetDateTime};
use transformers as worldline;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, RefundsRequestData as _},
};

#[derive(Debug, Clone)]
pub struct Worldline;

impl Worldline {
    pub fn generate_authorization_token(
        &self,
        auth: worldline::WorldlineAuthType,
        http_method: Method,
        content_type: &str,
        date: &str,
        endpoint: &str,
    ) -> CustomResult<String, errors::ConnectorError> {
        let signature_data: String = format!(
            "{}\n{}\n{}\n/{}\n",
            http_method,
            content_type.trim(),
            date.trim(),
            endpoint.trim()
        );
        let worldline::WorldlineAuthType {
            api_key,
            api_secret,
            ..
        } = auth;
        let key = hmac::Key::new(hmac::HMAC_SHA256, api_secret.expose().as_bytes());
        let signed_data = consts::BASE64_ENGINE.encode(hmac::sign(&key, signature_data.as_bytes()));

        Ok(format!("GCS v1HMAC:{}:{signed_data}", api_key.peek()))
    }

    pub fn get_current_date_time() -> CustomResult<String, errors::ConnectorError> {
        let format = format_description::parse(
            "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT",
        )
        .change_context(errors::ConnectorError::InvalidDateFormat)?;
        OffsetDateTime::now_utc()
            .format(&format)
            .change_context(errors::ConnectorError::InvalidDateFormat)
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Worldline
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let url = Self::get_url(self, req, connectors)?;
        let endpoint = url.replace(base_url, "");
        let http_method = Self::get_http_method(self);
        let auth = worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let date = Self::get_current_date_time()?;
        let content_type = Self::get_content_type(self);
        let signed_data: String =
            self.generate_authorization_token(auth, http_method, content_type, &date, &endpoint)?;

        Ok(vec![
            (headers::DATE.to_string(), date.into()),
            (
                headers::AUTHORIZATION.to_string(),
                signed_data.into_masked(),
            ),
            (
                headers::CONTENT_TYPE.to_string(),
                content_type.to_string().into(),
            ),
        ])
    }
}

impl ConnectorCommon for Worldline {
    fn id(&self) -> &'static str {
        "worldline"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.worldline.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: worldline::ErrorResponse = res
            .response
            .parse_struct("Worldline ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        logger::info!(connector_response=?response);

        let error = response.errors.into_iter().next().unwrap_or_default();
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: error
                .code
                .unwrap_or_else(|| hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: error
                .message
                .unwrap_or_else(|| hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            ..Default::default()
        })
    }
}

impl ConnectorValidation for Worldline {}

impl api::ConnectorAccessToken for Worldline {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Worldline {}

impl api::Payment for Worldline {}

impl api::MandateSetup for Worldline {}
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Worldline
{
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Worldline".to_string())
                .into(),
        )
    }
}

impl api::PaymentToken for Worldline {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Worldline
{
    // Not Implemented (R)
}

impl api::PaymentVoid for Worldline {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Worldline {
    fn get_headers(
        &self,
        req: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let auth: worldline::WorldlineAuthType =
            worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        let payment_id = &req.request.connector_transaction_id;
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}/cancel",
        ))
    }

    fn build_request(
        &self,
        req: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(PaymentsVoidType::get_http_method(self))
                .url(&PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: worldline::PaymentResponse = res
            .response
            .parse_struct("Worldline PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        logger::info!(connector_response=?response);

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

impl PaymentSync for Worldline {}
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Worldline {
    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn get_content_type(&self) -> &'static str {
        ""
    }

    fn get_headers(
        &self,
        req: &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        let base_url = self.base_url(connectors);
        let auth = worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}"
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(PaymentsSyncType::get_http_method(self))
                .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
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
        logger::debug!(payment_sync_response=?res);
        let mut response: worldline::Payment = res
            .response
            .parse_struct("Worldline Payment")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        response.capture_method = data.request.capture_method.unwrap_or_default();

        event_builder.map(|i| i.set_response_body(&response));
        logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl PaymentCapture for Worldline {}
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Worldline {
    fn get_headers(
        &self,
        req: &RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_id = req.request.connector_transaction_id.clone();
        let base_url = self.base_url(connectors);
        let auth = worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}/approve"
        ))
    }

    fn get_request_body(
        &self,
        req: &RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = worldline::ApproveRequest::try_from(req)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(PaymentsCaptureType::get_http_method(self))
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
        data: &RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
        errors::ConnectorError,
    >
    where
        Capture: Clone,
        PaymentsCaptureData: Clone,
        PaymentsResponseData: Clone,
    {
        logger::debug!(payment_capture_response=?res);
        let mut response: worldline::PaymentResponse = res
            .response
            .parse_struct("Worldline PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        response.payment.capture_method = enums::CaptureMethod::Manual;

        event_builder.map(|i| i.set_response_body(&response));
        logger::info!(connector_response=?response);

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

impl api::PaymentSession for Worldline {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Worldline {
    // Not Implemented
}

impl api::PaymentAuthorize for Worldline {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Worldline {
    fn get_headers(
        &self,
        req: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let auth = worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        Ok(format!("{base_url}v1/{merchant_account_id}/payments"))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = worldline::WorldlineRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = worldline::PaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(PaymentsAuthorizeType::get_http_method(self))
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
        logger::debug!(payment_authorize_response=?res);
        let mut response: worldline::PaymentResponse = res
            .response
            .parse_struct("Worldline PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        response.payment.capture_method = data.request.capture_method.unwrap_or_default();
        event_builder.map(|i| i.set_response_body(&response));
        logger::info!(connector_response=?response);
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

impl api::Refund for Worldline {}
impl api::RefundExecute for Worldline {}
impl api::RefundSync for Worldline {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Worldline {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_id = req.request.connector_transaction_id.clone();
        let base_url = self.base_url(connectors);
        let auth = worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/payments/{payment_id}/refund"
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = worldline::WorldlineRefundRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(RefundExecuteType::get_http_method(self))
            .url(&RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(RefundExecuteType::get_headers(self, req, connectors)?)
            .set_body(RefundExecuteType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        logger::debug!(target: "router::connector::worldline", response=?res);
        let response: worldline::RefundResponse = res
            .response
            .parse_struct("Worldline RefundResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        logger::info!(connector_response=?response);
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Worldline {
    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn get_content_type(&self) -> &'static str {
        ""
    }

    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        let base_url = self.base_url(connectors);
        let auth: worldline::WorldlineAuthType =
            worldline::WorldlineAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account_id = auth.merchant_account_id.expose();
        Ok(format!(
            "{base_url}v1/{merchant_account_id}/refunds/{refund_id}/"
        ))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(RefundSyncType::get_http_method(self))
                .url(&RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        logger::debug!(target: "router::connector::worldline", response=?res);
        let response: worldline::RefundResponse = res
            .response
            .parse_struct("Worldline RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        logger::info!(connector_response=?response);
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

fn is_endpoint_verification(headers: &actix_web::http::header::HeaderMap) -> bool {
    headers
        .get("x-gcs-webhooks-endpoint-verification")
        .is_some()
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Worldline {
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
        let header_value = utils::get_header_key_value("X-GCS-Signature", request.headers)?;
        let signature = consts::BASE64_ENGINE
            .decode(header_value.as_bytes())
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(signature)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(request.body.to_vec())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        || -> _ {
            Ok::<_, error_stack::Report<common_utils::errors::ParsingError>>(
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        request
                            .body
                            .parse_struct::<worldline::WebhookBody>("WorldlineWebhookEvent")?
                            .payment
                            .parse_value::<worldline::Payment>("WorldlineWebhookObjectId")?
                            .id,
                    ),
                ),
            )
        }()
        .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        if is_endpoint_verification(request.headers) {
            Ok(api_models::webhooks::IncomingWebhookEvent::EndpointVerification)
        } else {
            let details: worldline::WebhookBody = request
                .body
                .parse_struct("WorldlineWebhookObjectId")
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
            let event = match details.event_type {
                worldline::WebhookEvent::Paid => {
                    api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
                }
                worldline::WebhookEvent::Rejected | worldline::WebhookEvent::RejectedCapture => {
                    api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure
                }
                worldline::WebhookEvent::Unknown => {
                    api_models::webhooks::IncomingWebhookEvent::EventNotSupported
                }
            };
            Ok(event)
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let details = request
            .body
            .parse_struct::<worldline::WebhookBody>("WorldlineWebhookObjectId")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?
            .payment
            .ok_or(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        // Ideally this should be a strict type that has type information
        // PII information is likely being logged here when this response will be logged
        Ok(Box::new(details))
    }

    fn get_webhook_api_response(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _error_kind: Option<IncomingWebhookFlowError>,
    ) -> CustomResult<
        hyperswitch_domain_models::api::ApplicationResponse<serde_json::Value>,
        errors::ConnectorError,
    > {
        let verification_header = request.headers.get("x-gcs-webhooks-endpoint-verification");
        let response = match verification_header {
            None => hyperswitch_domain_models::api::ApplicationResponse::StatusOk,
            Some(header_value) => {
                let verification_signature_value = header_value
                    .to_str()
                    .change_context(errors::ConnectorError::WebhookResponseEncodingFailed)?
                    .to_string();
                hyperswitch_domain_models::api::ApplicationResponse::TextPlain(
                    verification_signature_value,
                )
            }
        };
        Ok(response)
    }
}

static WORLDLINE_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::Manual,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let supported_card_network = vec![
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::Discover,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::Visa,
        ];

        let mut worldline_supported_payment_methods = SupportedPaymentMethods::new();

        worldline_supported_payment_methods.add(
            enums::PaymentMethod::BankRedirect,
            enums::PaymentMethodType::Giropay,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        worldline_supported_payment_methods.add(
            enums::PaymentMethod::BankRedirect,
            enums::PaymentMethodType::Ideal,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        worldline_supported_payment_methods.add(
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

        worldline_supported_payment_methods.add(
            enums::PaymentMethod::Card,
            enums::PaymentMethodType::Debit,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods,
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

        worldline_supported_payment_methods
    });

static WORLDLINE_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Worldline",
    description: "Worldpay is an industry leading payments technology and solutions company with unique capabilities to power omni-commerce across the globe.r",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Sandbox,
};

static WORLDLINE_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] = [enums::EventClass::Payments];

impl ConnectorSpecifications for Worldline {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&WORLDLINE_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*WORLDLINE_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&WORLDLINE_SUPPORTED_WEBHOOK_FLOWS)
    }
}
