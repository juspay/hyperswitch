pub mod twilio;

use std::collections::HashMap;

use common_utils::events::ApiEventMetric;
use hyperswitch_domain_models::payments::HeaderPayload;
use router_env::logger;

use crate::{
    core::{
        api_locking,
        errors::{self, RouterResponse},
        payments,
    },
    routes::{app::ReqState, lock_utils, SessionState},
    services::{
        self,
        authentication::{
            ApiKeyBasicAuthWithMerchantIdFromRoute, AuthenticateAndFetch, AuthenticationData,
        },
        ApplicationResponse,
    },
    types::api,
};

/// Codes shared by every adapter. An adapter adds its own for what it rejects on its way in.
pub mod error_codes {
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
    pub const PROFILE_NOT_FOUND: &str = "PROFILE_NOT_FOUND";
    pub const PAYMENT_NOT_COMPLETED: &str = "PAYMENT_NOT_COMPLETED";
    pub const PROCESSING_ERROR: &str = "PROCESSING_ERROR";
    pub const DUPLICATE_TRANSACTION: &str = "DUPLICATE_TRANSACTION";
    /// The charge reached the connector with no final outcome. Kept apart from a decline so an
    /// IVR can be told not to ask for another card: the first authorization may be standing.
    pub const PAYMENT_PENDING: &str = "PAYMENT_PENDING";
    /// A card cannot be stored against nobody. Named apart so the IVR fix is unambiguous.
    pub const CUSTOMER_REQUIRED: &str = "CUSTOMER_REQUIRED";
    /// A card cannot be stored without consent, and the IVR has to ask before it can send one.
    pub const ACCEPTANCE_REQUIRED: &str = "ACCEPTANCE_REQUIRED";
    /// Verified card, nothing stored. Not a decline — another card would fail the same way.
    pub const TOKENIZE_FAILED: &str = "TOKENIZE_FAILED";
}

/// What to do with the collected card. Each maps to a different payments flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Moves money. Stores the card too, when the IVR asked for that.
    Charge,
    /// Stores the card and moves no money: a zero-amount verification at the connector.
    Tokenize,
}

/// A mapped provider request and the operation it expresses, so flow and payment cannot disagree.
pub struct Intent {
    pub operation: Operation,
    pub payments_request: api_models::payments::PaymentsRequest,
}

impl Intent {
    pub fn charge(payments_request: api_models::payments::PaymentsRequest) -> Self {
        Self {
            operation: Operation::Charge,
            payments_request,
        }
    }

    pub fn tokenize(payments_request: api_models::payments::PaymentsRequest) -> Self {
        Self {
            operation: Operation::Tokenize,
            payments_request,
        }
    }

    /// Runs `f` over the payment, keeping the operation attached to it.
    fn try_map(
        self,
        f: impl FnOnce(
            api_models::payments::PaymentsRequest,
        ) -> Result<api_models::payments::PaymentsRequest, Failure>,
    ) -> Result<Self, Failure> {
        Ok(Self {
            operation: self.operation,
            payments_request: f(self.payments_request)?,
        })
    }
}

pub use api_models::telephone_order::TelephoneOrderParameters as Parameters;

/// Why a charge failed, in terms the provider can read back to the caller.
#[derive(Debug)]
pub struct Failure {
    pub code: String,
    pub message: String,
    pub parameters: ResponseParameters,
}

impl Failure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            parameters: ResponseParameters::new(),
        }
    }

    /// Only for a failure with a payment behind it; one rejected earlier has nothing to point at.
    fn with_parameters(mut self, parameters: ResponseParameters) -> Self {
        self.parameters = parameters;
        self
    }

    /// A failure of ours: reported without detail, investigated from the logs.
    fn processing_error() -> Self {
        Self::new(
            error_codes::PROCESSING_ERROR,
            "Payment could not be processed",
        )
    }
}

pub type ResponseParameters = HashMap<String, String>;

/// How a finished payment reads to the caller. Three outcomes, not two: a payment that is not yet
/// paid is not thereby declined, and the difference decides whether the IVR asks for another card.
#[derive(Debug)]
pub enum Outcome {
    /// Money is secured. An authorization counts — the capture is the merchant's to make.
    Paid,
    /// Terminal, and no money moved. Asking the caller for another card is the right next step.
    Declined(Failure),
    /// We do not know, or money is held in a state that cannot be read back. Carries
    /// [`error_codes::PAYMENT_PENDING`] so a provider can tell it from a decline.
    Indeterminate(Failure),
}

/// The half of the payment every adapter fills in the same way, read out of [`Parameters`].
pub struct Context {
    /// `None` falls back to the merchant's default profile.
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    /// What makes a provider's retry idempotent: it collides on `(payment_id, merchant_id)`
    /// instead of charging again. Settled by [`TelephoneOrderAdapter::resolve_payment_id`].
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    /// `None` auto-captures. `Manual` leaves the capture to the merchant.
    pub capture_method: Option<common_enums::CaptureMethod>,
    /// `Some` keeps the card against the customer. Never defaulted — unattended charging is
    /// authority the merchant claims, not something we assume.
    pub setup_future_usage: Option<common_enums::FutureUsage>,
    /// The caller's agreement to that, as the IVR reported it. Never synthesised.
    pub customer_acceptance: Option<common_types::payments::CustomerAcceptance>,
    /// Whether the connector mints its own token. `None` leaves it to Hyperswitch; see
    /// [`Parameters::tokenization`].
    pub tokenization: Option<common_enums::Tokenization>,
    /// What the IVR knows about the caller. Merged with the provider's postal code by
    /// [`Context::billing_address`].
    pub billing: Option<api_models::payments::Address>,
    pub metadata: Option<serde_json::Value>,
}

impl Context {
    /// The IVR's address, with the postal code the provider collected filling in for a missing
    /// `zip`. A fallback, not an override: an IVR that sent a whole address sent a coherent one,
    /// and dropping a phone-collected postal code into the middle of it would make an address
    /// that matches nothing — which fails AVS harder than either would alone.
    pub fn billing_address(
        &self,
        postal_code: Option<hyperswitch_masking::Secret<String>>,
    ) -> Option<api_models::payments::Address> {
        if self.billing.is_none() && postal_code.is_none() {
            return None;
        }

        let mut billing = self.billing.clone().unwrap_or_default();
        if let Some(postal_code) = postal_code {
            billing
                .address
                .get_or_insert_with(Default::default)
                .zip
                .get_or_insert(postal_code);
        }
        Some(billing)
    }

    /// The shared fields, for an adapter to fill the rest in with `..context.payments_request()`.
    /// Confirmed, flagged a telephone order, never 3DS — a caller has no browser. Auto-captured
    /// unless the IVR asked otherwise.
    pub fn payments_request(self) -> api_models::payments::PaymentsRequest {
        api_models::payments::PaymentsRequest {
            confirm: Some(true),
            capture_method: Some(
                self.capture_method
                    .unwrap_or(common_enums::CaptureMethod::Automatic),
            ),
            authentication_type: Some(common_enums::AuthenticationType::NoThreeDs),
            payment_channel: Some(common_enums::PaymentChannel::TelephoneOrder),
            profile_id: self.profile_id,
            payment_id: self
                .payment_id
                .map(api_models::payments::PaymentIdType::PaymentIntentId),
            customer_id: self.customer_id,
            setup_future_usage: self.setup_future_usage,
            customer_acceptance: self.customer_acceptance,
            tokenization: self.tokenization,
            // Also on the billing address, which is where connectors look first.
            email: self
                .billing
                .as_ref()
                .and_then(|billing| billing.email.clone()),
            metadata: self.metadata,
            // `connector` is left unset so the merchant's configured routing decides.
            ..Default::default()
        }
    }
}

/// The providers this endpoint serves, as they appear in the `{provider}` path segment. These are
/// inbound providers that hand us a card to charge, not connectors we send it to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TelephoneOrderProvider {
    /// Twilio's `<Pay>` verb, through its Generic Pay Connector.
    Twilio,
}

/// One provider's translation to and from Hyperswitch. An adapter implements the translation
/// methods and nothing else; [`charge`](TelephoneOrderAdapter::charge) is the same for all of them.
#[async_trait::async_trait]
pub trait TelephoneOrderAdapter {
    /// The provider's request body. Serialized into the `ApiEvent` log through `masked_serialize`,
    /// so every sensitive field on it must be a `Secret`; [`Parameters`] is the exception.
    type Request: serde::de::DeserializeOwned
        + serde::Serialize
        + std::fmt::Debug
        + ApiEventMetric
        + Send
        + Sync;

    /// What the provider expects back, on success and on failure alike.
    type Response: serde::Serialize + std::fmt::Debug + ApiEventMetric + Send;

    /// The provider's identifier for the attempt. Logged, not used as a key.
    fn transaction_reference(request: &Self::Request) -> String;

    /// The provider's own value that stays the same across *its* retries: the payment id derives
    /// from it, so a retry collides instead of charging again. `None` means no such protection.
    fn idempotency_reference(request: &Self::Request) -> Option<&str>;

    /// Where the provider carries the IVR's [`Parameters`]. A reference: the ids read out of it
    /// are cloned by the few callers that need them owned.
    fn parameters(request: &Self::Request) -> Option<&Parameters>;

    /// Maps the provider's own fields onto the payment, and says which operation it is.
    fn build_intent(request: &Self::Request, context: Context) -> Result<Intent, Failure>;

    /// Answers a completed charge.
    fn charge_response(
        payment: &api_models::payments::PaymentsResponse,
        parameters: ResponseParameters,
    ) -> Self::Response;

    /// Answers a completed tokenize. `Err` when nothing was stored — a success with no token
    /// reads to the provider as one it can use. Adapters that never tokenize can leave this alone.
    fn tokenize_response(
        payment: &api_models::payments::PaymentsResponse,
        parameters: ResponseParameters,
    ) -> Result<Self::Response, Failure> {
        let _ = payment;
        Err(Failure::new(
            error_codes::TOKENIZE_FAILED,
            "Storing a card is not supported on this provider",
        )
        .with_parameters(parameters))
    }

    fn failure_response(failure: Failure) -> Self::Response;

    /// The credential check these requests go through; HTTP Basic carrying a Hyperswitch API key
    /// by default. Auth never sees the body, so `parameters.profile_id` is read here and handed in.
    fn authenticator(
        merchant_id: common_utils::id_type::MerchantId,
        request: &Self::Request,
    ) -> Box<dyn AuthenticateAndFetch<AuthenticationData, SessionState>> {
        Box::new(ApiKeyBasicAuthWithMerchantIdFromRoute {
            merchant_id,
            profile_id: Self::parameters(request)
                .and_then(|parameters| parameters.profile_id.clone()),
        })
    }

    /// The payment id this request becomes, settled in one place because the route locks on it and
    /// the core puts it on the payment. `parameters.payment_id` wins; else [`derive_payment_id`].
    fn resolve_payment_id(request: &Self::Request) -> Option<common_utils::id_type::PaymentId> {
        Self::parameters(request)
            .and_then(|parameters| parameters.payment_id.clone())
            .or_else(|| Self::idempotency_reference(request).and_then(derive_payment_id))
    }

    /// Answers the provider with a failure, leaving a log line behind to explain it.
    fn report_failure(
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

    /// Creates and confirms the payment the provider is asking for. Failures are answered with a
    /// `200` carrying an error code, so the provider reads a result instead of an opaque 5xx.
    async fn charge(
        state: SessionState,
        req_state: ReqState,
        auth: AuthenticationData,
        request: Self::Request,
    ) -> RouterResponse<Self::Response> {
        let transaction_reference = Self::transaction_reference(&request);

        let context = build_context(
            &auth,
            Self::parameters(&request),
            Self::resolve_payment_id(&request),
        );

        let intent = Self::build_intent(&request, context)
            .and_then(|intent| intent.try_map(finalize_payments_request));

        let outcome = match intent {
            Err(failure) => Err(failure),
            Ok(Intent {
                operation,
                payments_request,
            }) => {
                let profile_id = payments_request.profile_id.clone();

                // Same request either way; only the flow differs. `PaymentCreate` is generic
                // over it, so it serves both.
                let payments_response = match operation {
                    Operation::Charge => {
                        Box::pin(payments::payments_core::<
                            api::Authorize,
                            api::PaymentsResponse,
                            _,
                            _,
                            _,
                            payments::PaymentData<api::Authorize>,
                        >(
                            state.clone(),
                            req_state.clone(),
                            auth.platform.clone(),
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
                        .await
                    }
                    Operation::Tokenize => {
                        Box::pin(payments::payments_core::<
                            api::SetupMandate,
                            api::PaymentsResponse,
                            _,
                            _,
                            _,
                            payments::PaymentData<api::SetupMandate>,
                        >(
                            state.clone(),
                            req_state.clone(),
                            auth.platform.clone(),
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
                        .await
                    }
                };

                let payment = match payments_response {
                    Ok(ApplicationResponse::Json(payment))
                    | Ok(ApplicationResponse::JsonWithHeaders((payment, _))) => Ok(payment),
                    Ok(other) => {
                        logger::error!(
                            transaction_reference = %transaction_reference,
                            "Unexpected response variant from payments core: {other:?}"
                        );
                        Err(Failure::processing_error())
                    }
                    // The one error with a payment behind it. Answered with that charge's own
                    // outcome, so the retry does not read as a decline and send the caller round
                    // for a second card.
                    Err(error) => match error.current_context() {
                        errors::ApiErrorResponse::DuplicatePayment { payment_id } => {
                            logger::info!(
                                transaction_reference = %transaction_reference,
                                payment_id = %payment_id.get_string_repr(),
                                "A charge that already exists was retried; replaying its outcome"
                            );

                            retrieve_payment(&state, req_state, &auth, payment_id).await
                        }
                        _ => Err(classify_core_error(&transaction_reference, &error)),
                    },
                };

                // Carried alongside the payment: a replayed duplicate must still answer *this*
                // request's operation.
                payment.map(|payment| (operation, payment))
            }
        };

        Ok(match outcome {
            Ok((operation, payment)) => Self::answer(&transaction_reference, operation, &payment),
            Err(failure) => Self::report_failure(&transaction_reference, failure),
        })
    }

    /// Reads a finished payment back to the provider.
    fn answer(
        transaction_reference: &str,
        operation: Operation,
        payment: &api_models::payments::PaymentsResponse,
    ) -> ApplicationResponse<Self::Response> {
        match payment_outcome(payment) {
            Outcome::Paid => match operation {
                Operation::Charge => ApplicationResponse::Json(Self::charge_response(
                    payment,
                    outcome_parameters(payment),
                )),
                // A verified card with nothing stored is a success with no answer, so the
                // adapter is allowed to fail it.
                Operation::Tokenize => {
                    match Self::tokenize_response(payment, outcome_parameters(payment)) {
                        Ok(response) => ApplicationResponse::Json(response),
                        Err(failure) => {
                            logger::error!(
                                transaction_reference = %transaction_reference,
                                payment_id = %payment.payment_id.get_string_repr(),
                                "Telephone order verified a card but stored no payment method"
                            );
                            Self::report_failure(transaction_reference, failure)
                        }
                    }
                }
            },
            Outcome::Declined(failure) => Self::report_failure(transaction_reference, failure),
            // Logged louder than a decline: nobody has to act on a declined card, and somebody
            // has to act on an authorization the caller was not told about.
            Outcome::Indeterminate(failure) => {
                logger::error!(
                    transaction_reference = %transaction_reference,
                    payment_id = %payment.payment_id.get_string_repr(),
                    status = %payment.status,
                    "Telephone order charge has no outcome that can be read to the caller; \
                     funds may be held at the connector"
                );
                Self::report_failure(transaction_reference, failure)
            }
        }
    }
}

/// Locks the payment this request becomes, so a provider's retry waits for the charge in flight
/// instead of racing it. `NotApplicable` only when there is no id to lock on at all — the route
/// resolves the id once, records it on the span and hands it here.
pub fn locking_action<F>(
    payment_id: Option<&common_utils::id_type::PaymentId>,
    flow: F,
) -> api_locking::LockAction
where
    F: router_env::types::FlowMetric,
    lock_utils::ApiIdentifier: From<F>,
{
    payment_id.map_or(api_locking::LockAction::NotApplicable, |payment_id| {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    })
}

/// Reads the parameters the IVR passed through. Nothing is parsed here: [`Parameters`] holds these
/// as their own types, so an unusable one failed the request's deserialization at the route. The
/// profile was resolved by authentication, the payment id by
/// [`TelephoneOrderAdapter::resolve_payment_id`] — which the route needed first, to lock on.
fn build_context(
    auth: &AuthenticationData,
    parameters: Option<&Parameters>,
    payment_id: Option<common_utils::id_type::PaymentId>,
) -> Context {
    Context {
        // The profile authentication settled, which is the one the IVR named when it named one:
        // `parameters.profile_id` reaches auth through the adapter's `authenticator`.
        profile_id: auth
            .profile
            .as_ref()
            .map(|profile| profile.get_id().clone()),
        payment_id,
        customer_id: parameters.and_then(|parameters| parameters.customer_id.clone()),
        capture_method: parameters
            .and_then(|parameters| parameters.capture_method)
            .map(common_enums::CaptureMethod::from),
        setup_future_usage: parameters.and_then(|parameters| parameters.setup_future_usage),
        customer_acceptance: build_customer_acceptance(parameters),
        tokenization: parameters.and_then(|parameters| parameters.tokenization),
        billing: build_billing(parameters),
        metadata: build_metadata(parameters),
    }
}

/// `None` unless the provider sent one; [`finalize_payments_request`] rejects what needs it.
fn build_customer_acceptance(
    parameters: Option<&Parameters>,
) -> Option<common_types::payments::CustomerAcceptance> {
    let mut acceptance = parameters?.customer_acceptance.clone()?;

    // `now` is a little late — the disclosure plays before the card — but within the same call.
    acceptance
        .accepted_at
        .get_or_insert_with(common_utils::date_time::now);

    Some(acceptance)
}

/// The IVR's billing address. Twilio cannot send one; see [`Parameters::billing`].
fn build_billing(parameters: Option<&Parameters>) -> Option<api_models::payments::Address> {
    parameters?.billing.clone()
}

/// Carries the merchant's own parameters through to the payment, for reconciliation. Lands in
/// `payment_intent.metadata` unmasked — returned by payments retrieve, sent in webhooks. Only
/// `extra`: the fields [`Parameters`] names are on the payment itself.
fn build_metadata(parameters: Option<&Parameters>) -> Option<serde_json::Value> {
    let extra = parameters
        .map(|parameters| &parameters.extra)
        .filter(|extra| !extra.is_empty())?;

    serde_json::to_value(extra)
        .inspect_err(|error| {
            logger::warn!("Could not serialize telephone order parameters into metadata: {error:?}")
        })
        .ok()
}

/// Validates the mapped payment and settles its id, keeping the one the IVR supplied.
fn finalize_payments_request(
    mut payments_request: api_models::payments::PaymentsRequest,
) -> Result<api_models::payments::PaymentsRequest, Failure> {
    // What a stored card needs, checked here: every adapter passes through this function.
    if payments_request.setup_future_usage.is_some() {
        // Caught before the core, whose message names `setup_future_usage` — not a field any
        // IVR set.
        if payments_request.customer_id.is_none() {
            return Err(Failure::new(
                error_codes::CUSTOMER_REQUIRED,
                "customer_id is required to store a card. Pass it as a <Parameter>",
            ));
        }

        // Never written here: an invented consent record would be worth nothing. The save is
        // gated on it anyway, so without it the payment succeeds and stores nothing.
        if payments_request.customer_acceptance.is_none() {
            return Err(Failure::new(
                error_codes::ACCEPTANCE_REQUIRED,
                "customer_acceptance is required to store a card. Ask the caller to agree and \
                 pass it as a <Parameter>",
            ));
        }
    }

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

/// One id per provider attempt, shared by its retries. Derived, not generated: Twilio resends the
/// same body when we miss its budget, so a fresh id would charge the card again. The reference is
/// hashed because it is untrusted text, and kept a pure function so route and core agree.
fn derive_payment_id(reference: &str) -> Option<common_utils::id_type::PaymentId> {
    use common_utils::crypto::GenerateDigest;

    let digest = common_utils::crypto::Sha256
        .generate_digest(reference.as_bytes())
        .inspect_err(|error| {
            logger::error!("Could not derive a payment_id for a telephone order: {error:?}")
        })
        .ok()?;

    // `pay_` is what every payment id in this repo carries, generated or derived; 32 hex
    // characters is half the digest, well inside the 64 a `PaymentId` allows. Nothing stops a
    // merchant choosing this exact string for a payment of their own — the prefix is a
    // convention, not a namespace — but 128 bits of digest makes landing on one by accident not a
    // thing that happens.
    common_utils::id_type::PaymentId::wrap(format!("pay_{}", hex::encode(digest.get(..16)?))).ok()
}

/// Reads back the payment a retry collided with, so the collision answers with the original
/// outcome rather than a decline. Not force-synced: the lock means that charge already finished.
async fn retrieve_payment(
    state: &SessionState,
    req_state: ReqState,
    auth: &AuthenticationData,
    payment_id: &common_utils::id_type::PaymentId,
) -> Result<api_models::payments::PaymentsResponse, Failure> {
    let response = Box::pin(payments::payments_core::<
        api::PSync,
        api::PaymentsResponse,
        _,
        _,
        _,
        payments::PaymentData<api::PSync>,
    >(
        state.clone(),
        req_state,
        auth.platform.clone(),
        None,
        payments::operations::PaymentStatus,
        api::PaymentsRetrieveRequest {
            resource_id: api_models::payments::PaymentIdType::PaymentIntentId(payment_id.clone()),
            merchant_id: Some(auth.platform.get_processor().get_account().get_id().clone()),
            force_sync: false,
            ..Default::default()
        },
        services::api::AuthFlow::Merchant,
        payments::CallConnectorAction::Avoid,
        None,
        None,
        HeaderPayload::default(),
        None,
    ))
    .await;

    match response {
        Ok(ApplicationResponse::Json(payment))
        | Ok(ApplicationResponse::JsonWithHeaders((payment, _))) => Ok(payment),
        // The payment exists, so failing to read it is ours. Still not a decline: the first charge
        // stands either way, and a decline would send the caller round for another card.
        other => {
            logger::error!(
                payment_id = %payment_id.get_string_repr(),
                "Could not read back the payment a telephone order retry collided with: {other:?}"
            );
            Err(Failure::new(
                error_codes::DUPLICATE_TRANSACTION,
                "A payment already exists for this payment_id",
            )
            .with_parameters(ResponseParameters::from([(
                "payment_id".to_string(),
                payment_id.get_string_repr().to_owned(),
            )])))
        }
    }
}

/// Reads a payments core error back as something the provider can act on.
fn classify_core_error(
    transaction_reference: &str,
    error: &error_stack::Report<errors::ApiErrorResponse>,
) -> Failure {
    match error.current_context() {
        // `PaymentCreate`'s merchant-scoped profile lookup, which rejects a `parameters.profile_id`
        // that is not this merchant's — an IVR configuration mistake, not a failure of ours.
        errors::ApiErrorResponse::ProfileNotFound { id } => {
            logger::warn!(
                transaction_reference = %transaction_reference,
                "Telephone order named a profile the merchant does not have: {id}"
            );
            Failure::new(
                error_codes::PROFILE_NOT_FOUND,
                format!("profile_id {id} is not available for this merchant"),
            )
        }
        // Kept apart from `PROCESSING_ERROR`: a mistyped expiry is worth re-prompting for, an
        // outage is not. The core's message names the field to ask for again.
        errors::ApiErrorResponse::PreconditionFailed { message }
        | errors::ApiErrorResponse::InvalidRequestData { message } => {
            invalid_request(transaction_reference, message.clone())
        }
        errors::ApiErrorResponse::InvalidDataValue { field_name } => {
            invalid_request(transaction_reference, format!("{field_name} is not valid"))
        }
        errors::ApiErrorResponse::MissingRequiredField { field_name } => {
            invalid_request(transaction_reference, format!("{field_name} is required"))
        }
        errors::ApiErrorResponse::InvalidDataFormat {
            field_name,
            expected_format,
        } => invalid_request(
            transaction_reference,
            format!("{field_name} is not in the expected format: {expected_format}"),
        ),

        // A failure we cannot classify is reported as `PROCESSING_ERROR` — ours, not the card's —
        // and investigated from the logs.
        _ => {
            logger::error!(
                transaction_reference = %transaction_reference,
                "Payments core returned an error for a telephone order charge: {error:?}"
            );
            Failure::processing_error()
        }
    }
}

/// A rejection the caller caused. `warn`, not `error`: a mistyped card is not an incident.
fn invalid_request(transaction_reference: &str, message: String) -> Failure {
    logger::warn!(
        transaction_reference = %transaction_reference,
        "Telephone order charge was rejected on its input: {message}"
    );
    Failure::new(error_codes::INVALID_REQUEST, message)
}

/// How a finished payment reads to the caller. Matched exhaustively on purpose: a wildcard would
/// drop every `IntentStatus` Hyperswitch grows next into whichever bucket happens to be last.
fn payment_outcome(payment: &api_models::payments::PaymentsResponse) -> Outcome {
    use common_enums::IntentStatus as Status;

    match payment.status {
        // Captured, in whole or in part.
        Status::Succeeded | Status::PartiallyCaptured => Outcome::Paid,

        // Authorized and not captured — still paid: the funds are held, and a decline here would
        // leave the authorization standing while the IVR collects another card.
        Status::RequiresCapture | Status::PartiallyAuthorizedAndRequiresCapture => Outcome::Paid,

        // Money has moved, or may yet, by an amount that cannot be read back. `CancelledPostCapture`
        // too: a capture happened before the cancellation.
        Status::Processing
        | Status::PartiallyCapturedAndCapturable
        | Status::PartiallyCapturedAndProcessing
        | Status::Review
        | Status::RequiresMerchantAction
        | Status::Conflicted
        | Status::CancelledPostCapture => Outcome::Indeterminate(
            Failure::new(
                error_codes::PAYMENT_PENDING,
                format!(
                    "Payment outcome is not final and funds may be held: {}",
                    payment.status
                ),
            )
            .with_parameters(outcome_parameters(payment)),
        ),

        // Terminal with nothing held — the only statuses where another card is the right next step.
        // `RequiresCustomerAction` included: its redirect is unreachable for a caller on a phone.
        Status::Failed
        | Status::Cancelled
        | Status::Expired
        | Status::RequiresPaymentMethod
        | Status::RequiresConfirmation
        | Status::RequiresCustomerAction => Outcome::Declined(decline_failure(payment)),
    }
}

/// A decline in the connector's own terms where it gave them, and in ours where it did not.
fn decline_failure(payment: &api_models::payments::PaymentsResponse) -> Failure {
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

    Failure::new(code, message).with_parameters(outcome_parameters(payment))
}

/// What a merchant needs to find this payment again — the only place a `payment_id` reaches the
/// provider when its own success field is null. An explicit list, never the request's own.
fn outcome_parameters(payment: &api_models::payments::PaymentsResponse) -> ResponseParameters {
    let mut parameters = ResponseParameters::from([
        (
            "payment_id".to_string(),
            payment.payment_id.get_string_repr().to_owned(),
        ),
        ("status".to_string(), payment.status.to_string()),
        (
            "amount_capturable".to_string(),
            payment.amount_capturable.get_amount_as_i64().to_string(),
        ),
    ]);

    // A missing value drops its key: Twilio raises a JSON mapping exception on a non-string, which
    // would break the whole response rather than this one field.
    let optional = [
        (
            "merchant_order_reference_id",
            payment.merchant_order_reference_id.clone(),
        ),
        ("connector", payment.connector.clone()),
        (
            "connector_transaction_id",
            payment.connector_transaction_id.clone(),
        ),
        ("unified_code", payment.unified_code.clone()),
        // Sent on a failure too: the card may have been kept where the money did not move.
        ("payment_method_id", payment.payment_method_id.clone()),
        (
            "amount_received",
            payment
                .amount_received
                .map(|amount| amount.get_amount_as_i64().to_string()),
        ),
    ];

    for (key, value) in optional {
        if let Some(value) = value {
            parameters.insert(key.to_string(), value);
        }
    }

    parameters
}
