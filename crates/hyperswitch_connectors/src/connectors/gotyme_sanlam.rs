pub mod transformers;

use std::sync::LazyLock;

use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::ResultExt;
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
        PaymentsSessionRouterData, PaymentsSyncRouterData, RefreshTokenRouterData,
        RefundExecuteRouterData, RefundSyncRouterData, SetupMandateRouterData,
        TokenizationRouterData,
    },
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::payouts::{PoFulfill, PoSync},
    router_request_types::PayoutsData,
    router_response_types::PayoutsResponseData,
    types::PayoutsRouterData,
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::types::{PayoutFulfillType, PayoutSyncType};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks,
};
use hyperswitch_masking::{Mask, PeekInterface};
use transformers as gotyme_sanlam;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct GotymeSanlam {
    #[cfg(feature = "payouts")]
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl GotymeSanlam {
    pub fn new() -> &'static Self {
        &Self {
            #[cfg(feature = "payouts")]
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

impl api::Payment for GotymeSanlam {}
impl api::PaymentSession for GotymeSanlam {}
impl api::ConnectorAccessToken for GotymeSanlam {}
impl api::MandateSetup for GotymeSanlam {}
impl api::PaymentAuthorize for GotymeSanlam {}
impl api::PaymentSync for GotymeSanlam {}
impl api::PaymentCapture for GotymeSanlam {}
impl api::PaymentVoid for GotymeSanlam {}
impl api::Refund for GotymeSanlam {}
impl api::RefundExecute for GotymeSanlam {}
impl api::RefundSync for GotymeSanlam {}
impl api::PaymentToken for GotymeSanlam {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for GotymeSanlam
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

impl ConnectorCommon for GotymeSanlam {
    fn id(&self) -> &'static str {
        "gotyme_sanlam"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.gotyme_sanlam.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = gotyme_sanlam::GotymeSanlamAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![
            (
                headers::X_API_KEY.to_string(),
                auth.api_key.peek().to_owned().into_masked(),
            ),
            (
                headers::PROFILE_ID.to_string(),
                auth.profile_id.peek().to_owned().into(),
            ),
        ])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: gotyme_sanlam::GotymeSanlamErrorResponse = res
            .response
            .parse_struct("GotymeSanlamErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error_code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response
                .error_message
                .or(response.message)
                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: None,
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

#[cfg(feature = "payouts")]
impl api::Payouts for GotymeSanlam {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for GotymeSanlam {}
#[cfg(feature = "payouts")]
impl api::PayoutSync for GotymeSanlam {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for GotymeSanlam {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/meep-tymebank/invoke",
            self.base_url(connectors)
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.source_currency,
        )?;

        let connector_router_data = gotyme_sanlam::GotymeSanlamRouterData::from((amount, req));
        let connector_req =
            gotyme_sanlam::GotymeSanlamPayoutTransferRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutFulfillType::get_headers(self, req, connectors)?)
            .set_body(PayoutFulfillType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, errors::ConnectorError> {
        let response: gotyme_sanlam::GotymeSanlamPayoutResponse = res
            .response
            .parse_struct("GotymeSanlamPayoutResponse")
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoSync, PayoutsData, PayoutsResponseData> for GotymeSanlam {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/meep-tymebank/invoke",
            self.base_url(connectors)
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoSync>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutSyncType::get_headers(self, req, connectors)?)
            .set_body(PayoutSyncType::get_request_body(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoSync>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = gotyme_sanlam::GotymeSanlamPayoutGetRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoSync>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoSync>, errors::ConnectorError> {
        let response: gotyme_sanlam::GotymeSanlamPayoutResponse = res
            .response
            .parse_struct("GotymeSanlamPayoutResponse")
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

impl ConnectorValidation for GotymeSanlam {
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

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for GotymeSanlam {
    fn build_request(
        &self,
        _req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Session".to_string(),
            connector: "GotymeSanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for GotymeSanlam
{
    fn build_request(
        &self,
        _req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PaymentMethodToken".to_string(),
            connector: "GotymeSanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for GotymeSanlam {
    fn build_request(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "AccessTokenAuth".to_string(),
            connector: "GotymeSanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for GotymeSanlam
{
    fn build_request(
        &self,
        _req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "SetupMandate".to_string(),
            connector: "GotymeSanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for GotymeSanlam {
    fn build_request(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Authorize".to_string(),
            connector: "GotymeSanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for GotymeSanlam {
    fn build_request(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PSync".to_string(),
            connector: "GotymeSanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for GotymeSanlam {
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "GotymeSanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for GotymeSanlam {
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Void".to_string(),
            connector: "GotymeSanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for GotymeSanlam {
    fn build_request(
        &self,
        _req: &RefundExecuteRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Execute".to_string(),
            connector: "GotymeSanlam".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for GotymeSanlam {
    fn build_request(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "RSync".to_string(),
            connector: "GotymeSanlam".to_string(),
        }
        .into())
    }
}

impl webhooks::IncomingWebhook for GotymeSanlam {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented.into())
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented.into())
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        Err(errors::ConnectorError::WebhooksNotImplemented.into())
    }
}

static GOTYME_SANLAM_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![enums::CaptureMethod::Automatic];

        let mut gotyme_sanlam_supported_payment_methods = SupportedPaymentMethods::new();
        gotyme_sanlam_supported_payment_methods.add(
            enums::PaymentMethod::BankDebit,
            enums::PaymentMethodType::EftDebitOrder,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported,
                refunds: common_enums::FeatureStatus::NotSupported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        gotyme_sanlam_supported_payment_methods
    });

static GOTYME_SANLAM_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "GotymeSanlam",
    description: "GotymeSanlam connector",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static GOTYME_SANLAM_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for GotymeSanlam {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&GOTYME_SANLAM_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*GOTYME_SANLAM_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&GOTYME_SANLAM_SUPPORTED_WEBHOOK_FLOWS)
    }

    #[cfg(feature = "payouts")]
    /// Generate connector request reference ID for payout flows
    fn generate_payout_connector_request_reference_id(
        &self,
        _payout_attempt: &hyperswitch_domain_models::payouts::payout_attempt::PayoutAttempt,
    ) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
