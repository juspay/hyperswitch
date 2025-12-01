//! API interface

/// authentication module
pub mod authentication;
/// authentication_v2 module
pub mod authentication_v2;
pub mod disputes;
pub mod disputes_v2;
pub mod files;
pub mod files_v2;
#[cfg(feature = "frm")]
pub mod fraud_check;
#[cfg(feature = "frm")]
pub mod fraud_check_v2;
pub mod gateway;
pub mod payments;
pub mod payments_v2;
#[cfg(feature = "payouts")]
pub mod payouts;
#[cfg(feature = "payouts")]
pub mod payouts_v2;
pub mod refunds;
pub mod refunds_v2;
pub mod revenue_recovery;
pub mod revenue_recovery_v2;
pub mod subscriptions;
pub mod subscriptions_v2;
pub mod vault;
pub mod vault_v2;

use std::fmt::Debug;

use common_enums::{
    enums::{
        self, CallConnectorAction, CaptureMethod, EventClass, PaymentAction, PaymentMethodType,
    },
    PaymentMethod,
};
use common_utils::{
    errors::CustomResult,
    request::{Method, Request, RequestContent},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    connector_endpoints::Connectors,
    errors::api_error_response::ApiErrorResponse,
    payment_method_data::PaymentMethodData,
    router_data::{
        AccessToken, AccessTokenAuthenticationResponse, ConnectorAuthType, ErrorResponse,
        RouterData,
    },
    router_data_v2::{
        flow_common_types::{AuthenticationTokenFlowData, WebhookSourceVerifyData},
        AccessTokenFlowData, MandateRevokeFlowData, UasFlowData,
    },
    router_flow_types::{
        mandate_revoke::MandateRevoke, AccessTokenAuth, AccessTokenAuthentication, Authenticate,
        AuthenticationConfirmation, PostAuthenticate, PreAuthenticate, VerifyWebhookSource,
    },
    router_request_types::{
        self,
        unified_authentication_service::{
            UasAuthenticationRequestData, UasAuthenticationResponseData,
            UasConfirmationRequestData, UasPostAuthenticationRequestData,
            UasPreAuthenticationRequestData,
        },
        AccessTokenAuthenticationRequestData, AccessTokenRequestData, MandateRevokeRequestData,
        VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        self, ConnectorInfo, MandateRevokeResponseData, PaymentMethodDetails,
        SupportedPaymentMethods, VerifyWebhookSourceResponseData,
    },
};
use masking::Maskable;
use serde_json::json;

#[cfg(feature = "frm")]
pub use self::fraud_check::*;
#[cfg(feature = "frm")]
pub use self::fraud_check_v2::*;
#[cfg(feature = "payouts")]
pub use self::payouts::*;
#[cfg(feature = "payouts")]
pub use self::payouts_v2::*;
pub use self::{payments::*, refunds::*, vault::*, vault_v2::*};
use crate::{
    api::subscriptions::Subscriptions, connector_integration_v2::ConnectorIntegrationV2, consts,
    errors, events::connector_api_logs::ConnectorEvent, metrics, types, webhooks,
};

/// Connector trait
pub trait Connector:
    Send
    + Refund
    + Payment
    + ConnectorRedirectResponse
    + webhooks::IncomingWebhook
    + ConnectorAccessToken
    + ConnectorAuthenticationToken
    + disputes::Dispute
    + files::FileUpload
    + ConnectorTransactionId
    + Payouts
    + ConnectorVerifyWebhookSource
    + FraudCheck
    + ConnectorMandateRevoke
    + authentication::ExternalAuthentication
    + TaxCalculation
    + UnifiedAuthenticationService
    + revenue_recovery::RevenueRecovery
    + ExternalVault
    + Subscriptions
{
}

impl<
        T: Refund
            + Payment
            + ConnectorRedirectResponse
            + Send
            + webhooks::IncomingWebhook
            + ConnectorAccessToken
            + ConnectorAuthenticationToken
            + disputes::Dispute
            + files::FileUpload
            + ConnectorTransactionId
            + Payouts
            + ConnectorVerifyWebhookSource
            + FraudCheck
            + ConnectorMandateRevoke
            + authentication::ExternalAuthentication
            + TaxCalculation
            + UnifiedAuthenticationService
            + revenue_recovery::RevenueRecovery
            + ExternalVault
            + Subscriptions,
    > Connector for T
{
}

/// Alias for Box<&'static (dyn Connector + Sync)>
pub type BoxedConnector = Box<&'static (dyn Connector + Sync)>;

/// type BoxedConnectorIntegration
pub type BoxedConnectorIntegration<'a, T, Req, Resp> =
    Box<&'a (dyn ConnectorIntegration<T, Req, Resp> + Send + Sync)>;

/// trait ConnectorIntegrationAny
pub trait ConnectorIntegrationAny<T, Req, Resp>: Send + Sync + 'static {
    /// fn get_connector_integration
    fn get_connector_integration(&self) -> BoxedConnectorIntegration<'_, T, Req, Resp>;
}

impl<S, T, Req, Resp> ConnectorIntegrationAny<T, Req, Resp> for S
where
    S: ConnectorIntegration<T, Req, Resp> + Send + Sync,
{
    fn get_connector_integration(&self) -> BoxedConnectorIntegration<'_, T, Req, Resp> {
        Box::new(self)
    }
}

/// trait ConnectorIntegration
pub trait ConnectorIntegration<T, Req, Resp>:
    ConnectorIntegrationAny<T, Req, Resp> + Sync + ConnectorCommon
{
    /// fn get_headers
    fn get_headers(
        &self,
        _req: &RouterData<T, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    /// fn get_content_type
    fn get_content_type(&self) -> &'static str {
        mime::APPLICATION_JSON.essence_str()
    }

    /// fn get_content_type
    fn get_accept_type(&self) -> &'static str {
        mime::APPLICATION_JSON.essence_str()
    }

    /// primarily used when creating signature based on request method of payment flow
    fn get_http_method(&self) -> Method {
        Method::Post
    }

    /// fn get_url
    fn get_url(
        &self,
        _req: &RouterData<T, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(String::new())
    }

    /// fn get_request_body
    fn get_request_body(
        &self,
        _req: &RouterData<T, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Ok(RequestContent::Json(Box::new(json!(r#"{}"#))))
    }

    /// fn get_request_form_data
    fn get_request_form_data(
        &self,
        _req: &RouterData<T, Req, Resp>,
    ) -> CustomResult<Option<reqwest::multipart::Form>, errors::ConnectorError> {
        Ok(None)
    }

    /// fn build_request
    fn build_request(
        &self,
        req: &RouterData<T, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        metrics::UNIMPLEMENTED_FLOW.add(
            1,
            router_env::metric_attributes!(("connector", req.connector.clone())),
        );
        Ok(None)
    }

    /// fn handle_response
    fn handle_response(
        &self,
        data: &RouterData<T, Req, Resp>,
        event_builder: Option<&mut ConnectorEvent>,
        _res: types::Response,
    ) -> CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        T: Clone,
        Req: Clone,
        Resp: Clone,
    {
        event_builder.map(|e| e.set_error(json!({"error": "Not Implemented"})));
        Ok(data.clone())
    }

    /// fn get_error_response
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        event_builder.map(|event| event.set_error(json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
        Ok(ErrorResponse::get_not_implemented())
    }

    /// fn get_5xx_error_response
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        event_builder.map(|event| event.set_error(json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
        let error_message = match res.status_code {
            500 => "internal_server_error",
            501 => "not_implemented",
            502 => "bad_gateway",
            503 => "service_unavailable",
            504 => "gateway_timeout",
            505 => "http_version_not_supported",
            506 => "variant_also_negotiates",
            507 => "insufficient_storage",
            508 => "loop_detected",
            510 => "not_extended",
            511 => "network_authentication_required",
            _ => "unknown_error",
        };
        Ok(ErrorResponse {
            code: res.status_code.to_string(),
            message: error_message.to_string(),
            reason: String::from_utf8(res.response.to_vec()).ok(),
            status_code: res.status_code,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }

    /// whenever capture sync is implemented at the connector side, this method should be overridden
    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<CaptureSyncMethod, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("multiple capture sync".into()).into())
    }

    /// fn get_certificate
    fn get_certificate(
        &self,
        _req: &RouterData<T, Req, Resp>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    /// fn get_certificate_key
    fn get_certificate_key(
        &self,
        _req: &RouterData<T, Req, Resp>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }
}

/// Sync Methods for multiple captures
#[derive(Debug)]
pub enum CaptureSyncMethod {
    /// For syncing multiple captures individually
    Individual,
    /// For syncing multiple captures together
    Bulk,
}

/// Connector accepted currency unit as either "Base" or "Minor"
#[derive(Debug)]
pub enum CurrencyUnit {
    /// Base currency unit
    Base,
    /// Minor currency unit
    Minor,
}

/// The trait that provides the common
pub trait ConnectorCommon {
    /// Name of the connector (in lowercase).
    fn id(&self) -> &'static str;

    /// Connector accepted currency unit as either "Base" or "Minor"
    fn get_currency_unit(&self) -> CurrencyUnit {
        CurrencyUnit::Minor // Default implementation should be remove once it is implemented in all connectors
    }

    /// HTTP header used for authorization.
    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    /// HTTP `Content-Type` to be used for POST requests.
    /// Defaults to `application/json`.
    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    // FIXME write doc - think about this
    // fn headers(&self) -> Vec<(&str, &str)>;

    /// The base URL for interacting with the connector's API.
    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str;

    /// common error response for a connector if it is same in all case
    fn build_error_response(
        &self,
        res: types::Response,
        _event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: consts::NO_ERROR_CODE.to_string(),
            message: consts::NO_ERROR_MESSAGE.to_string(),
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

impl ConnectorAccessTokenSuffix for BoxedConnector {}

/// Current flow information passed to the connector specifications trait
///
/// In order to make some desicion about the preprocessing or alternate flow
#[derive(Clone, Debug)]
pub enum CurrentFlowInfo<'a> {
    /// Authorize flow information
    Authorize {
        /// The authentication type being used
        auth_type: &'a enums::AuthenticationType,
        /// The payment authorize request data
        request_data: &'a router_request_types::PaymentsAuthorizeData,
    },
    /// CompleteAuthorize flow information
    CompleteAuthorize {
        /// The payment authorize request data
        request_data: &'a router_request_types::CompleteAuthorizeData,
    },
}

/// Alternate API flow that must be made instead of the current flow.
/// For example, PreAuthenticate flow must be made instead of Authorize flow.
#[derive(Debug, Clone, Copy)]
pub enum AlternateFlow {
    /// Pre-authentication flow
    PreAuthenticate,
}

/// The Preprocessing flow that must be made before the current flow.
///
/// For example, PreProcessing flow must be made before Authorize flow.
/// Or PostAuthenticate flow must be made before CompleteAuthorize flow for cybersource.
#[derive(Debug, Clone, Copy)]
pub enum PreProcessingFlowName {
    /// Authentication flow must be made before the actual flow
    Authenticate,
    /// Post-authentication flow must be made before the actual flow
    PostAuthenticate,
}

/// Response of the preprocessing flow
#[derive(Debug)]
pub struct PreProcessingFlowResponse<'a> {
    /// Payment response data from the preprocessing flow
    pub response: &'a Result<router_response_types::PaymentsResponseData, ErrorResponse>,
    /// Attempt status after the preprocessing flow
    pub attempt_status: enums::AttemptStatus,
}

/// The trait that provides specifications about the connector
pub trait ConnectorSpecifications {
    /// Preprocessing flow name if any, that must be made before the current flow.
    fn get_preprocessing_flow_if_needed(
        &self,
        _current_flow: CurrentFlowInfo<'_>,
    ) -> Option<PreProcessingFlowName> {
        None
    }
    /// Based on the current flow and preprocessing_flow_response, decide if the main flow must be called or not
    ///
    /// By default, always continue with the main flow after the preprocessing flow.
    fn decide_should_continue_after_preprocessing(
        &self,
        _current_flow: CurrentFlowInfo<'_>,
        _pre_processing_flow_name: PreProcessingFlowName,
        _preprocessing_flow_response: PreProcessingFlowResponse<'_>,
    ) -> bool {
        true
    }
    /// If Some is returned, the returned api flow must be made instead of the current flow.
    fn get_alternate_flow_if_needed(
        &self,
        _current_flow: CurrentFlowInfo<'_>,
    ) -> Option<AlternateFlow> {
        None
    }
    /// Details related to payment method supported by the connector
    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

    /// Supported webhooks flows
    fn get_supported_webhook_flows(&self) -> Option<&'static [EventClass]> {
        None
    }

    /// About the connector
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        None
    }

    /// Check if connector should make another request to create an access token
    /// Connectors should override this method if they require an authentication token to create a new access token
    fn authentication_token_for_token_creation(&self) -> bool {
        false
    }

    /// Check if connector should make another request to create an customer
    /// Connectors should override this method if they require to create a connector customer
    fn should_call_connector_customer(
        &self,
        _payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> bool {
        false
    }

    /// Whether SDK session token generation is enabled for this connector
    fn is_sdk_client_token_generation_enabled(&self) -> bool {
        false
    }

    /// Payment method types that support SDK session token generation
    fn supported_payment_method_types_for_sdk_client_token_generation(
        &self,
    ) -> Vec<PaymentMethodType> {
        vec![]
    }

    /// Validate if SDK session token generation is allowed for given payment method type
    fn validate_sdk_session_token_for_payment_method(
        &self,
        current_core_payment_method_type: &PaymentMethodType,
    ) -> bool {
        self.is_sdk_client_token_generation_enabled()
            && self
                .supported_payment_method_types_for_sdk_client_token_generation()
                .contains(current_core_payment_method_type)
    }

    #[cfg(not(feature = "v2"))]
    /// Generate connector request reference ID
    fn generate_connector_request_reference_id(
        &self,
        _payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
        is_config_enabled_to_send_payment_id_as_connector_request_id: bool,
    ) -> String {
        // Send payment_id if config is enabled for a merchant, else send attempt_id
        if is_config_enabled_to_send_payment_id_as_connector_request_id {
            payment_attempt.payment_id.get_string_repr().to_owned()
        } else {
            payment_attempt.attempt_id.to_owned()
        }
    }

    #[cfg(feature = "v2")]
    /// Generate connector request reference ID
    fn generate_connector_request_reference_id(
        &self,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> String {
        payment_intent
            .merchant_reference_id
            .as_ref()
            .map(|id| id.get_string_repr().to_owned())
            .unwrap_or_else(|| payment_attempt.id.get_string_repr().to_owned())
    }

    #[cfg(feature = "v1")]
    /// Generate connector customer reference ID for payments
    fn generate_connector_customer_id(
        &self,
        _customer_id: &Option<common_utils::id_type::CustomerId>,
        _merchant_id: &common_utils::id_type::MerchantId,
    ) -> Option<String> {
        None
    }

    #[cfg(feature = "v2")]
    /// Generate connector customer reference ID for payments
    fn generate_connector_customer_id(
        &self,
        _customer_id: &Option<common_utils::id_type::CustomerId>,
        _merchant_id: &common_utils::id_type::MerchantId,
    ) -> Option<String> {
        todo!()
    }

    /// Check if connector needs tokenization call before setup mandate flow
    fn should_call_tokenization_before_setup_mandate(&self) -> bool {
        true
    }
}

/// Extended trait for connector common to allow functions with generic type
pub trait ConnectorCommonExt<Flow, Req, Resp>:
    ConnectorCommon + ConnectorIntegration<Flow, Req, Resp>
{
    /// common header builder when every request for the connector have same headers
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        Ok(Vec::new())
    }
}

/// trait ConnectorMandateRevoke
pub trait ConnectorMandateRevoke:
    ConnectorIntegration<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>
{
}

/// trait ConnectorMandateRevokeV2
pub trait ConnectorMandateRevokeV2:
    ConnectorIntegrationV2<
    MandateRevoke,
    MandateRevokeFlowData,
    MandateRevokeRequestData,
    MandateRevokeResponseData,
>
{
}

/// trait ConnectorAuthenticationToken
pub trait ConnectorAuthenticationToken:
    ConnectorIntegration<
    AccessTokenAuthentication,
    AccessTokenAuthenticationRequestData,
    AccessTokenAuthenticationResponse,
>
{
}

/// trait ConnectorAuthenticationTokenV2
pub trait ConnectorAuthenticationTokenV2:
    ConnectorIntegrationV2<
    AccessTokenAuthentication,
    AuthenticationTokenFlowData,
    AccessTokenAuthenticationRequestData,
    AccessTokenAuthenticationResponse,
>
{
}

/// trait ConnectorAccessToken
pub trait ConnectorAccessToken:
    ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
{
}

/// trait ConnectorAccessTokenV2
pub trait ConnectorAccessTokenV2:
    ConnectorIntegrationV2<AccessTokenAuth, AccessTokenFlowData, AccessTokenRequestData, AccessToken>
{
}

/// trait ConnectorVerifyWebhookSource
pub trait ConnectorVerifyWebhookSource:
    ConnectorIntegration<
    VerifyWebhookSource,
    VerifyWebhookSourceRequestData,
    VerifyWebhookSourceResponseData,
>
{
}

/// trait ConnectorVerifyWebhookSourceV2
pub trait ConnectorVerifyWebhookSourceV2:
    ConnectorIntegrationV2<
    VerifyWebhookSource,
    WebhookSourceVerifyData,
    VerifyWebhookSourceRequestData,
    VerifyWebhookSourceResponseData,
>
{
}

/// trait UnifiedAuthenticationService
pub trait UnifiedAuthenticationService:
    ConnectorCommon
    + UasPreAuthentication
    + UasPostAuthentication
    + UasAuthenticationConfirmation
    + UasAuthentication
{
}

/// trait UasPreAuthentication
pub trait UasPreAuthentication:
    ConnectorIntegration<
    PreAuthenticate,
    UasPreAuthenticationRequestData,
    UasAuthenticationResponseData,
>
{
}

/// trait UasPostAuthentication
pub trait UasPostAuthentication:
    ConnectorIntegration<
    PostAuthenticate,
    UasPostAuthenticationRequestData,
    UasAuthenticationResponseData,
>
{
}

/// trait UasAuthenticationConfirmation
pub trait UasAuthenticationConfirmation:
    ConnectorIntegration<
    AuthenticationConfirmation,
    UasConfirmationRequestData,
    UasAuthenticationResponseData,
>
{
}

/// trait UasAuthentication
pub trait UasAuthentication:
    ConnectorIntegration<Authenticate, UasAuthenticationRequestData, UasAuthenticationResponseData>
{
}

/// trait UnifiedAuthenticationServiceV2
pub trait UnifiedAuthenticationServiceV2:
    ConnectorCommon
    + UasPreAuthenticationV2
    + UasPostAuthenticationV2
    + UasAuthenticationV2
    + UasAuthenticationConfirmationV2
{
}

///trait UasPreAuthenticationV2
pub trait UasPreAuthenticationV2:
    ConnectorIntegrationV2<
    PreAuthenticate,
    UasFlowData,
    UasPreAuthenticationRequestData,
    UasAuthenticationResponseData,
>
{
}

/// trait UasPostAuthenticationV2
pub trait UasPostAuthenticationV2:
    ConnectorIntegrationV2<
    PostAuthenticate,
    UasFlowData,
    UasPostAuthenticationRequestData,
    UasAuthenticationResponseData,
>
{
}

/// trait UasAuthenticationConfirmationV2
pub trait UasAuthenticationConfirmationV2:
    ConnectorIntegrationV2<
    AuthenticationConfirmation,
    UasFlowData,
    UasConfirmationRequestData,
    UasAuthenticationResponseData,
>
{
}

/// trait UasAuthenticationV2
pub trait UasAuthenticationV2:
    ConnectorIntegrationV2<
    Authenticate,
    UasFlowData,
    UasAuthenticationRequestData,
    UasAuthenticationResponseData,
>
{
}

/// trait ConnectorValidation
pub trait ConnectorValidation: ConnectorCommon + ConnectorSpecifications {
    /// Validate, the payment request against the connector supported features
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<CaptureMethod>,
        payment_method: PaymentMethod,
        pmt: Option<PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        let is_default_capture_method =
            [CaptureMethod::Automatic, CaptureMethod::SequentialAutomatic]
                .contains(&capture_method);
        let is_feature_supported = match self.get_supported_payment_methods() {
            Some(supported_payment_methods) => {
                let connector_payment_method_type_info = get_connector_payment_method_type_info(
                    supported_payment_methods,
                    payment_method,
                    pmt,
                    self.id(),
                )?;

                connector_payment_method_type_info
                    .map(|payment_method_type_info| {
                        payment_method_type_info
                            .supported_capture_methods
                            .contains(&capture_method)
                    })
                    .unwrap_or(true)
            }
            None => is_default_capture_method,
        };

        if is_feature_supported {
            Ok(())
        } else {
            Err(errors::ConnectorError::NotSupported {
                message: capture_method.to_string(),
                connector: self.id(),
            }
            .into())
        }
    }

    /// fn validate_mandate_payment
    fn validate_mandate_payment(
        &self,
        pm_type: Option<PaymentMethodType>,
        _pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let connector = self.id();
        match pm_type {
            Some(pm_type) => Err(errors::ConnectorError::NotSupported {
                message: format!("{pm_type} mandate payment"),
                connector,
            }
            .into()),
            None => Err(errors::ConnectorError::NotSupported {
                message: " mandate payment".to_string(),
                connector,
            }
            .into()),
        }
    }

    /// fn validate_psync_reference_id
    fn validate_psync_reference_id(
        &self,
        data: &router_request_types::PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        data.connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)
            .map(|_| ())
    }

    /// fn is_webhook_source_verification_mandatory
    fn is_webhook_source_verification_mandatory(&self) -> bool {
        false
    }
}

/// trait ConnectorRedirectResponse
pub trait ConnectorRedirectResponse {
    /// fn get_flow_type
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        _action: PaymentAction,
    ) -> CustomResult<CallConnectorAction, errors::ConnectorError> {
        Ok(CallConnectorAction::Avoid)
    }
}

/// Empty trait for when payouts feature is disabled
#[cfg(not(feature = "payouts"))]
pub trait Payouts {}
/// Empty trait for when payouts feature is disabled
#[cfg(not(feature = "payouts"))]
pub trait PayoutsV2 {}

/// Empty trait for when frm feature is disabled
#[cfg(not(feature = "frm"))]
pub trait FraudCheck {}
/// Empty trait for when frm feature is disabled
#[cfg(not(feature = "frm"))]
pub trait FraudCheckV2 {}

fn get_connector_payment_method_type_info(
    supported_payment_method: &SupportedPaymentMethods,
    payment_method: PaymentMethod,
    payment_method_type: Option<PaymentMethodType>,
    connector: &'static str,
) -> CustomResult<Option<PaymentMethodDetails>, errors::ConnectorError> {
    let payment_method_details =
        supported_payment_method
            .get(&payment_method)
            .ok_or_else(|| errors::ConnectorError::NotSupported {
                message: payment_method.to_string(),
                connector,
            })?;

    payment_method_type
        .map(|pmt| {
            payment_method_details.get(&pmt).cloned().ok_or_else(|| {
                errors::ConnectorError::NotSupported {
                    message: format!("{payment_method} {pmt}"),
                    connector,
                }
                .into()
            })
        })
        .transpose()
}

/// ConnectorTransactionId trait
pub trait ConnectorTransactionId: ConnectorCommon + Sync {
    /// fn connector_transaction_id
    fn connector_transaction_id(
        &self,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> Result<Option<String>, ApiErrorResponse> {
        Ok(payment_attempt
            .get_connector_payment_id()
            .map(ToString::to_string))
    }
}

/// Trait ConnectorAccessTokenSuffix
pub trait ConnectorAccessTokenSuffix {
    /// Function to get dynamic access token key suffix from Connector
    fn get_access_token_key<F, Req, Res>(
        &self,
        router_data: &RouterData<F, Req, Res>,
        merchant_connector_id_or_connector_name: String,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(common_utils::access_token::get_default_access_token_key(
            &router_data.merchant_id,
            merchant_connector_id_or_connector_name,
        ))
    }
}
