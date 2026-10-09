use api_models::twilio_generic_pay::{
    TwilioGenericPayRequest, TwilioGenericPayResponse, TwilioPayMethod,
};
use common_utils::types::{AmountConvertor, StringMajorUnitForConnector};
use hyperswitch_domain_models::payments::HeaderPayload;
use router_env::{instrument, logger, tracing};

use crate::{
    core::{errors::RouterResponse, payments, utils as core_utils},
    routes::{app::ReqState, SessionState},
    services::{self, authentication::AuthenticationData, ApplicationResponse},
    types::api,
};

mod error_codes {
    pub const TOKENIZE_UNSUPPORTED: &str = "TOKENIZE_UNSUPPORTED";
    pub const BANK_ACCOUNT_UNSUPPORTED: &str = "BANK_ACCOUNT_UNSUPPORTED";
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
    pub const PROFILE_NOT_FOUND: &str = "PROFILE_NOT_FOUND";
    pub const PAYMENT_NOT_COMPLETED: &str = "PAYMENT_NOT_COMPLETED";
    pub const PROCESSING_ERROR: &str = "PROCESSING_ERROR";
}

struct TwilioCardDetails {
    card_number: cards::CardNumber,
    card_exp_month: hyperswitch_masking::Secret<String>,
    card_exp_year: hyperswitch_masking::Secret<String>,
    card_cvc: hyperswitch_masking::Secret<String>,
}

#[derive(Debug)]
struct ChargeFailure {
    code: &'static str,
    message: String,
}

impl ChargeFailure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[instrument(skip_all)]
pub async fn twilio_generic_pay_charge(
    state: SessionState,
    req_state: ReqState,
    auth: AuthenticationData,
    request: TwilioGenericPayRequest,
) -> RouterResponse<TwilioGenericPayResponse> {
    let transaction_id = request.transaction_id.clone();

    // The endpoint is merchant level, so the profile comes from the IVR's `<Parameter>` values.
    // When it is absent the payment flow falls back to the merchant's default profile.
    let profile_id = match resolve_profile_id(&state, &auth, &request).await {
        Ok(profile_id) => profile_id,
        Err(failure) => {
            logger::warn!(
                transaction_id = %transaction_id,
                error_code = failure.code,
                "Twilio generic pay request failed: {}",
                failure.message
            );
            return Ok(ApplicationResponse::Json(
                TwilioGenericPayResponse::failure(failure.code.to_string(), failure.message),
            ));
        }
    };

    let payments_request = match build_payments_request(&request, profile_id.clone()) {
        Ok(payments_request) => payments_request,
        Err(failure) => {
            logger::warn!(
                transaction_id = %transaction_id,
                error_code = failure.code,
                "Twilio generic pay request failed: {}",
                failure.message
            );
            return Ok(ApplicationResponse::Json(
                TwilioGenericPayResponse::failure(failure.code.to_string(), failure.message),
            ));
        }
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
                transaction_id = %transaction_id,
                "Unexpected response variant from payments core: {other:?}"
            );
            return Ok(ApplicationResponse::Json(
                TwilioGenericPayResponse::failure(
                    error_codes::PROCESSING_ERROR.to_string(),
                    "Payment could not be processed".to_string(),
                ),
            ));
        }
        // Deliberately answered with a 200 rather than propagated, so the IVR receives a
        // readable `error_code` instead of an opaque 5xx. A failure we cannot classify is reported
        // as a decline and investigated from the logs. This does not affect Twilio's retries
        // either way: those fire only when we fail to respond inside ten seconds, and an error
        // response delivered promptly is still a response.
        Err(error) => {
            logger::error!(
                transaction_id = %transaction_id,
                "Payments core returned an error for a Twilio generic pay charge: {error:?}"
            );
            return Ok(ApplicationResponse::Json(
                TwilioGenericPayResponse::failure(
                    error_codes::PROCESSING_ERROR.to_string(),
                    "Payment could not be processed".to_string(),
                ),
            ));
        }
    };

    Ok(ApplicationResponse::Json(build_twilio_response(&payment)))
}

async fn resolve_profile_id(
    state: &SessionState,
    auth: &AuthenticationData,
    request: &TwilioGenericPayRequest,
) -> Result<Option<common_utils::id_type::ProfileId>, ChargeFailure> {
    let Some(requested) = extract_profile_id(request)? else {
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
        logger::warn!("Twilio generic pay profile validation failed: {error:?}");
        ChargeFailure::new(
            error_codes::PROFILE_NOT_FOUND,
            format!(
                "profile_id {} is not available for this merchant",
                requested.get_string_repr()
            ),
        )
    })
    .map(|profile| profile.map(|profile| profile.get_id().clone()))
}

/// Reads `parameters.profile_id`, when the IVR sent one.
fn extract_profile_id(
    request: &TwilioGenericPayRequest,
) -> Result<Option<common_utils::id_type::ProfileId>, ChargeFailure> {
    let Some(profile_id) = request
        .parameters
        .as_ref()
        .and_then(|parameters| parameters.get("profile_id"))
    else {
        return Ok(None);
    };

    common_utils::id_type::ProfileId::try_from(std::borrow::Cow::Owned(profile_id.clone()))
        .map(Some)
        .map_err(|error| {
            ChargeFailure::new(
                error_codes::INVALID_REQUEST,
                format!("parameters.profile_id is not a valid profile id: {error}"),
            )
        })
}

/// Validates the Twilio request and maps it onto a create-and-confirm payment.
fn build_payments_request(
    request: &TwilioGenericPayRequest,
    profile_id: Option<common_utils::id_type::ProfileId>,
) -> Result<api_models::payments::PaymentsRequest, ChargeFailure> {
    if request.method != TwilioPayMethod::Charge {
        return Err(ChargeFailure::new(
            error_codes::TOKENIZE_UNSUPPORTED,
            "Tokenization is not supported on this endpoint",
        ));
    }

    if request.bankaccountnumber.is_some() || request.routingnumber.is_some() {
        return Err(ChargeFailure::new(
            error_codes::BANK_ACCOUNT_UNSUPPORTED,
            "Bank account payments are not supported on this endpoint",
        ));
    }

    let card = extract_card(request)?;

    let currency = request.currency_code.ok_or_else(|| {
        ChargeFailure::new(error_codes::INVALID_REQUEST, "currency_code is required")
    })?;

    let amount = request
        .amount
        .clone()
        .ok_or_else(|| ChargeFailure::new(error_codes::INVALID_REQUEST, "amount is required"))?;

    let amount = StringMajorUnitForConnector
        .convert_back(amount, currency)
        .map_err(|error| {
            ChargeFailure::new(
                error_codes::INVALID_REQUEST,
                format!("amount could not be parsed: {error}"),
            )
        })?;

    let billing = request
        .postal_code
        .clone()
        .map(|zip| api_models::payments::Address {
            address: Some(api_models::payments::AddressDetails {
                zip: Some(zip),
                ..Default::default()
            }),
            ..Default::default()
        });

    let mut payments_request = api_models::payments::PaymentsRequest {
        amount: Some(amount.into()),
        currency: Some(currency),
        confirm: Some(true),
        capture_method: Some(common_enums::CaptureMethod::Automatic),
        authentication_type: Some(common_enums::AuthenticationType::NoThreeDs),
        // Connectors translate this into their MOTO / telephone-order indicator.
        payment_channel: Some(common_enums::PaymentChannel::TelephoneOrder),
        payment_method: Some(common_enums::PaymentMethod::Card),
        payment_method_data: Some(api_models::payments::PaymentMethodDataRequest {
            payment_method_data: Some(api_models::payments::PaymentMethodData::Card(
                api_models::payments::Card {
                    card_number: card.card_number,
                    card_exp_month: card.card_exp_month,
                    card_exp_year: card.card_exp_year,
                    card_cvc: card.card_cvc,
                    ..Default::default()
                },
            )),
            billing: billing.clone(),
        }),
        billing,
        description: request.description.clone(),
        merchant_order_reference_id: Some(request.transaction_id.clone()),
        customer_id: extract_customer_id(request),
        metadata: build_metadata(request),
        profile_id,
        // `connector` is left unset so the merchant's configured routing decides.
        ..Default::default()
    };

    payments_request
        .validate()
        .map_err(|message| ChargeFailure::new(error_codes::INVALID_REQUEST, message))?;

    crate::routes::payments::get_or_generate_payment_id(&mut payments_request).map_err(
        |error| {
            ChargeFailure::new(
                error_codes::INVALID_REQUEST,
                format!("payment_id could not be generated: {error}"),
            )
        },
    )?;

    Ok(payments_request)
}

fn extract_card(request: &TwilioGenericPayRequest) -> Result<TwilioCardDetails, ChargeFailure> {
    let missing = |field: &'static str| {
        ChargeFailure::new(
            error_codes::INVALID_REQUEST,
            format!("{field} is required for a card charge"),
        )
    };

    Ok(TwilioCardDetails {
        card_number: request
            .cardnumber
            .clone()
            .ok_or_else(|| missing("cardnumber"))?,
        card_exp_month: request
            .expiry_month
            .clone()
            .ok_or_else(|| missing("expiry_month"))?,
        card_exp_year: request
            .expiry_year
            .clone()
            .ok_or_else(|| missing("expiry_year"))?,
        card_cvc: request.cvv.clone().ok_or_else(|| missing("cvv"))?,
    })
}

fn extract_customer_id(
    request: &TwilioGenericPayRequest,
) -> Option<common_utils::id_type::CustomerId> {
    request
        .parameters
        .as_ref()?
        .get("customer_id")
        .and_then(|customer_id| {
            common_utils::id_type::CustomerId::try_from(std::borrow::Cow::Owned(
                customer_id.clone(),
            ))
            .inspect_err(|error| {
                logger::warn!("Ignoring unusable customer_id from Twilio parameters: {error:?}")
            })
            .ok()
        })
}

/// Carries the IVR's `<Parameter>` values through to the payment, for reconciliation.
fn build_metadata(request: &TwilioGenericPayRequest) -> Option<serde_json::Value> {
    let parameters = request.parameters.as_ref()?;
    if parameters.is_empty() {
        return None;
    }
    serde_json::to_value(parameters)
        .inspect_err(|error| {
            logger::warn!("Could not serialize Twilio parameters into metadata: {error:?}")
        })
        .ok()
}

/// Maps a finished payment onto the response Twilio expects.
fn build_twilio_response(
    payment: &api_models::payments::PaymentsResponse,
) -> TwilioGenericPayResponse {
    match payment.status {
        common_enums::IntentStatus::Succeeded | common_enums::IntentStatus::PartiallyCaptured => {
            TwilioGenericPayResponse::success(payment.payment_id.get_string_repr().to_owned())
        }
        // Everything else is reported as a failure. A phone call cannot wait out a `Processing`
        // or satisfy a `RequiresCustomerAction`, so neither is a result we can read to the caller.
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

            TwilioGenericPayResponse::failure(code, message)
        }
    }
}
