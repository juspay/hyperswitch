pub mod transformers;

use std::sync::LazyLock;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        fraud_check::{Checkout, Fulfillment, RecordReturn, Sale, Transaction},
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        fraud_check::{
            FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
            FraudCheckSaleData, FraudCheckTransactionData,
        },
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        fraud_check::FraudCheckResponseData, ConnectorInfo, PaymentsResponseData,
        RefundsResponseData, SupportedPaymentMethods,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation, FraudCheck, FraudCheckCheckout, FraudCheckSale, FraudCheckTransaction,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
#[cfg(feature = "frm")]
use hyperswitch_interfaces::{
    api::{FraudCheckFulfillment, FraudCheckRecordReturn},
    errors::ConnectorError,
};
#[cfg(feature = "frm")]
use masking::Maskable;
use masking::{ExposeInterface, Mask};
use transformers as sift;

use crate::{
    constants::headers,
    types::{
        FrmCheckoutRouterData, FrmCheckoutType, FrmSaleRouterData, FrmSaleType,
        FrmTransactionRouterData, FrmTransactionType, ResponseRouterData,
    },
    utils,
};

#[derive(Clone)]
pub struct Sift {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Sift {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

impl api::Payment for Sift {}
impl api::PaymentSession for Sift {}
impl api::ConnectorAccessToken for Sift {}
impl api::MandateSetup for Sift {}
impl api::PaymentAuthorize for Sift {}
impl api::PaymentSync for Sift {}
impl api::PaymentCapture for Sift {}
impl api::PaymentVoid for Sift {}
impl api::Refund for Sift {}
impl api::RefundExecute for Sift {}
impl api::RefundSync for Sift {}
impl api::PaymentToken for Sift {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Sift
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Sift
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

impl ConnectorCommon for Sift {
    fn id(&self) -> &'static str {
        "sift"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
        //    TODO! Check connector documentation, on which unit they are processing the currency.
        //    If the connector accepts amount in lower unit ( i.e cents for USD) then return api::CurrencyUnit::Minor,
        //    if connector accepts amount in base unit (i.e dollars for USD) then return api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.sift.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = sift::SiftAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: sift::SiftErrorResponse = res
            .response
            .parse_struct("SiftErrorResponse")
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
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
        })
    }
}

impl ConnectorValidation for Sift {
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

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Sift {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Sift {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Sift {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Sift {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Sift {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Sift {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Sift {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Sift {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Sift {}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Sift {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

static SIFT_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(SupportedPaymentMethods::new);

static SIFT_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Sift",
    description: "Sift connector",
    connector_type: enums::PaymentConnectorCategory::PaymentGateway,
};

static SIFT_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Sift {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&SIFT_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*SIFT_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&SIFT_SUPPORTED_WEBHOOK_FLOWS)
    }
}

#[cfg(feature = "frm")]
impl FraudCheck for Sift {}
#[cfg(feature = "frm")]
impl FraudCheckSale for Sift {}
#[cfg(feature = "frm")]
impl FraudCheckCheckout for Sift {}
#[cfg(feature = "frm")]
impl FraudCheckTransaction for Sift {}
#[cfg(feature = "frm")]
impl FraudCheckFulfillment for Sift {}
#[cfg(feature = "frm")]
impl FraudCheckRecordReturn for Sift {}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData> for Sift {
    fn get_headers(
        &self,
        req: &FrmCheckoutRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &FrmCheckoutRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let user_id =
            req.request
                .customer_id
                .clone()
                .ok_or(ConnectorError::MissingRequiredField {
                    field_name: "customer_id",
                })?;

        let auth = sift::SiftAuthType::try_from(&req.connector_auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        let api_key = auth.api_key.expose();

        Ok(format!(
            "{}/v205/users/{}/score?api_key={}&abuse_types=payment_abuse&fields=score_percentiles",
            self.base_url(connectors),
            user_id.get_string_repr(),
            api_key
        ))
    }

    fn build_request(
        &self,
        req: &FrmCheckoutRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        println!("$$$ build_request Checkout sift ");

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&FrmCheckoutType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(FrmCheckoutType::get_headers(self, req, connectors)?)
                // .set_body(FrmCheckoutType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &FrmCheckoutRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FrmCheckoutRouterData, ConnectorError> {
        println!("$$$ res checkout {:?}", res.response);
        let response: sift::SiftFraudCheckResponse = res
            .response
            .parse_struct("SiftPaymentsResponse Checkout")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        <FrmCheckoutRouterData>::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        println!("$$$ error checkout {:?}", res.response);

        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData> for Sift {
    fn get_headers(
        &self,
        req: &FrmTransactionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &FrmTransactionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let user_id =
            req.request
                .customer_id
                .clone()
                .ok_or(ConnectorError::MissingRequiredField {
                    field_name: "customer_id",
                })?;

        let auth = sift::SiftAuthType::try_from(&req.connector_auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        let api_key = auth.api_key.expose();

        Ok(format!(
            "{}/v205/users/{}/score?api_key={}&abuse_types=payment_abuse&fields=score_percentiles",
            self.base_url(connectors),
            user_id.get_string_repr(),
            api_key
        ))
    }

    fn build_request(
        &self,
        req: &FrmTransactionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&FrmTransactionType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(FrmTransactionType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &FrmTransactionRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FrmTransactionRouterData, ConnectorError> {
        let response: sift::SiftFraudCheckResponse = res
            .response
            .parse_struct("SiftPaymentsResponse Transaction")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        <FrmTransactionRouterData>::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Sale, FraudCheckSaleData, FraudCheckResponseData> for Sift {
    // Not implemented
}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData> for Sift {
    // Not implemented
}

#[cfg(feature = "frm")]
impl ConnectorIntegration<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
    for Sift
{
    // Not implemented
}
