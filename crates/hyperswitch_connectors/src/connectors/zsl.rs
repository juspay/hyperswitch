pub mod transformers;

use std::fmt::Debug;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::{BytesExt, ValueExt},
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse,
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
        PaymentsSessionRouterData, PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
        TokenizationRouterData,
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
    webhooks::{IncomingWebhook, IncomingWebhookFlowError, IncomingWebhookRequestDetails},
};
use lazy_static::lazy_static;
use masking::{ExposeInterface, Secret};
use transformers::{self as zsl, get_status};

use crate::{
    constants::headers,
    types::{RefreshTokenRouterData, ResponseRouterData},
};

#[derive(Debug, Clone)]
pub struct Zsl;

impl api::Payment for Zsl {}
impl api::PaymentSession for Zsl {}
impl api::ConnectorAccessToken for Zsl {}
impl api::MandateSetup for Zsl {}
impl api::PaymentAuthorize for Zsl {}
impl api::PaymentSync for Zsl {}
impl api::PaymentCapture for Zsl {}
impl api::PaymentVoid for Zsl {}
impl api::Refund for Zsl {}
impl api::RefundExecute for Zsl {}
impl api::RefundSync for Zsl {}
impl api::PaymentToken for Zsl {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Zsl
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        Ok(header)
    }
}

impl ConnectorCommon for Zsl {
    fn id(&self) -> &'static str {
        "zsl"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.zsl.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response = serde_urlencoded::from_bytes::<zsl::ZslErrorResponse>(&res.response)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error_reason = zsl::ZslResponseStatus::try_from(response.status.clone())?.to_string();

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status,
            message: error_reason.clone(),
            reason: Some(error_reason),
            attempt_status: Some(common_enums::AttemptStatus::Failure),
            connector_transaction_id: None,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

impl ConnectorValidation for Zsl {
    fn is_webhook_source_verification_mandatory(&self) -> bool {
        true
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Zsl {
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
        Ok(format!("{}ecp", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = zsl::ZslRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = zsl::ZslPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
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
        let response = serde_urlencoded::from_bytes::<zsl::ZslPaymentsResponse>(&res.response)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i: &mut ConnectorEvent| i.set_response_body(&response));
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Zsl {
    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: zsl::ZslWebhookResponse = res
            .response
            .parse_struct("ZslWebhookResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i: &mut ConnectorEvent| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Zsl {
    fn build_request(
        &self,
        _req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Session flow".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Zsl
{
    fn build_request(
        &self,
        _req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "PaymentMethod Tokenization flow ".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Zsl {
    fn build_request(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "AccessTokenAuth flow".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Zsl {
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "SetupMandate flow".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Zsl {
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Capture flow".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Zsl {
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Void flow ".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Zsl {
    fn build_request(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Refund flow".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Zsl {
    fn build_request(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Rsync flow ".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

#[async_trait::async_trait]
impl IncomingWebhook for Zsl {
    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::PaymentAttemptId(notif.mer_ref),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(get_status(notif.status))
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let response = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(Box::new(response))
    }

    async fn verify_webhook_source(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        connector_account_details: common_utils::crypto::Encryptable<Secret<serde_json::Value>>,
        _connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_account_details = connector_account_details
            .parse_value::<ConnectorAuthType>("ConnectorAuthType")
            .change_context_lazy(|| errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let auth_type = zsl::ZslAuthType::try_from(&connector_account_details)?;
        let key = auth_type.api_key.expose();
        let mer_id = auth_type.merchant_id.expose();
        let webhook_response = get_webhook_object_from_body(request.body)?;
        let signature = zsl::calculate_signature(
            webhook_response.enctype,
            zsl::ZslSignatureType::WebhookSignature {
                status: webhook_response.status,
                txn_id: webhook_response.txn_id,
                txn_date: webhook_response.txn_date,
                paid_ccy: webhook_response.paid_ccy.to_string(),
                paid_amt: webhook_response.paid_amt,
                mer_ref: webhook_response.mer_ref,
                mer_id,
                key,
            },
        )?;

        Ok(signature.eq(&webhook_response.signature))
    }

    fn get_webhook_api_response(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        _error_kind: Option<IncomingWebhookFlowError>,
    ) -> CustomResult<ApplicationResponse<serde_json::Value>, errors::ConnectorError> {
        Ok(ApplicationResponse::TextPlain("CALLBACK-OK".to_string()))
    }
}

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<zsl::ZslWebhookResponse, errors::ConnectorError> {
    let response: zsl::ZslWebhookResponse =
        serde_urlencoded::from_bytes::<zsl::ZslWebhookResponse>(body)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
    Ok(response)
}

lazy_static! {
    static ref ZSL_SUPPORTED_PAYMENT_METHODS: SupportedPaymentMethods = {
        let supported_capture_methods = vec![enums::CaptureMethod::Automatic];

        let mut zsl_supported_payment_methods = SupportedPaymentMethods::new();
        zsl_supported_payment_methods.add(
            enums::PaymentMethod::BankTransfer,
            enums::PaymentMethodType::LocalBankTransfer,
            PaymentMethodDetails{
                mandates: common_enums::FeatureStatus::NotSupported,
                refunds: common_enums::FeatureStatus::NotSupported,
                supported_capture_methods,
                specific_features: None,
            },
        );

        zsl_supported_payment_methods
    };

    static ref ZSL_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        display_name: "ZSL",
        description:
            "Zsl is a payment gateway operating in China, specializing in facilitating local bank transfers",
        connector_type: enums::PaymentConnectorCategory::PaymentGateway,
    };

    static ref ZSL_SUPPORTED_WEBHOOK_FLOWS: Vec<enums::EventClass> = Vec::new();

}

impl ConnectorSpecifications for Zsl {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&*ZSL_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*ZSL_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&*ZSL_SUPPORTED_WEBHOOK_FLOWS)
    }
}
