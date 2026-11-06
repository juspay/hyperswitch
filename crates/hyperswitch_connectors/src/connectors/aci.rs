mod aci_result_codes;
mod cof_record;
#[cfg(test)]
mod golden_tests;
mod scheme_trace_id;
#[cfg(test)]
mod test_fixtures;
pub mod transformers;

use std::{fmt::Debug, sync::LazyLock};

use api_models::webhooks::IncomingWebhookEvent;
use common_enums::enums;
use common_utils::{
    crypto,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        authentication::{
            Authentication, PostAuthentication, PreAuthentication, PreAuthenticationVersionCall,
        },
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        authentication::{
            ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData,
            PreAuthNRequestData,
        },
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        AuthenticationResponseData, ConnectorInfo, PaymentMethodDetails, PaymentsResponseData,
        RefundsResponseData, SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::SetupMandateRouterData,
};
use hyperswitch_interfaces::{
    api::{
        self,
        authentication::{
            ConnectorAuthentication, ConnectorPostAuthentication, ConnectorPreAuthentication,
            ConnectorPreAuthenticationVersionCall, ExternalAuthentication,
        },
        ConnectorCommon, ConnectorIntegration, ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails, WebhookContext},
};
use hyperswitch_masking::{ExposeInterface, Mask, Maskable, PeekInterface, Secret};
use serde::{de::DeserializeOwned, Serialize};
use transformers::{self as aci, Endpoint};

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{
        convert_amount, convert_payment_authorize_router_response,
        convert_setup_mandate_router_data_to_authorize_router_data,
    },
};

#[derive(Clone)]
pub struct Aci {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Aci {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }

    /// `router_data` with `amount` in ACI's major-unit decimal string.
    fn with_amount<'a, T>(
        &self,
        amount: MinorUnit,
        currency: enums::Currency,
        router_data: &'a T,
    ) -> CustomResult<aci::AciRouterData<&'a T>, errors::ConnectorError> {
        let amount = convert_amount(self.amount_converter, amount, currency)?;
        Ok(aci::AciRouterData::from((amount, router_data)))
    }

    fn headers(
        &self,
        auth_type: &ConnectorAuthType,
        sends_content_type: bool,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let content_type = sends_content_type.then(|| {
            (
                headers::CONTENT_TYPE.to_string(),
                self.common_get_content_type().to_string().into(),
            )
        });
        Ok(content_type
            .into_iter()
            .chain(self.get_auth_header(auth_type)?)
            .collect())
    }

    /// Parses an ACI response body as `R` and maps it onto the flow's router data, failing with
    /// `mapping_error` when the mapping does.
    fn handle<R, F, Req, Resp>(
        &self,
        data: &RouterData<F, Req, Resp>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
        label: &'static str,
        mapping_error: errors::ConnectorError,
    ) -> CustomResult<RouterData<F, Req, Resp>, errors::ConnectorError>
    where
        R: DeserializeOwned + Serialize + Debug,
        RouterData<F, Req, Resp>: TryFrom<
            ResponseRouterData<F, R, Req, Resp>,
            Error = error_stack::Report<errors::ConnectorError>,
        >,
        F: Clone,
        Req: Clone,
        Resp: Clone,
    {
        let response: R = res
            .response
            .parse_struct(label)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(mapping_error)
    }

    /// Mandate setup goes out over the same `/v1/payments` machinery as a charge: the setup is
    /// reshaped into an authorize and built as a zero-amount verification. Raw cards only, the
    /// same surface as the old `/v1/registrations` path: a wallet or bank redirect has no
    /// credential ACI can register from a verification.
    fn setup_mandate_request(
        &self,
        req: &SetupMandateRouterData,
    ) -> CustomResult<aci::AciPaymentsRequest, errors::ConnectorError> {
        match req.request.payment_method_data {
            PaymentMethodData::Card(_) | PaymentMethodData::CardWithOptionalCVC(_) => {
                let authorize_req = convert_payment_authorize_router_response((
                    req,
                    setup_mandate_as_authorize_data(req),
                ));
                let connector_router_data = self.with_amount(
                    authorize_req.request.minor_amount,
                    authorize_req.request.currency,
                    &authorize_req,
                )?;
                aci::setup_mandate_request(&connector_router_data)
            }
            _ => Err(errors::ConnectorError::NotSupported {
                message: "Mandate setup for non-card payment methods".to_string(),
                connector: "ACI".into(),
            }
            .into()),
        }
    }
}

/// Reshape a setup-mandate request into the authorize request the shared ACI request builder
/// consumes.
///
/// The shared converter zeroes the amount (a setup is a zero-amount verification) but also drops
/// three fields ACI does put on the wire, because most connectors have no use for them on this
/// flow. They are restored here rather than in the shared helper, which five other connectors also
/// call.
fn setup_mandate_as_authorize_data(req: &SetupMandateRouterData) -> PaymentsAuthorizeData {
    let mut authorize_data = convert_setup_mandate_router_data_to_authorize_router_data(req);
    // `customParameters[*]` — merchant-supplied pairs, and the fixed orchestrator/paymentId.
    authorize_data.metadata = req.request.metadata.clone().map(ExposeInterface::expose);
    // `merchantTransactionId` prefers the merchant's own order reference over the attempt id.
    authorize_data.merchant_order_reference_id = req.request.merchant_order_reference_id.clone();
    // `threeDSecure.*` — an externally authenticated setup must carry its CAVV/ECI, or the acquirer
    // sees an unauthenticated CIT and the liability shift is lost.
    authorize_data.authentication_data = req.request.authentication_data.clone();
    authorize_data
}

impl ConnectorCommon for Aci {
    fn id(&self) -> &'static str {
        "aci"
    }
    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }
    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.aci.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = aci::AciAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.api_key.peek()).into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: aci::AciErrorResponse = res
            .response
            .parse_struct("AciErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        Ok(response.into_error_response(res.status_code))
    }
}

impl ConnectorValidation for Aci {}

/// Implements [`ConnectorIntegration`] for one ACI flow: the [`Endpoint`] it calls, the request
/// body it sends (a query sends none), whether its headers carry a `Content-Type` (every flow's
/// do but the 3DS status query's), and the response it parses — named for the parse error, and
/// mapped onto the flow's router data, a failure of which reports `mapping_error`
/// (`ResponseHandlingFailed` unless given).
///
/// Headers depend on the account alone, never on the request, so `get_headers` cannot fail for
/// want of an id the URL needs.
macro_rules! aci_flow {
    (
        $(#[$doc:meta])*
        $flow:ty: $request:ty => $response:ty {
            endpoint: |$endpoint_req:ident| $endpoint:expr,
            $(body: |$connector:ident, $body_req:ident| $body:expr,)?
            $(content_type: $content_type:literal,)?
            parses: $aci_response:ty as $label:literal,
            $(mapping_error: $mapping_error:ident,)?
        }
    ) => {
        $(#[$doc])*
        impl ConnectorIntegration<$flow, $request, $response> for Aci {
            fn get_headers(
                &self,
                req: &RouterData<$flow, $request, $response>,
                _connectors: &Connectors,
            ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
                self.headers(
                    &req.connector_auth_type,
                    aci_flow!(@content_type $($content_type)?),
                )
            }

            fn get_content_type(&self) -> &'static str {
                self.common_get_content_type()
            }

            fn get_url(
                &self,
                req: &RouterData<$flow, $request, $response>,
                connectors: &Connectors,
            ) -> CustomResult<String, errors::ConnectorError> {
                let $endpoint_req = req;
                $endpoint.url(self.base_url(connectors), &req.connector_auth_type)
            }

            $(
                fn get_request_body(
                    &self,
                    $body_req: &RouterData<$flow, $request, $response>,
                    _connectors: &Connectors,
                ) -> CustomResult<RequestContent, errors::ConnectorError> {
                    let $connector = self;
                    Ok(RequestContent::FormUrlEncoded(Box::new($body)))
                }
            )?

            fn build_request(
                &self,
                req: &RouterData<$flow, $request, $response>,
                connectors: &Connectors,
            ) -> CustomResult<Option<Request>, errors::ConnectorError> {
                let $endpoint_req = req;
                let builder = RequestBuilder::new()
                    .method($endpoint.method())
                    .url(&ConnectorIntegration::<$flow, $request, $response>::get_url(
                        self, req, connectors,
                    )?)
                    .attach_default_headers()
                    .headers(ConnectorIntegration::<$flow, $request, $response>::get_headers(
                        self, req, connectors,
                    )?);
                $(
                    let $body_req = req;
                    let builder = builder.set_body(
                        ConnectorIntegration::<$flow, $request, $response>::get_request_body(
                            self, $body_req, connectors,
                        )?,
                    );
                )?
                Ok(Some(builder.build()))
            }

            fn handle_response(
                &self,
                data: &RouterData<$flow, $request, $response>,
                event_builder: Option<&mut ConnectorEvent>,
                res: Response,
            ) -> CustomResult<RouterData<$flow, $request, $response>, errors::ConnectorError> {
                self.handle::<$aci_response, _, _, _>(
                    data,
                    event_builder,
                    res,
                    $label,
                    aci_flow!(@mapping_error $($mapping_error)?),
                )
            }

            fn get_error_response(
                &self,
                res: Response,
                event_builder: Option<&mut ConnectorEvent>,
            ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
                self.build_error_response(res, event_builder)
            }
        }
    };
    (@content_type) => {
        true
    };
    (@content_type $content_type:literal) => {
        $content_type
    };
    (@mapping_error) => {
        errors::ConnectorError::ResponseHandlingFailed
    };
    (@mapping_error $mapping_error:ident) => {
        errors::ConnectorError::$mapping_error
    };
}

impl api::Payment for Aci {}
impl api::PaymentAuthorize for Aci {}
impl api::PaymentSync for Aci {}
impl api::PaymentVoid for Aci {}
impl api::PaymentCapture for Aci {}
impl api::PaymentSession for Aci {}
impl api::ConnectorAccessToken for Aci {}
impl api::PaymentToken for Aci {}
impl api::MandateSetup for Aci {}
impl api::Refund for Aci {}
impl api::RefundExecute for Aci {}
impl api::RefundSync for Aci {}
impl ConnectorPreAuthentication for Aci {}
impl ConnectorPreAuthenticationVersionCall for Aci {}
impl ConnectorAuthentication for Aci {}
impl ConnectorPostAuthentication for Aci {}
impl ExternalAuthentication for Aci {}

aci_flow! {
    /// A charge or preauthorization (`DB`/`PA`), or a charge against a stored registration.
    Authorize: PaymentsAuthorizeData => PaymentsResponseData {
        endpoint: |req| Endpoint::authorize(&req.request),
        body: |connector, req| aci::AciPaymentsRequest::try_from(&connector.with_amount(
            req.request.minor_amount,
            req.request.currency,
            req,
        )?)?,
        parses: aci::AciPaymentsResponse as "AciPaymentsResponse",
    }
}

aci_flow! {
    /// Mandate setup as a zero-amount CIT on `/v1/payments`, not bare tokenization on
    /// `/v1/registrations`, which takes no amount, no `paymentType` and no
    /// `standingInstruction` block: the acquirer is never told a card-on-file agreement was
    /// established and returns no scheme reference for later MITs to quote.
    ///
    /// Deliberately the same response type and mapping as authorize: the setup is a payment, so
    /// the whole harvest comes for free — `registrationId` as the mandate id, the scheme
    /// reference, the Mastercard agreement id and the sent series type as the stamped
    /// card-on-file record, and a 3DS redirect on a challenged verification (recorded once its
    /// PSync is final).
    SetupMandate: SetupMandateRequestData => PaymentsResponseData {
        endpoint: |_req| Endpoint::Payments,
        body: |connector, req| connector.setup_mandate_request(req)?,
        parses: aci::AciPaymentsResponse as "AciPaymentsResponse",
    }
}

aci_flow! {
    PSync: PaymentsSyncData => PaymentsResponseData {
        endpoint: |req| Endpoint::PaymentStatus(
            req.request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
        ),
        parses: aci::AciPaymentsResponse as "AciPaymentsResponse",
    }
}

aci_flow! {
    Capture: PaymentsCaptureData => PaymentsResponseData {
        endpoint: |req| Endpoint::Payment(req.request.connector_transaction_id.clone()),
        body: |connector, req| aci::AciCaptureRequest::try_from(&connector.with_amount(
            req.request.minor_amount_to_capture,
            req.request.currency,
            req,
        )?)?,
        parses: aci::AciCaptureResponse as "AciCaptureResponse",
    }
}

aci_flow! {
    /// Answered by the payment response, whose success settles at `Voided`.
    Void: PaymentsCancelData => PaymentsResponseData {
        endpoint: |req| Endpoint::Payment(req.request.connector_transaction_id.clone()),
        body: |_connector, req| aci::AciCancelRequest::try_from(req)?,
        parses: aci::AciPaymentsResponse as "AciPaymentsResponse",
    }
}

aci_flow! {
    Execute: RefundsData => RefundsResponseData {
        endpoint: |req| Endpoint::Payment(req.request.connector_transaction_id.clone()),
        body: |connector, req| aci::AciRefundRequest::try_from(&connector.with_amount(
            req.request.minor_refund_amount,
            req.request.currency,
            req,
        )?)?,
        parses: aci::AciRefundResponse as "AciRefundResponse",
        mapping_error: ResponseDeserializationFailed,
    }
}

aci_flow! {
    /// Standalone 3DS, a non-payment authentication (`threeDSecure.npa=true`).
    PreAuthentication: PreAuthNRequestData => AuthenticationResponseData {
        endpoint: |_req| Endpoint::ThreeDSecure,
        body: |_connector, req| aci::AciStandaloneThreeDsRequest::try_from(req)?,
        parses: aci::AciPaymentsResponse as "AciPaymentsResponse (3DS PreAuth)",
    }
}

aci_flow! {
    /// The final result of a standalone 3DS authentication.
    PostAuthentication: ConnectorPostAuthenticationRequestData => AuthenticationResponseData {
        endpoint: |req| {
            Endpoint::ThreeDSecureStatus(req.request.threeds_server_transaction_id.clone())
        },
        content_type: false,
        parses: aci::AciPaymentsResponse as "AciPaymentsResponse (3DS PostAuth)",
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Aci {}

/// ACI has no separate 3DS versioning endpoint.
impl
    ConnectorIntegration<
        PreAuthenticationVersionCall,
        PreAuthNRequestData,
        AuthenticationResponseData,
    > for Aci
{
}

/// The redirect/challenge is handled by the browser after pre-authentication.
impl
    ConnectorIntegration<
        Authentication,
        ConnectorAuthenticationRequestData,
        AuthenticationResponseData,
    > for Aci
{
}

/// Flows ACI has no counterpart for.
macro_rules! unsupported_flow {
    ($flow:ty: $request:ty => $response:ty, $message:literal) => {
        impl ConnectorIntegration<$flow, $request, $response> for Aci {
            fn build_request(
                &self,
                _req: &RouterData<$flow, $request, $response>,
                _connectors: &Connectors,
            ) -> CustomResult<Option<Request>, errors::ConnectorError> {
                Err(errors::ConnectorError::NotSupported {
                    message: $message.to_string(),
                    connector: "ACI".into(),
                }
                .into())
            }
        }
    };
}

unsupported_flow!(
    PaymentMethodToken: PaymentMethodTokenizationData => PaymentsResponseData,
    "Payment method tokenization not supported"
);
unsupported_flow!(
    Session: PaymentsSessionData => PaymentsResponseData,
    "Payment sessions not supported"
);
unsupported_flow!(
    AccessTokenAuth: AccessTokenRequestData => AccessToken,
    "Access token authentication not supported"
);

/// The value of webhook header `name`, when present; a non-UTF-8 value is an error.
fn optional_webhook_header<'a>(
    request: &'a IncomingWebhookRequestDetails<'_>,
    name: &'static str,
) -> CustomResult<Option<&'a str>, errors::ConnectorError> {
    request
        .headers
        .get(name)
        .map(|value| {
            value
                .to_str()
                .map_err(|_| report!(errors::ConnectorError::WebhookBodyDecodingFailed))
                .attach_printable_lazy(|| format!("Invalid {name} header value (not UTF-8)"))
        })
        .transpose()
}

/// The webhook body as it arrived, encrypted or not.
fn webhook_body<'a>(
    request: &'a IncomingWebhookRequestDetails<'_>,
) -> CustomResult<aci::AciWebhookBody<'a>, errors::ConnectorError> {
    aci::AciWebhookBody::of(
        optional_webhook_header(request, "X-Initialization-Vector")?,
        optional_webhook_header(request, "X-Authentication-Tag")?,
        request.body,
    )
}

/// The notification in a decrypted webhook body.
fn webhook_notification(
    request: &IncomingWebhookRequestDetails<'_>,
    error: errors::ConnectorError,
    purpose: &'static str,
) -> CustomResult<aci::AciWebhookNotification, errors::ConnectorError> {
    serde_json::from_slice(request.body)
        .change_context(error)
        .attach_printable_lazy(|| {
            format!(
                "Failed to deserialize ACI webhook notification for {purpose} (expected decrypted payload)"
            )
        })
}

#[async_trait::async_trait]
impl IncomingWebhook for Aci {
    /// Decrypts the body with the webhook secret of the account the webhook URL names, so every
    /// later step — and source verification — reads the notification JSON.
    ///
    /// The secret is the account's, so the webhook URL must name it (`.../{merchant_id}/mca_...`):
    /// a URL naming only the connector carries no secret and every encrypted body fails here.
    async fn decode_webhook_body(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        connector_name: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_name,
                connector_webhook_details,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        webhook_body(request)?.decode(&secrets.secret)
    }

    /// AES-GCM is authenticated encryption: [`Self::decode_webhook_body`] already proved the
    /// source when it decrypted the body, and the body handed here is the plaintext. What is left
    /// to tell is whether the notification was encrypted at all.
    async fn verify_webhook_source(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
        _connector_name: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        Ok(webhook_body(request)?.is_authenticated())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        webhook_notification(
            request,
            errors::ConnectorError::WebhookResourceObjectNotFound,
            "ID extraction",
        )?
        .object_reference_id()
        .ok_or_else(|| {
            report!(errors::ConnectorError::WebhookResourceObjectNotFound)
                .attach_printable("Missing 'id' in webhook payload for ID extraction")
        })
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _context: Option<&WebhookContext>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        let notification = webhook_notification(
            request,
            errors::ConnectorError::WebhookEventTypeNotFound,
            "event type",
        )?;
        match notification.into_payment_payload() {
            Some(payload) => payload
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)
                .attach_printable(
                    "Could not deserialize ACI payment webhook payload for event type determination",
                )
                .map(|payload| payload.event()),
            None => Ok(IncomingWebhookEvent::EventNotSupported),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        let notification = webhook_notification(
            request,
            errors::ConnectorError::WebhookResourceObjectNotFound,
            "resource object",
        )?;
        match notification.into_payment_payload() {
            Some(payload) => {
                let payload = payload
                    .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)
                    .attach_printable("Failed to deserialize ACI payment webhook payload")?;
                Ok(Box::new(payload))
            }
            None => Err(errors::ConnectorError::WebhookEventTypeNotFound.into()),
        }
    }
}

static ACI_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::Manual,
    ];
    let card = PaymentMethodDetails {
        mandates: enums::FeatureStatus::Supported,
        refunds: enums::FeatureStatus::Supported,
        supported_capture_methods: supported_capture_methods.clone(),
        specific_features: Some(
            api_models::feature_matrix::PaymentMethodSpecificFeatures::Card(
                api_models::feature_matrix::CardSpecificFeatures {
                    three_ds: common_enums::FeatureStatus::Supported,
                    no_three_ds: common_enums::FeatureStatus::Supported,
                    supported_card_networks: vec![
                        common_enums::CardNetwork::Visa,
                        common_enums::CardNetwork::Mastercard,
                        common_enums::CardNetwork::AmericanExpress,
                        common_enums::CardNetwork::JCB,
                        common_enums::CardNetwork::DinersClub,
                        common_enums::CardNetwork::Discover,
                        common_enums::CardNetwork::UnionPay,
                        common_enums::CardNetwork::Maestro,
                    ],
                },
            ),
        ),
    };
    let without_mandates = PaymentMethodDetails {
        mandates: enums::FeatureStatus::NotSupported,
        refunds: enums::FeatureStatus::Supported,
        supported_capture_methods,
        specific_features: None,
    };

    use enums::{PaymentMethod, PaymentMethodType};
    let table: [(PaymentMethod, &[PaymentMethodType], &PaymentMethodDetails); 4] = [
        (
            PaymentMethod::Card,
            &[PaymentMethodType::Credit, PaymentMethodType::Debit],
            &card,
        ),
        (
            PaymentMethod::Wallet,
            &[
                PaymentMethodType::MbWay,
                PaymentMethodType::AliPay,
                PaymentMethodType::ApplePay,
                PaymentMethodType::GooglePay,
                PaymentMethodType::SamsungPay,
            ],
            &without_mandates,
        ),
        (
            PaymentMethod::BankRedirect,
            &[
                PaymentMethodType::Eps,
                PaymentMethodType::Eft,
                PaymentMethodType::Ideal,
                PaymentMethodType::Giropay,
                PaymentMethodType::Sofort,
                PaymentMethodType::Interac,
                PaymentMethodType::Przelewy24,
                PaymentMethodType::Trustly,
            ],
            &without_mandates,
        ),
        (
            PaymentMethod::PayLater,
            &[PaymentMethodType::Klarna],
            &without_mandates,
        ),
    ];

    let mut supported = SupportedPaymentMethods::new();
    for (payment_method, payment_method_types, details) in table {
        for payment_method_type in payment_method_types {
            supported.add(payment_method, *payment_method_type, details.clone());
        }
    }
    supported
});

static ACI_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "ACI",
    description:
        "ACI Payments delivers secure, real-time electronic payment solutions for businesses, banks, and governments, enabling seamless transactions across channels.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Sandbox,
};

static ACI_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Aci {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&ACI_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*ACI_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&ACI_SUPPORTED_WEBHOOK_FLOWS)
    }

    #[cfg(feature = "v1")]
    fn generate_connector_request_reference_id(
        &self,
        _payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
        _is_config_enabled_to_send_payment_id_as_connector_request_id: bool,
    ) -> String {
        // OPPWA merchantTransactionId carries the payment id without the `_N` attempt suffix.
        payment_attempt.payment_id.get_string_repr().to_owned()
    }
}
