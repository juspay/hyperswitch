//! API interface

pub mod disputes;
pub mod disputes_v2;
pub mod files;
pub mod files_v2;
#[cfg(feature = "frm")]
pub mod fraud_check;
#[cfg(feature = "frm")]
pub mod fraud_check_v2;
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

use common_enums::{
    enums::{CallConnectorAction, CaptureMethod, EventClass, PaymentAction, PaymentMethodType},
    PaymentMethod,
};
use common_utils::{
    errors::CustomResult,
    request::{Method, Request, RequestContent},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_data_v2::{
        flow_common_types::WebhookSourceVerifyData, AccessTokenFlowData, MandateRevokeFlowData,
        UasFlowData,
    },
    router_flow_types::{
        mandate_revoke::MandateRevoke, AccessTokenAuth, Authenticate, AuthenticationConfirmation,
        PostAuthenticate, PreAuthenticate, VerifyWebhookSource,
    },
    router_request_types::{
        unified_authentication_service::{
            UasAuthenticationRequestData, UasAuthenticationResponseData,
            UasConfirmationRequestData, UasPostAuthenticationRequestData,
            UasPreAuthenticationRequestData,
        },
        AccessTokenRequestData, MandateRevokeRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        ConnectorInfo, MandateRevokeResponseData, PaymentMethodDetails, SupportedPaymentMethods,
        VerifyWebhookSourceResponseData,
    },
};
use masking::Maskable;
use serde_json::json;

#[cfg(feature = "payouts")]
pub use self::payouts::*;
pub use self::{payments::*, refunds::*};
use crate::{
    configs::Connectors, connector_integration_v2::ConnectorIntegrationV2, consts, errors,
    events::connector_api_logs::ConnectorEvent, metrics, types,
};

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
            issuer_error_code: None,
            issuer_error_message: None,
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
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

/// The trait that provides specifications about the connector
pub trait ConnectorSpecifications {
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
                    .unwrap_or_else(|| is_default_capture_method)
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
                message: format!("{} mandate payment", pm_type),
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
        data: &hyperswitch_domain_models::router_request_types::PaymentsSyncData,
        _is_three_ds: bool,
        _status: common_enums::enums::AttemptStatus,
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
                    message: format!("{} {}", payment_method, pmt),
                    connector,
                }
                .into()
            })
        })
        .transpose()
}
