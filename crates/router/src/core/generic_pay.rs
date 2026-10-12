use std::collections::HashMap;

use hyperswitch_domain_models::payments::HeaderPayload;
use hyperswitch_masking::{PeekInterface, Secret};
use router_env::logger;

use crate::{
    core::{
        errors::{self, RouterResponse},
        payments, utils as core_utils,
    },
    routes::{app::ReqState, SessionState},
    services::{self, authentication::AuthenticationData, ApplicationResponse},
    types::api,
};

/// Codes shared by every adapter. An adapter adds its own for what it rejects on its way in.
pub mod error_codes {
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
    pub const PROFILE_NOT_FOUND: &str = "PROFILE_NOT_FOUND";
    pub const PAYMENT_NOT_COMPLETED: &str = "PAYMENT_NOT_COMPLETED";
    pub const PROCESSING_ERROR: &str = "PROCESSING_ERROR";
    pub const DUPLICATE_TRANSACTION: &str = "DUPLICATE_TRANSACTION";
}

/// Values the IVR attaches to the request. A caller-facing flow has no body of its own, so this is
/// how Hyperswitch ids arrive; the merchant decides what else does, hence the masking.
pub type Parameters = HashMap<String, Secret<String>>;

/// Why a charge is not reaching the connector, or did not end up paid, in terms the provider can
/// read back to the caller.
#[derive(Debug)]
pub struct Failure {
    pub code: String,
    pub message: String,
}

impl Failure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    /// A failure of ours: reported without detail, investigated from the logs.
    fn processing_error() -> Self {
        Self::new(
            error_codes::PROCESSING_ERROR,
            "Payment could not be processed",
        )
    }
}

/// The half of the payment every adapter fills in the same way, read out of [`Parameters`].
pub struct Context {
    /// `None` falls back to the merchant's default profile.
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    /// What makes a provider's retry idempotent: the retry collides on `(payment_id, merchant_id)`
    /// instead of charging again.
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    /// The parameters themselves, carried through for reconciliation.
    pub metadata: Option<serde_json::Value>,
}

impl Context {
    /// The shared fields, for an adapter to fill the rest in with `..context.payments_request()`.
    ///
    /// A card read out over the phone is confirmed in the same call, captured automatically,
    /// flagged as a telephone order, and never sent through 3DS — a caller has no browser.
    pub fn payments_request(self) -> api_models::payments::PaymentsRequest {
        api_models::payments::PaymentsRequest {
            confirm: Some(true),
            capture_method: Some(common_enums::CaptureMethod::Automatic),
            authentication_type: Some(common_enums::AuthenticationType::NoThreeDs),
            payment_channel: Some(common_enums::PaymentChannel::TelephoneOrder),
            profile_id: self.profile_id,
            payment_id: self
                .payment_id
                .map(api_models::payments::PaymentIdType::PaymentIntentId),
            customer_id: self.customer_id,
            metadata: self.metadata,
            // `connector` is left unset so the merchant's configured routing decides.
            ..Default::default()
        }
    }
}

/// One provider's translation to and from Hyperswitch. An adapter implements the translation
/// methods and nothing else; [`charge`](GenericPayAdapter::charge) is the same for all of them.
#[async_trait::async_trait]
pub trait GenericPayAdapter {
    type Request: Send + Sync;
    type Response: Send;

    /// The provider's identifier for the attempt. Logged to line our records up with theirs, not
    /// used as a key — providers do not agree on what makes one unique.
    fn transaction_reference(request: &Self::Request) -> String;

    /// Where in the request the provider carries the IVR's [`Parameters`].
    fn parameters(request: &Self::Request) -> Option<&Parameters>;

    /// Maps the provider's own fields — amount, payment method, descriptors — onto the payment.
    fn build_payments_request(
        request: &Self::Request,
        context: Context,
    ) -> Result<api_models::payments::PaymentsRequest, Failure>;

    fn success_response(payment: &api_models::payments::PaymentsResponse) -> Self::Response;

    fn failure_response(failure: Failure) -> Self::Response;

    /// Answers the provider with a failure, leaving a log line behind to explain it.
    fn reject(
        transaction_reference: &str,
        failure: Failure,
    ) -> ApplicationResponse<Self::Response> {
        logger::warn!(
            transaction_reference = %transaction_reference,
            error_code = %failure.code,
            "Generic pay charge was not completed: {}",
            failure.message
        );
        ApplicationResponse::Json(Self::failure_response(failure))
    }

    /// Creates and confirms the payment the provider is asking for.
    ///
    /// Failures are answered with a `200` carrying an error code rather than propagated, so the
    /// provider reads a result instead of an opaque 5xx. Retries are unaffected: they fire only
    /// when we fail to respond in time, and an error delivered promptly is still a response.
    async fn charge(
        state: SessionState,
        req_state: ReqState,
        auth: AuthenticationData,
        request: Self::Request,
    ) -> RouterResponse<Self::Response> {
        let transaction_reference = Self::transaction_reference(&request);

        let context = match build_context(&state, &auth, Self::parameters(&request)).await {
            Ok(context) => context,
            Err(failure) => return Ok(Self::reject(&transaction_reference, failure)),
        };

        // Passed to the payments core alongside the request, which carries it too.
        let profile_id = context.profile_id.clone();

        let payments_request = match Self::build_payments_request(&request, context)
            .and_then(finalize_payments_request)
        {
            Ok(payments_request) => payments_request,
            Err(failure) => return Ok(Self::reject(&transaction_reference, failure)),
        };

        let payments_response = Box::pin(payments::payments_core::<
            api::Authorize,
            api::PaymentsResponse,
            _,
            _,
            _,
            payments::PaymentData<api::Authorize>,
        >(
            state.clone(),
            req_state,
            auth.platform,
            profile_id,
            payments::PaymentCreate,
            payments_request,
            services::api::AuthFlow::Merchant,
            payments::CallConnectorAction::Trigger,
            None,
            None,
            HeaderPayload::with_source(common_enums::PaymentSource::MerchantServer),
            None,
        ))
        .await;

        let payment = match payments_response {
            Ok(ApplicationResponse::Json(payment))
            | Ok(ApplicationResponse::JsonWithHeaders((payment, _))) => payment,
            Ok(other) => {
                logger::error!(
                    transaction_reference = %transaction_reference,
                    "Unexpected response variant from payments core: {other:?}"
                );
                return Ok(Self::reject(
                    &transaction_reference,
                    Failure::processing_error(),
                ));
            }
            Err(error) => {
                let failure = classify_core_error(&transaction_reference, &error);
                return Ok(Self::reject(&transaction_reference, failure));
            }
        };

        Ok(match payment_outcome(&payment) {
            Ok(()) => ApplicationResponse::Json(Self::success_response(&payment)),
            Err(failure) => Self::reject(&transaction_reference, failure),
        })
    }
}

/// Reads the ids the IVR passed through, checking the profile among them belongs to this merchant.
async fn build_context(
    state: &SessionState,
    auth: &AuthenticationData,
    parameters: Option<&Parameters>,
) -> Result<Context, Failure> {
    Ok(Context {
        profile_id: resolve_profile_id(state, auth, parameters).await?,
        payment_id: extract_id(parameters, "payment_id")?,
        // The customer is a convenience, so an unusable one is dropped rather than failing a
        // charge the caller is waiting on.
        customer_id: extract_id(parameters, "customer_id")
            .inspect_err(|failure| {
                logger::warn!(
                    "Ignoring unusable customer_id from generic pay parameters: {}",
                    failure.message
                )
            })
            .ok()
            .flatten(),
        metadata: build_metadata(parameters),
    })
}

/// Resolves `parameters.profile_id` against the authenticated merchant. These endpoints are
/// merchant level — an API key is scoped to a merchant — so the profile arrives in the parameters,
/// and its absence leaves the payment flow to fall back to the default profile.
async fn resolve_profile_id(
    state: &SessionState,
    auth: &AuthenticationData,
    parameters: Option<&Parameters>,
) -> Result<Option<common_utils::id_type::ProfileId>, Failure> {
    let Some(requested) =
        extract_id::<common_utils::id_type::ProfileId>(parameters, "profile_id")?
    else {
        return Ok(None);
    };

    // Merchant-scoped lookup, so another merchant's profile is "not found" rather than usable.
    core_utils::validate_and_get_business_profile(
        state.store.as_ref(),
        auth.platform.get_processor(),
        Some(&requested),
    )
    .await
    .map_err(|error| {
        logger::warn!("Generic pay profile validation failed: {error:?}");
        Failure::new(
            error_codes::PROFILE_NOT_FOUND,
            format!(
                "profile_id {} is not available for this merchant",
                requested.get_string_repr()
            ),
        )
    })
    .map(|profile| profile.map(|profile| profile.get_id().clone()))
}

/// Reads one Hyperswitch id out of the parameters. An unusable value is rejected rather than
/// ignored: dropping an id the merchant meant to pass is worse than a rejection they can see.
fn extract_id<I>(parameters: Option<&Parameters>, key: &str) -> Result<Option<I>, Failure>
where
    I: TryFrom<std::borrow::Cow<'static, str>>,
    <I as TryFrom<std::borrow::Cow<'static, str>>>::Error: std::fmt::Display,
{
    let Some(value) = parameters.and_then(|parameters| parameters.get(key)) else {
        return Ok(None);
    };

    I::try_from(std::borrow::Cow::Owned(value.peek().clone()))
        .map(Some)
        .map_err(|error| {
            Failure::new(
                error_codes::INVALID_REQUEST,
                format!("parameters.{key} is not a valid {key}: {error}"),
            )
        })
}

/// Carries the IVR's parameters through to the payment, for reconciliation.
fn build_metadata(parameters: Option<&Parameters>) -> Option<serde_json::Value> {
    let parameters = parameters.filter(|parameters| !parameters.is_empty())?;

    serde_json::to_value(parameters)
        .inspect_err(|error| {
            logger::warn!("Could not serialize generic pay parameters into metadata: {error:?}")
        })
        .ok()
}

/// Validates the mapped payment and settles its id, keeping the one the IVR supplied.
fn finalize_payments_request(
    mut payments_request: api_models::payments::PaymentsRequest,
) -> Result<api_models::payments::PaymentsRequest, Failure> {
    payments_request
        .validate()
        .map_err(|message| Failure::new(error_codes::INVALID_REQUEST, message))?;

    crate::routes::payments::get_or_generate_payment_id(&mut payments_request).map_err(
        |error| {
            Failure::new(
                error_codes::INVALID_REQUEST,
                format!("payment_id could not be resolved: {error}"),
            )
        },
    )?;

    Ok(payments_request)
}

/// Reads a payments core error back as something the provider can act on.
fn classify_core_error(
    transaction_reference: &str,
    error: &error_stack::Report<errors::ApiErrorResponse>,
) -> Failure {
    match error.current_context() {
        // A retry of a charge we already hold: the insert collided, so the card was not authorized
        // again and the first payment may well have succeeded. Kept apart from a decline.
        errors::ApiErrorResponse::DuplicatePayment { .. } => {
            logger::info!(
                transaction_reference = %transaction_reference,
                "A charge that already exists was retried; not authorizing it again"
            );
            Failure::new(
                error_codes::DUPLICATE_TRANSACTION,
                "A payment already exists for this payment_id",
            )
        }
        // A failure we cannot classify is reported as a decline and investigated from the logs.
        _ => {
            logger::error!(
                transaction_reference = %transaction_reference,
                "Payments core returned an error for a generic pay charge: {error:?}"
            );
            Failure::processing_error()
        }
    }
}

/// Whether a finished payment is one we can report to the caller as paid.
fn payment_outcome(payment: &api_models::payments::PaymentsResponse) -> Result<(), Failure> {
    match payment.status {
        common_enums::IntentStatus::Succeeded | common_enums::IntentStatus::PartiallyCaptured => {
            Ok(())
        }
        // Everything else is a failure: a phone call cannot wait out a `Processing` or satisfy a
        // `RequiresCustomerAction`, so neither is a result we can read to the caller.
        _ => {
            let code = payment
                .unified_code
                .clone()
                .or_else(|| payment.error_code.clone())
                .unwrap_or_else(|| error_codes::PAYMENT_NOT_COMPLETED.to_string());

            let message = payment
                .unified_message
                .clone()
                .or_else(|| payment.error_message.clone())
                .unwrap_or_else(|| format!("Payment was not completed: {}", payment.status));

            Err(Failure::new(code, message))
        }
    }
}
