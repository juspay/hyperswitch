pub mod transformers;
use std::sync::LazyLock;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::{report, ResultExt};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentIntent;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    payment_method_data::PaymentMethodData,
    payments::payment_attempt::PaymentAttempt,
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
        ConnectorTransactionId, ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        PaymentsAuthorizeType, PaymentsCaptureType, PaymentsSyncType, PaymentsVoidType,
        RefundExecuteType, RefundSyncType, Response,
    },
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::Mask;
#[cfg(feature = "v2")]
use masking::PeekInterface;
use transformers as nexinets;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{
        is_mandate_supported, to_connector_meta, PaymentMethodDataType, PaymentsSyncRequestData,
    },
};

#[derive(Debug, Clone)]
pub struct Nexinets;

impl api::Payment for Nexinets {}
impl api::PaymentSession for Nexinets {}
impl api::ConnectorAccessToken for Nexinets {}
impl api::MandateSetup for Nexinets {}
impl api::PaymentAuthorize for Nexinets {}
impl api::PaymentSync for Nexinets {}
impl api::PaymentCapture for Nexinets {}
impl api::PaymentVoid for Nexinets {}
impl api::Refund for Nexinets {}
impl api::RefundExecute for Nexinets {}
impl api::RefundSync for Nexinets {}

impl Nexinets {
    pub fn connector_transaction_id(
        &self,
        connector_meta: Option<&serde_json::Value>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let meta: nexinets::NexinetsPaymentsMetadata = to_connector_meta(connector_meta.cloned())?;
        Ok(meta.transaction_id)
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Nexinets
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

impl ConnectorCommon for Nexinets {
    fn id(&self) -> &'static str {
        "nexinets"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.nexinets.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = nexinets::NexinetsAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: nexinets::NexinetsErrorResponse = res
            .response
            .parse_struct("NexinetsErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let errors = response.errors;
        let mut message = String::new();
        let mut static_message = String::new();
        for error in errors.iter() {
            let field = error.field.to_owned().unwrap_or_default();
            let mut msg = String::new();
            if !field.is_empty() {
                msg.push_str(format!("{} : {}", field, error.message).as_str());
            } else {
                error.message.clone_into(&mut msg)
            }
            if message.is_empty() {
                message.push_str(&msg);
                static_message.push_str(&msg);
            } else {
                message.push_str(format!(", {msg}").as_str());
            }
        }
        let connector_reason = format!("reason : {} , message : {}", response.message, message);

        Ok(ErrorResponse {
            status_code: response.status,
            code: response.code.to_string(),
            message: static_message,
            reason: Some(connector_reason),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Nexinets {
    fn validate_mandate_payment(
        &self,
        pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::Card,
            PaymentMethodDataType::PaypalRedirect,
            PaymentMethodDataType::ApplePay,
            PaymentMethodDataType::Eps,
            PaymentMethodDataType::Giropay,
            PaymentMethodDataType::Ideal,
        ]);
        is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Nexinets {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Nexinets {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Nexinets
{
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Nexinets".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Nexinets {
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
        let url = if matches!(
            req.request.capture_method,
            Some(enums::CaptureMethod::Automatic) | Some(enums::CaptureMethod::SequentialAutomatic)
        ) {
            format!("{}/orders/debit", self.base_url(connectors))
        } else {
            format!("{}/orders/preauth", self.base_url(connectors))
        };
        Ok(url)
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nexinets::NexinetsPaymentsRequest::try_from(req)?;
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
        let response: nexinets::NexinetsPreAuthOrDebitResponse = res
            .response
            .parse_struct("Nexinets PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Nexinets {
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
        let meta: nexinets::NexinetsPaymentsMetadata =
            to_connector_meta(req.request.connector_meta.clone())?;
        let order_id = nexinets::get_order_id(&meta)?;
        let transaction_id = match meta.psync_flow {
            transformers::NexinetsTransactionType::Debit
            | transformers::NexinetsTransactionType::Capture => {
                req.request.get_connector_transaction_id()?
            }
            _ => nexinets::get_transaction_id(&meta)?,
        };
        Ok(format!(
            "{}/orders/{order_id}/transactions/{transaction_id}",
            self.base_url(connectors)
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
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: nexinets::NexinetsPaymentResponse = res
            .response
            .parse_struct("nexinets NexinetsPaymentResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Nexinets {
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
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let meta: nexinets::NexinetsPaymentsMetadata =
            to_connector_meta(req.request.connector_meta.clone())?;
        let order_id = nexinets::get_order_id(&meta)?;
        let transaction_id = nexinets::get_transaction_id(&meta)?;
        Ok(format!(
            "{}/orders/{order_id}/transactions/{transaction_id}/capture",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nexinets::NexinetsCaptureOrVoidRequest::try_from(req)?;
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
        let response: nexinets::NexinetsPaymentResponse = res
            .response
            .parse_struct("NexinetsPaymentResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Nexinets {
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
        let meta: nexinets::NexinetsPaymentsMetadata =
            to_connector_meta(req.request.connector_meta.clone())?;
        let order_id = nexinets::get_order_id(&meta)?;
        let transaction_id = nexinets::get_transaction_id(&meta)?;
        Ok(format!(
            "{}/orders/{order_id}/transactions/{transaction_id}/cancel",
            self.base_url(connectors),
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nexinets::NexinetsCaptureOrVoidRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PaymentsVoidType::get_url(self, req, connectors)?)
            .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
            .set_body(PaymentsVoidType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: nexinets::NexinetsPaymentResponse = res
            .response
            .parse_struct("NexinetsPaymentResponse")
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Nexinets {
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
        let meta: nexinets::NexinetsPaymentsMetadata =
            to_connector_meta(req.request.connector_metadata.clone())?;
        let order_id = nexinets::get_order_id(&meta)?;
        Ok(format!(
            "{}/orders/{order_id}/transactions/{}/refund",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nexinets::NexinetsRefundRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
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
        let response: nexinets::NexinetsRefundResponse = res
            .response
            .parse_struct("nexinets RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Nexinets {
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
        let transaction_id = req
            .request
            .connector_refund_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorRefundID)?;
        let meta: nexinets::NexinetsPaymentsMetadata =
            to_connector_meta(req.request.connector_metadata.clone())?;
        let order_id = nexinets::get_order_id(&meta)?;
        Ok(format!(
            "{}/orders/{order_id}/transactions/{transaction_id}",
            self.base_url(connectors)
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
        let response: nexinets::NexinetsRefundResponse = res
            .response
            .parse_struct("nexinets RefundSyncResponse")
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
impl IncomingWebhook for Nexinets {
    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        Ok(IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

impl api::PaymentToken for Nexinets {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Nexinets
{
    // Not Implemented (R)
}

static NEXINETS_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::Manual,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let supported_card_network = vec![
            common_enums::CardNetwork::Visa,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::DinersClub,
            common_enums::CardNetwork::Discover,
            common_enums::CardNetwork::Interac,
            common_enums::CardNetwork::JCB,
            common_enums::CardNetwork::CartesBancaires,
            common_enums::CardNetwork::UnionPay,
        ];

        let mut nexinets_supported_payment_methods = SupportedPaymentMethods::new();

        nexinets_supported_payment_methods.add(
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
                            supported_card_networks: supported_card_network.clone(),
                        }
                    }),
                ),
            },
        );
        nexinets_supported_payment_methods.add(
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
                            supported_card_networks: supported_card_network.clone(),
                        }
                    }),
                ),
            },
        );

        nexinets_supported_payment_methods.add(
            enums::PaymentMethod::BankRedirect,
            enums::PaymentMethodType::Ideal,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        nexinets_supported_payment_methods.add(
            enums::PaymentMethod::BankRedirect,
            enums::PaymentMethodType::Giropay,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );
        nexinets_supported_payment_methods.add(
            enums::PaymentMethod::BankRedirect,
            enums::PaymentMethodType::Sofort,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );
        nexinets_supported_payment_methods.add(
            enums::PaymentMethod::BankRedirect,
            enums::PaymentMethodType::Eps,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );
        nexinets_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::ApplePay,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );
        nexinets_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::Paypal,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        nexinets_supported_payment_methods
    });

static NEXINETS_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        display_name: "Nexinets",
        description: "Nexi and Nets join forces to create The European PayTech leader, a strategic combination to offer future-proof innovative payment solutions.",
        connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
        integration_status: enums::ConnectorIntegrationStatus::Sandbox,
    };

static NEXINETS_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Nexinets {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&NEXINETS_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*NEXINETS_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&NEXINETS_SUPPORTED_WEBHOOK_FLOWS)
    }

    #[cfg(feature = "v2")]
    fn generate_connector_request_reference_id(
        &self,
        payment_intent: &PaymentIntent,
        _payment_attempt: &PaymentAttempt,
    ) -> String {
        // The length of merchantOrderId for Nexinets should not exceed 30 characters.
        payment_intent
            .merchant_reference_id
            .as_ref()
            .map(|id| id.get_string_repr().to_owned())
            .unwrap_or_else(|| {
                let max_payment_reference_id_length =
                    nexinets::nexinets_constants::MAX_PAYMENT_REFERENCE_ID_LENGTH;
                nanoid::nanoid!(max_payment_reference_id_length)
            })
    }
}

impl ConnectorTransactionId for Nexinets {
    #[cfg(feature = "v1")]
    fn connector_transaction_id(
        &self,
        payment_attempt: &PaymentAttempt,
    ) -> Result<Option<String>, ApiErrorResponse> {
        let metadata =
            Self::connector_transaction_id(self, payment_attempt.connector_metadata.as_ref());
        metadata.map_err(|_| ApiErrorResponse::ResourceIdNotFound)
    }

    #[cfg(feature = "v2")]
    fn connector_transaction_id(
        &self,
        payment_attempt: &PaymentAttempt,
    ) -> Result<Option<String>, ApiErrorResponse> {
        use hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse;

        let metadata = Self::connector_transaction_id(
            self,
            payment_attempt
                .connector_metadata
                .as_ref()
                .map(|connector_metadata| connector_metadata.peek()),
        );
        metadata.map_err(|_| ApiErrorResponse::ResourceIdNotFound)
    }
}
