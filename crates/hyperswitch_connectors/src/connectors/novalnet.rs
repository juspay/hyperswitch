pub mod transformers;
use core::str;
use std::collections::HashSet;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    crypto,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
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
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorRedirectResponse,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{ExposeInterface, Mask};
use transformers as novalnet;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, PaymentMethodDataType, PaymentsAuthorizeRequestData},
};

#[derive(Clone)]
pub struct Novalnet {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Novalnet {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

impl api::Payment for Novalnet {}
impl api::PaymentSession for Novalnet {}
impl api::ConnectorAccessToken for Novalnet {}
impl api::MandateSetup for Novalnet {}
impl api::PaymentAuthorize for Novalnet {}
impl api::PaymentSync for Novalnet {}
impl api::PaymentCapture for Novalnet {}
impl api::PaymentVoid for Novalnet {}
impl api::Refund for Novalnet {}
impl api::RefundExecute for Novalnet {}
impl api::RefundSync for Novalnet {}
impl api::PaymentToken for Novalnet {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Novalnet
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Novalnet
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

impl ConnectorCommon for Novalnet {
    fn id(&self) -> &'static str {
        "novalnet"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.novalnet.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = novalnet::NovalnetAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let api_key: String = auth.payment_access_key.expose();
        let encoded_api_key = common_utils::consts::BASE64_ENGINE.encode(api_key);
        Ok(vec![(
            headers::X_NN_ACCESS_KEY.to_string(),
            encoded_api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: novalnet::NovalnetErrorResponse = res
            .response
            .parse_struct("NovalnetErrorResponse")
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
        })
    }
}

impl ConnectorValidation for Novalnet {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
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
    fn validate_mandate_payment(
        &self,
        pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd: HashSet<PaymentMethodDataType> =
            HashSet::from([PaymentMethodDataType::Card]);
        utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl ConnectorRedirectResponse for Novalnet {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        action: enums::PaymentAction,
    ) -> CustomResult<enums::CallConnectorAction, errors::ConnectorError> {
        match action {
            enums::PaymentAction::PSync => Ok(enums::CallConnectorAction::Trigger),
            _ => Ok(enums::CallConnectorAction::Avoid),
        }
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Novalnet {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Novalnet {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Novalnet
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Novalnet {
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
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = self.base_url(connectors);
        match req.request.is_auto_capture()? {
            true => Ok(format!("{}/payment", endpoint)),
            false => Ok(format!("{}/authorize", endpoint)),
        }
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

        let connector_router_data = novalnet::NovalnetRouterData::from((amount, req));
        let connector_req = novalnet::NovalnetPaymentsRequest::try_from(&connector_router_data)?;
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
        let response: novalnet::NovalnetPaymentsResponse = res
            .response
            .parse_struct("Novalnet PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Novalnet {
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
        let endpoint = self.base_url(connectors);
        Ok(format!("{}/transaction/details", endpoint))
    }

    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = novalnet::NovalnetSyncRequest::try_from(req)?;
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
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: novalnet::NovalnetPSyncResponse = res
            .response
            .parse_struct("novalnet PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Novalnet {
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
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let endpoint = self.base_url(connectors);
        Ok(format!("{}/transaction/capture", endpoint))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;

        let connector_router_data = novalnet::NovalnetRouterData::from((amount, req));
        let connector_req = novalnet::NovalnetCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
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
        let response: novalnet::NovalnetCaptureResponse = res
            .response
            .parse_struct("Novalnet PaymentsCaptureResponse")
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Novalnet {
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
        let endpoint = self.base_url(connectors);
        Ok(format!("{}/transaction/refund", endpoint))
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

        let connector_router_data = novalnet::NovalnetRouterData::from((refund_amount, req));
        let connector_req = novalnet::NovalnetRefundRequest::try_from(&connector_router_data)?;
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
        let response: novalnet::NovalnetRefundResponse = res
            .response
            .parse_struct("novalnet RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Novalnet {
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
        let endpoint = self.base_url(connectors);
        Ok(format!("{}/transaction/details", endpoint))
    }

    fn get_request_body(
        &self,
        req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = novalnet::NovalnetSyncRequest::try_from(req)?;
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
        let response: novalnet::NovalnetRefundSyncResponse = res
            .response
            .parse_struct("novalnet RefundSyncResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Novalnet {
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
        let endpoint = self.base_url(connectors);
        Ok(format!("{}/transaction/cancel", endpoint))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = novalnet::NovalnetCancelRequest::try_from(req)?;
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
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsVoidType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: novalnet::NovalnetCancelResponse = res
            .response
            .parse_struct("NovalnetPaymentsVoidResponse")
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

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<novalnet::NovalnetWebhookNotificationResponse, errors::ConnectorError> {
    let novalnet_webhook_notification_response = body
        .parse_struct("NovalnetWebhookNotificationResponse")
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

    Ok(novalnet_webhook_notification_response)
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Novalnet {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::Sha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let notif_item = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        hex::decode(notif_item.event.checksum)
            .change_context(errors::ConnectorError::WebhookVerificationSecretInvalid)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let (amount, currency) = match notif.transaction {
            novalnet::NovalnetWebhookTransactionData::CaptureTransactionData(data) => {
                (data.amount, data.currency)
            }
            novalnet::NovalnetWebhookTransactionData::CancelTransactionData(data) => {
                (data.amount, data.currency)
            }

            novalnet::NovalnetWebhookTransactionData::RefundsTransactionData(data) => {
                (data.amount, data.currency)
            }

            novalnet::NovalnetWebhookTransactionData::SyncTransactionData(data) => {
                (data.amount, data.currency)
            }
        };
        let amount = amount
            .map(|amount| amount.to_string())
            .unwrap_or("".to_string());
        let currency = currency
            .map(|amount| amount.to_string())
            .unwrap_or("".to_string());

        let secret_auth = String::from_utf8(connector_webhook_secrets.secret.to_vec())
            .change_context(errors::ConnectorError::WebhookVerificationSecretInvalid)
            .attach_printable("Could not convert webhook secret auth to UTF-8")?;
        let reversed_secret_auth = novalnet::reverse_string(&secret_auth);

        let message = format!(
            "{}{}{}{}{}{}",
            notif.event.tid,
            notif.event.event_type,
            notif.result.status,
            amount,
            currency,
            reversed_secret_auth
        );

        Ok(message.into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        let transaction_order_no = match notif.transaction {
            novalnet::NovalnetWebhookTransactionData::CaptureTransactionData(data) => data.order_no,
            novalnet::NovalnetWebhookTransactionData::CancelTransactionData(data) => data.order_no,
            novalnet::NovalnetWebhookTransactionData::RefundsTransactionData(data) => data.order_no,
            novalnet::NovalnetWebhookTransactionData::SyncTransactionData(data) => data.order_no,
        };

        if novalnet::is_refund_event(&notif.event.event_type) {
            Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::ConnectorRefundId(notif.event.tid.to_string()),
            ))
        } else {
            match transaction_order_no {
                Some(order_no) => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::PaymentAttemptId(order_no),
                )),
                None => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        notif.event.tid.to_string(),
                    ),
                )),
            }
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        let optional_transaction_status = match notif.transaction {
            novalnet::NovalnetWebhookTransactionData::CaptureTransactionData(data) => {
                Some(data.status)
            }
            novalnet::NovalnetWebhookTransactionData::CancelTransactionData(data) => data.status,
            novalnet::NovalnetWebhookTransactionData::RefundsTransactionData(data) => {
                Some(data.status)
            }
            novalnet::NovalnetWebhookTransactionData::SyncTransactionData(data) => {
                Some(data.status)
            }
        };

        let transaction_status =
            optional_transaction_status.ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "transaction_status",
            })?;
        // NOTE: transaction_status will always be present for Webhooks
        // But we are handling optional type here, since we are reusing TransactionData Struct from NovalnetPaymentsResponseTransactionData for Webhooks response too
        // In NovalnetPaymentsResponseTransactionData, transaction_status is optional

        let incoming_webhook_event =
            novalnet::get_incoming_webhook_event(notif.event.event_type, transaction_status);
        Ok(incoming_webhook_event)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(notif))
    }
}
