pub mod transformers;

use std::sync::LazyLock;

use common_enums::enums;
use common_utils::{crypto, errors::CustomResult, ext_traits::ByteSliceExt, request::Request};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, RouterData},
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
        PaymentsSessionRouterData, PaymentsSyncRouterData, RefreshTokenRouterData,
        RefundExecuteRouterData, RefundSyncRouterData, SetupMandateRouterData,
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
    types::Response,
    webhooks,
};
use hyperswitch_masking::{Mask, PeekInterface};
use transformers as sanlam;

use crate::{constants::headers, types::ResponseRouterData, utils::get_header_key_value};

#[derive(Clone)]
pub struct Sanlam {}

impl Sanlam {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for Sanlam {}
impl api::PaymentSession for Sanlam {}
impl api::ConnectorAccessToken for Sanlam {}
impl api::MandateSetup for Sanlam {}
impl api::PaymentAuthorize for Sanlam {}
impl api::PaymentSync for Sanlam {}
impl api::PaymentCapture for Sanlam {}
impl api::PaymentVoid for Sanlam {}
impl api::Refund for Sanlam {}
impl api::RefundExecute for Sanlam {}
impl api::RefundSync for Sanlam {}
impl api::PaymentToken for Sanlam {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Sanlam
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Sanlam {
    fn id(&self) -> &'static str {
        "sanlam"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.sanlam.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = sanlam::SanlamAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![
            (
                headers::AUTHORIZATION.to_string(),
                auth.api_key.peek().to_owned().into_masked(),
            ),
            (
                headers::MERCHANT_ID.to_string(),
                auth.merchant_id.peek().to_owned().into(),
            ),
        ])
    }
}

impl ConnectorValidation for Sanlam {
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
        _data: &PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        Ok(())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Sanlam {
    fn build_request(
        &self,
        _req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Session".to_string(),
            connector: "Sanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Sanlam
{
    fn build_request(
        &self,
        _req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PaymentMethodToken".to_string(),
            connector: "Sanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Sanlam {
    fn build_request(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "AccessTokenAuth".to_string(),
            connector: "Sanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Sanlam {
    fn build_request(
        &self,
        _req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "SetupMandate".to_string(),
            connector: "Sanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Sanlam {
    fn build_request(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Authorize".to_string(),
            connector: "Sanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Sanlam {
    fn build_request(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PSync".to_string(),
            connector: "Sanlam".to_string(),
        }
        .into())
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: sanlam::SanlamWebhookEvent = res
            .response
            .parse_struct("SanlamWebhookEvent")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Sanlam {
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "Sanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Sanlam {
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Void".to_string(),
            connector: "Sanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Sanlam {
    fn build_request(
        &self,
        _req: &RefundExecuteRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Execute".to_string(),
            connector: "Sanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Sanlam {
    fn build_request(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "RSync".to_string(),
            connector: "Sanlam".to_string(),
        }
        .into())
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Sanlam {
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
        let signature = get_header_key_value("X-Signature", request.headers)?;
        hex::decode(signature)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let message = std::str::from_utf8(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(message.to_string().into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: sanlam::SanlamWebhookEvent =
            request
                .body
                .parse_struct("SanlamWebhookEvent")
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        let id = match details {
            sanlam::SanlamWebhookEvent::Payment(ref event) => {
                api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::PaymentAttemptId(
                        event.payment.user_reference.clone(),
                    ),
                )
            }
        };

        Ok(id)
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let details: sanlam::SanlamWebhookEvent =
            request
                .body
                .parse_struct("SanlamWebhookEvent")
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        let event_type = match details {
            sanlam::SanlamWebhookEvent::Payment(ref event) => match event.event_type {
                sanlam::SanlamWebhookEventType::PaymentSucceeded => {
                    api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
                }
                sanlam::SanlamWebhookEventType::PaymentFailed => {
                    api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure
                }
                sanlam::SanlamWebhookEventType::DisputeOpened => {
                    api_models::webhooks::IncomingWebhookEvent::DisputeOpened
                }
            },
        };

        Ok(event_type)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        let details: sanlam::SanlamWebhookEvent =
            request
                .body
                .parse_struct("SanlamWebhookEvent")
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(Box::new(details))
    }
}

static SANLAM_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![enums::CaptureMethod::Automatic];

    let mut sanlam_supported_payment_methods = SupportedPaymentMethods::new();
    sanlam_supported_payment_methods.add(
        enums::PaymentMethod::BankDebit,
        enums::PaymentMethodType::EftDebitOrder,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    sanlam_supported_payment_methods
});

static SANLAM_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Sanlam",
    description: "Sanlam connector",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static SANLAM_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Sanlam {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&SANLAM_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*SANLAM_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&SANLAM_SUPPORTED_WEBHOOK_FLOWS)
    }
}
