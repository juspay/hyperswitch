/// GET /v1/payments/{payment_id}/client
///
/// Returns a combined list of:
///   - merchant-enabled payment methods (filtered via Euclid constraint graph + session flow routing)
///   - customer saved payment methods (fetched from DB via `CustomerPaymentMethodsFetcher` trait)
///
/// Auth:
///   - `SdkAuthorizationAuth` via Authorization header.
///   - `PublishableKeyAuth` via `api-key: pk_...` and `client_secret` query param.
///
/// Merchant secret-key auth is not supported for this client endpoint.
use api_models::payment_methods::{
    ClientPaymentMethodsListResponse, CustomerPaymentMethod, CustomerPaymentMethodDataForClient,
    CustomerPaymentMethodForClient, MaskedBankDetails, PaymentMethodListIntentDataInput,
    ResponsePaymentMethodsEnabledForClient,
};
use common_utils::{consts, ext_traits::AsyncExt, generate_id, id_type};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing, Flow};

use crate::{
    core::{
        blocklist::utils as blocklist_utils,
        configs::dimension_state,
        errors::{self, StorageErrorExt},
        payment_methods::{
            cards, transformers::list_customer_payment_methods_from_modular_service,
        },
        payments::helpers,
        utils as core_utils,
    },
    pii::PeekInterface,
    routes::{self, payment_methods::ParentPaymentMethodToken},
    services,
    types::{
        domain::{self, Profile},
        storage::{self, BankDebitTokenData, PaymentTokenData},
    },
};

/// How far ahead of the payment tokens a token pin is expired.
///
/// A pin that outlived the token it names would hand out a token the token store has already
/// forgotten. Both are written within the same build, so a small margin is enough.
const PIN_EXPIRY_MARGIN_IN_SECS: i64 = 5;

// ---------------------------------------------------------------------------
// Trait: CustomerPaymentMethodsFetcher
// ---------------------------------------------------------------------------

/// Hands back the payment token this payment has already agreed on for `payment_method_id`,
/// pinning `payment_token` as that token when this is the first call to get there.
///
/// The token is the one part of a listing that is newly generated on every call; pinning it —
/// rather than caching the built listing — is what lets two calls for a payment agree without
/// ever serving a list that outlived the configuration it was built from.
///
/// The pin is a single writer: whichever call stores its token first wins, and a call racing it
/// reads that token back instead of its own, so concurrent callers answer with the same token.
/// Both tokens resolve to the same saved payment method, so the one that loses is simply unused.
///
/// The pin expires a little before the token itself does, so it can never name a token the token
/// store has already forgotten. Without a payment there is nothing to pin against, and the freshly
/// generated token is returned unchanged.
async fn pinned_payment_token(
    state: &routes::SessionState,
    payment_intent: Option<&storage::PaymentIntent>,
    payment_method_id: &str,
    payment_token: String,
    intent_fulfillment_time: i64,
) -> String {
    match payment_intent {
        None => payment_token,
        Some(payment_intent) => {
            let redis_key = payment_intent
                .payment_id
                .get_pm_token_redis_key(&payment_intent.processor_merchant_id, payment_method_id);

            core_utils::pin_value(
                state,
                &redis_key,
                "PinnedPaymentToken",
                payment_token,
                intent_fulfillment_time.saturating_sub(PIN_EXPIRY_MARGIN_IN_SECS),
            )
            .await
        }
    }
}

/// Abstraction over saved-PM retrieval — allows future swap to a remote PM service.
#[async_trait::async_trait]
pub trait CustomerPaymentMethodsFetcher: Send + Sync {
    async fn fetch(
        &self,
        state: &routes::SessionState,
        platform: &domain::Platform,
        payment_intent: Option<&storage::PaymentIntent>,
        payment_attempt: Option<&storage::PaymentAttempt>,
        customer: &domain::Customer,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> errors::RouterResult<Vec<CustomerPaymentMethodForClient>>;
}

fn bank_redirect_data_for_client(
    payment_method: common_enums::PaymentMethod,
    bank: Option<MaskedBankDetails>,
) -> Option<CustomerPaymentMethodDataForClient> {
    if payment_method == common_enums::PaymentMethod::BankRedirect {
        bank.map(CustomerPaymentMethodDataForClient::BankRedirect)
    } else {
        None
    }
}

/// Convert a legacy `CustomerPaymentMethod` into the slimmer client-facing type.
fn to_client_pm(pm: CustomerPaymentMethod) -> CustomerPaymentMethodForClient {
    let payment_method_data = pm
        .card
        .map(|card| CustomerPaymentMethodDataForClient::Card(Box::new(card)))
        .or_else(|| pm.wallets.map(CustomerPaymentMethodDataForClient::Wallet))
        .or_else(|| bank_redirect_data_for_client(pm.payment_method, pm.bank));

    CustomerPaymentMethodForClient {
        payment_token: pm.payment_token,
        payment_method: pm.payment_method,
        payment_method_type: pm.payment_method_type,
        default_payment_method_set: pm.default_payment_method_set,
        requires_cvv: pm.requires_cvv,
        recurring_enabled: pm.recurring_enabled,
        created: pm.created,
        last_used_at: pm.last_used_at,
        payment_method_data,
    }
}

/// DB-backed implementation — delegates to `cards::list_customer_payment_method`.
///
/// `intent_fulfillment_time` is the window `cards::list_customer_payment_method` stores its
/// tokens for; it is carried here so a token pin cannot outlive the token it names.
pub struct DbCustomerPaymentMethodsFetcher {
    pub intent_fulfillment_time: i64,
}

#[async_trait::async_trait]
impl CustomerPaymentMethodsFetcher for DbCustomerPaymentMethodsFetcher {
    async fn fetch(
        &self,
        state: &routes::SessionState,
        platform: &domain::Platform,
        payment_intent: Option<&storage::PaymentIntent>,
        payment_attempt: Option<&storage::PaymentAttempt>,
        customer: &domain::Customer,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> errors::RouterResult<Vec<CustomerPaymentMethodForClient>> {
        let candidates = Box::pin(cards::fetch_customer_payment_method_candidates(
            state,
            platform,
            payment_intent,
            payment_attempt,
            customer.get_id(),
            None, // limit
            dimensions,
        ))
        .await
        .attach_printable("Failed to fetch saved payment method candidates for client")?;

        let response_body = candidates
            .into_response(state, platform, payment_intent, payment_attempt)
            .await?;

        let intent_fulfillment_time = self.intent_fulfillment_time;

        Ok(
            futures::future::join_all(response_body.customer_payment_methods.into_iter().map(
                |payment_method| async move {
                    let payment_token = pinned_payment_token(
                        state,
                        payment_intent,
                        &payment_method.payment_method_id,
                        payment_method.payment_token,
                        intent_fulfillment_time,
                    )
                    .await;

                    to_client_pm(CustomerPaymentMethod {
                        payment_token,
                        ..payment_method
                    })
                },
            ))
            .await,
        )
    }
}

// ---------------------------------------------------------------------------
// ModularCustomerPaymentMethodsFetcher
// ---------------------------------------------------------------------------

/// Context derived from `PaymentIntentContext` needed to fetch and map modular-service PMs.
pub struct ModularCustomerPaymentMethodsFetcher {
    pub profile_id: id_type::ProfileId,
    pub intent_fulfillment_time: i64,
    pub off_session_payment_flag: bool,
    pub is_connector_agnostic_mit_enabled: bool,
}

impl ModularCustomerPaymentMethodsFetcher {
    /// Generate a short-lived Redis payment token for a modular-service PM and persist it.
    async fn store_payment_token_in_redis(
        state: &routes::SessionState,
        payment_method: common_enums::PaymentMethod,
        pm_id: String,
        intent_fulfillment_time: i64,
    ) -> errors::RouterResult<String> {
        let payment_token = generate_id(consts::ID_LENGTH, "token");

        // Build the PaymentTokenData variant that matches the payment method type,
        // mirroring the logic in `get_pm_list_context`.
        let token_data: PaymentTokenData = match payment_method {
            common_enums::PaymentMethod::Card => {
                PaymentTokenData::permanent_card(Some(pm_id.clone()), None, pm_id.clone(), None)
            }
            common_enums::PaymentMethod::Wallet => PaymentTokenData::wallet_token(pm_id.clone()),
            common_enums::PaymentMethod::BankDebit => {
                PaymentTokenData::BankDebit(BankDebitTokenData {
                    payment_method_id: pm_id.clone(),
                    locker_id: None,
                })
            }
            // Fallback for PM types that don't have a specific PaymentTokenData variant.
            _ => PaymentTokenData::temporary_generic(generate_id(consts::ID_LENGTH, "token")),
        };

        // Persist the token → PaymentTokenData mapping in Redis so that when the
        // customer submits the payment the token can be resolved back to the PM.
        ParentPaymentMethodToken::create_key_for_token((&payment_token, payment_method))
            .insert(intent_fulfillment_time, token_data, state)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to store payment token in Redis")?;

        Ok(payment_token)
    }
}

#[async_trait::async_trait]
impl CustomerPaymentMethodsFetcher for ModularCustomerPaymentMethodsFetcher {
    async fn fetch(
        &self,
        state: &routes::SessionState,
        platform: &domain::Platform,
        payment_intent: Option<&storage::PaymentIntent>,
        _payment_attempt: Option<&storage::PaymentAttempt>,
        customer: &domain::Customer,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> errors::RouterResult<Vec<CustomerPaymentMethodForClient>> {
        let merchant_id = platform.get_processor().get_processor_merchant_id();
        let id = customer.get_global_id().cloned().ok_or(
            errors::ApiErrorResponse::MissingRequiredField {
                field_name: "id".into(),
            },
        )?;

        let items = list_customer_payment_methods_from_modular_service(
            state,
            &merchant_id,
            &self.profile_id,
            id,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

        // Base requires_cvv from merchant/superposition config (same as legacy PM list).
        let requires_cvv_base = dimensions
            .get_requires_cvv(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                Some(customer.get_id()),
            )
            .await;

        // Fetch enabled MCAs for the merchant so we can check whether the connector that
        // issued a mandate token is still enabled for this profile — mirroring the
        // `get_mca_status` check performed in the legacy DB flow.
        let merchant_connector_accounts = state
            .store
            .list_enabled_merchant_connector_accounts_without_encrypted_by_merchant_id_profile_id(
                &merchant_id,
                &self.profile_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: merchant_id.get_string_repr().to_owned(),
            })?;

        let active_mca_ids: std::collections::HashSet<id_type::MerchantConnectorAccountId> =
            merchant_connector_accounts
                .iter()
                .map(|mca| mca.get_id())
                .collect();

        let mut customer_payment_methods = Vec::with_capacity(items.len());

        for pm in items {
            // Replicate `get_mca_status` from the legacy DB flow:
            //   - agnostic MIT: network_transaction_id is present and the feature is enabled
            //   - mandate match: at least one Active connector token belongs to an active MCA
            //     for this profile (equivalent to MCA id present in connector_mandate_details)
            let has_active_connector_token = pm.connector_tokens.as_ref().is_some_and(|tokens| {
                tokens.iter().any(|t| {
                    t.status == common_enums::ConnectorTokenStatus::Active
                        && active_mca_ids.contains(&t.connector_id)
                })
            });
            let agnostic_mit =
                self.is_connector_agnostic_mit_enabled && pm.network_transaction_id.is_some();
            let recurring_enabled = agnostic_mit || has_active_connector_token;

            // requires_cvv mirrors the legacy logic (cards.rs list_customer_payment_method):
            // CVV is skipped when this is an off-session payment and the PM has a usable
            // mandate or network transaction ID.
            let requires_cvv = if self.is_connector_agnostic_mit_enabled {
                requires_cvv_base
                    && !(self.off_session_payment_flag
                        && (pm.connector_tokens.is_some() || pm.network_transaction_id.is_some()))
            } else {
                requires_cvv_base
                    && !(self.off_session_payment_flag && pm.connector_tokens.is_some())
            };

            let payment_token = Self::store_payment_token_in_redis(
                state,
                pm.payment_method_type,
                pm.id.clone(),
                self.intent_fulfillment_time,
            )
            .await?;
            let payment_token = pinned_payment_token(
                state,
                payment_intent,
                &pm.id,
                payment_token,
                self.intent_fulfillment_time,
            )
            .await;

            // Build the client-facing response item.
            let payment_method_data = pm.payment_method_data.and_then(|d| d.into());

            // Mirror the legacy `list_customer_payment_method` filter: skip PMs where
            // CVV is not required (off-session + has tokens → CVV skipped) AND no
            // active mandate exists. Such PMs are unusable and should not be surfaced.
            if requires_cvv || recurring_enabled {
                customer_payment_methods.push(CustomerPaymentMethodForClient {
                    payment_token,
                    payment_method: pm.payment_method_type,
                    payment_method_type: Some(pm.payment_method_subtype),
                    default_payment_method_set: pm.is_default,
                    requires_cvv,
                    recurring_enabled: Some(recurring_enabled),
                    created: Some(pm.created),
                    last_used_at: Some(pm.last_used_at),
                    payment_method_data,
                });
            }
        }

        Ok(customer_payment_methods)
    }
}

// ---------------------------------------------------------------------------
// Internal context struct
// ---------------------------------------------------------------------------

struct PaymentIntentContext {
    payment_intent: storage::PaymentIntent,
    payment_attempt: storage::PaymentAttempt,
    business_profile: Profile,
    is_cit_transaction: bool,
    billing_address: Option<domain::Address>,
    shipping_address: Option<domain::Address>,
    customer: Option<hyperswitch_domain_models::customer::Customer>,
}

async fn load_payment_intent_context(
    state: &routes::SessionState,
    platform: &domain::Platform,
    payment_id: &id_type::PaymentId,
) -> errors::RouterResult<PaymentIntentContext> {
    let db = &*state.store;

    let payment_intent = db
        .find_payment_intent_by_payment_id_processor_merchant_id(
            payment_id,
            &platform.get_processor().get_processor_merchant_id(),
            platform.get_processor().get_key_store(),
            platform.get_processor().get_storage_scheme(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let payment_attempt = db
        .find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
            &payment_intent.payment_id,
            &domain::ProcessorMerchantId::from(payment_intent.processor_merchant_id.clone()),
            &payment_intent.active_attempt.get_id(),
            platform.get_processor().get_storage_scheme(),
            platform.get_processor().get_key_store(),
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

    let profile_id = payment_intent
        .profile_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("'profile_id' not set in payment intent")?;

    let business_profile = db
        .find_business_profile_by_profile_id(platform.get_processor().get_key_store(), &profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let shipping_address = helpers::get_address_by_id(
        state,
        payment_intent.shipping_address_id.clone(),
        platform.get_processor().get_key_store(),
        &payment_intent.payment_id,
        platform.get_processor().get_account().get_id(),
        platform.get_processor().get_account().storage_scheme,
    )
    .await?;

    let billing_address = helpers::get_address_by_id(
        state,
        payment_intent.billing_address_id.clone(),
        platform.get_processor().get_key_store(),
        &payment_intent.payment_id,
        platform.get_processor().get_account().get_id(),
        platform.get_processor().get_account().storage_scheme,
    )
    .await?;

    let customer = payment_intent
        .customer_id
        .as_ref()
        .async_and_then(|customer_id| async {
            let use_merchant_reference_id =
                helpers::should_use_merchant_reference_id_as_customer_id_for_provider(
                    state,
                    platform.get_provider(),
                    Some(customer_id),
                )
                .await;
            db.find_customer_by_customer_id_merchant_id(
                customer_id,
                &payment_intent.merchant_id,
                platform.get_provider().get_key_store(),
                platform.get_provider().get_account().storage_scheme,
                use_merchant_reference_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
            .ok()
        })
        .await;

    let setup_future_usage = payment_intent.setup_future_usage;
    let is_cit_transaction = payment_attempt.mandate_details.is_some()
        || setup_future_usage
            .map(|future_usage| future_usage == common_enums::FutureUsage::OffSession)
            .unwrap_or(false);

    Ok(PaymentIntentContext {
        payment_intent,
        payment_attempt,
        business_profile,
        is_cit_transaction,
        billing_address,
        shipping_address,
        customer,
    })
}

// ---------------------------------------------------------------------------
// fetch_enabled_payment_methods  — Gate 1 + Gate 2 + hashmap building
// ---------------------------------------------------------------------------

struct EnabledPmsResult {
    payment_methods_enabled: Vec<ResponsePaymentMethodsEnabledForClient>,
    sdk_next_action: api_models::payments::SdkNextAction,
    connector_supports_installments: bool,
}

async fn fetch_enabled_payment_methods(
    state: &routes::SessionState,
    platform: &domain::Platform,
    payment_intent_context: &PaymentIntentContext,
) -> errors::RouterResult<EnabledPmsResult> {
    let merchant_enabled_pms_context = Box::pin(cards::build_merchant_enabled_pms_context(
        state,
        platform,
        &payment_intent_context.business_profile,
        Some(&payment_intent_context.payment_intent),
        Some(&payment_intent_context.payment_attempt),
        payment_intent_context.billing_address.as_ref(),
        payment_intent_context.shipping_address.as_ref(),
        payment_intent_context.customer.as_ref(),
        payment_intent_context.is_cit_transaction,
    ))
    .await?;

    let customer_acceptance_support_config = &state.conf.customer_acceptance_support;

    let mut flat_pms = merchant_enabled_pms_context
        .payment_experience_pms_for_client(customer_acceptance_support_config);
    flat_pms.extend(
        merchant_enabled_pms_context
            .card_network_pms_for_client(customer_acceptance_support_config),
    );
    flat_pms.extend(merchant_enabled_pms_context.bank_redirect_pms_for_client(state)?);
    flat_pms.extend(
        merchant_enabled_pms_context.bank_debit_pms_for_client(customer_acceptance_support_config),
    );
    flat_pms.extend(
        merchant_enabled_pms_context
            .bank_transfer_pms_for_client(customer_acceptance_support_config),
    );

    Ok(EnabledPmsResult {
        payment_methods_enabled: flat_pms,
        sdk_next_action: merchant_enabled_pms_context.sdk_next_action,
        connector_supports_installments: merchant_enabled_pms_context
            .connector_supports_installments,
    })
}

// ---------------------------------------------------------------------------
// fetch_customer_payment_methods  — saved PMs for the payment's customer
// ---------------------------------------------------------------------------

async fn filter_customer_payment_methods<F, Fut>(
    customer_pms: Vec<CustomerPaymentMethodForClient>,
    enabled: &[ResponsePaymentMethodsEnabledForClient],
    filter_blocklisted: F,
) -> Vec<CustomerPaymentMethodForClient>
where
    F: FnOnce(Vec<CustomerPaymentMethodForClient>) -> Fut,
    Fut: std::future::Future<Output = Vec<CustomerPaymentMethodForClient>>,
{
    // 4. Filter customer PMs to only those whose (payment_method, payment_method_type)
    //    combination is present in the merchant-enabled list.
    let customer_pms = filter_customer_pms_by_enabled(customer_pms, enabled);
    // 5. Drop saved cards whose BIN is blocklisted for this merchant/profile (no-op unless
    //    the merchant has the blocklist guard enabled).
    let customer_pms = filter_blocklisted(customer_pms).await;
    cards::filter_latest_wallet_methods(customer_pms, |pm| {
        (pm.payment_method, pm.payment_method_type, pm.last_used_at)
    })
}

/// Filter `customer_payment_methods` to only those whose `(payment_method, payment_method_type)`
/// combination is present in `payment_methods_enabled`.
fn filter_customer_pms_by_enabled(
    customer_pms: Vec<CustomerPaymentMethodForClient>,
    enabled: &[ResponsePaymentMethodsEnabledForClient],
) -> Vec<CustomerPaymentMethodForClient> {
    customer_pms
        .into_iter()
        .filter(|customer_payment_method| {
            enabled.iter().any(|enabled_payment_method| {
                enabled_payment_method.payment_method == customer_payment_method.payment_method
                    && match customer_payment_method.payment_method_type {
                        Some(payment_method_type) => {
                            enabled_payment_method.payment_method_type == payment_method_type
                        }
                        // Cards may have no subtype (determined by BIN lookup at payment time).
                        // Allow them through if the merchant has any card subtype enabled.
                        None if customer_payment_method.payment_method
                            == common_enums::PaymentMethod::Card =>
                        {
                            true
                        }
                        None => false,
                    }
            })
        })
        .collect()
}

/// Filter out saved cards whose BIN (`card_isin`) has an active blocklist entry for this
/// merchant/profile (merchant-wide entries with a NULL `profile_id` match every profile).
/// Runs only when the merchant has enabled the blocklist guard (the same
/// `payments.payment_blocklist_guard` config that gates confirm-time and eligibility-time
/// blocklist checks). BIN lookups are
/// deduplicated across the list and run concurrently. Non-card payment methods and cards
/// without a stored `card_isin` are passed through unchanged — fingerprint-level (exact
/// card) blocklist entries cannot be evaluated at list time since the stored record does
/// not carry a blocklist-comparable fingerprint.
async fn filter_customer_pms_by_blocklist(
    state: &routes::SessionState,
    platform: &domain::Platform,
    profile_id: &id_type::ProfileId,
    customer_pms: Vec<CustomerPaymentMethodForClient>,
) -> Vec<CustomerPaymentMethodForClient> {
    let processor = platform.get_processor();
    let guard_enabled = dimension_state::Dimensions::new()
        .with_processor_merchant_id(processor.get_processor_merchant_id())
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
        .with_profile_id(profile_id.clone())
        .get_payment_blocklist_guard(
            state.store.as_ref(),
            state.superposition_service.as_ref(),
            None,
        )
        .await;

    let bins: std::collections::HashSet<String> = if guard_enabled {
        customer_pms
            .iter()
            .filter_map(
                |customer_pm| match customer_pm.payment_method_data.as_ref() {
                    Some(CustomerPaymentMethodDataForClient::Card(card)) => card.card_isin.clone(),
                    _ => None,
                },
            )
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    let blocked_bins = if bins.is_empty() {
        std::collections::HashSet::new()
    } else {
        blocklist_utils::get_blocked_bins(
            state,
            processor,
            common_enums::BlocklistTransactionType::Payment,
            profile_id,
            bins,
        )
        .await
    };

    filter_customer_pms_by_blocked_bins(customer_pms, &blocked_bins)
}

fn filter_customer_pms_by_blocked_bins(
    customer_pms: Vec<CustomerPaymentMethodForClient>,
    blocked_bins: &std::collections::HashSet<String>,
) -> Vec<CustomerPaymentMethodForClient> {
    if blocked_bins.is_empty() {
        customer_pms
    } else {
        let initial_count = customer_pms.len();
        let filtered: Vec<CustomerPaymentMethodForClient> = customer_pms
            .into_iter()
            .filter(
                |customer_pm| match customer_pm.payment_method_data.as_ref() {
                    Some(CustomerPaymentMethodDataForClient::Card(card)) => !card
                        .card_isin
                        .as_ref()
                        .is_some_and(|card_isin| blocked_bins.contains(card_isin)),
                    _ => true,
                },
            )
            .collect();
        logger::info!(
            filtered_out = initial_count - filtered.len(),
            "Filtered blocklisted saved cards from the payment method list"
        );
        filtered
    }
}

async fn fetch_customer_payment_methods(
    state: &routes::SessionState,
    platform: &domain::Platform,
    payment_intent_context: &PaymentIntentContext,
) -> errors::RouterResult<Vec<CustomerPaymentMethodForClient>> {
    match (
        payment_intent_context.payment_intent.customer_id.as_ref(),
        payment_intent_context.customer.as_ref(),
    ) {
        (None, _) => Ok(vec![]),
        (Some(_), None) => Err(errors::ApiErrorResponse::CustomerNotFound.into()),
        (Some(_), Some(customer)) => {
            let dimensions = dimension_state::Dimensions::new()
                .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
                .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id());

            let feature_config =
                crate::core::utils::get_feature_config(state, platform, &dimensions).await;

            if feature_config.is_payment_method_modular_allowed {
                logger::info!("Fetching customer payment methods from modular service");

                let profile_id = payment_intent_context
                    .payment_intent
                    .profile_id
                    .clone()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("'profile_id' not set in payment intent")?;

                let intent_fulfillment_time = payment_intent_context
                    .business_profile
                    .get_order_fulfillment_time()
                    .unwrap_or(consts::DEFAULT_INTENT_FULFILLMENT_TIME);

                let off_session_payment_flag = payment_intent_context
                    .payment_intent
                    .setup_future_usage
                    .map(|future_usage| future_usage == common_enums::FutureUsage::OffSession)
                    .unwrap_or(false);

                let is_connector_agnostic_mit_enabled = payment_intent_context
                    .business_profile
                    .is_connector_agnostic_mit_enabled
                    .unwrap_or(false);

                ModularCustomerPaymentMethodsFetcher {
                    profile_id,
                    intent_fulfillment_time,
                    off_session_payment_flag,
                    is_connector_agnostic_mit_enabled,
                }
                .fetch(
                    state,
                    platform,
                    Some(&payment_intent_context.payment_intent),
                    Some(&payment_intent_context.payment_attempt),
                    customer,
                    &dimensions,
                )
                .await
            } else {
                logger::info!("Fetching customer payment methods from DB");
                DbCustomerPaymentMethodsFetcher {
                    intent_fulfillment_time: payment_intent_context
                        .business_profile
                        .get_order_fulfillment_time()
                        .unwrap_or(consts::DEFAULT_INTENT_FULFILLMENT_TIME),
                }
                .fetch(
                    state,
                    platform,
                    Some(&payment_intent_context.payment_intent),
                    Some(&payment_intent_context.payment_attempt),
                    customer,
                    &dimensions,
                )
                .await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// list_payment_methods_client  — top-level orchestrator
// ---------------------------------------------------------------------------

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsList))]
pub async fn list_payment_methods_client(
    state: routes::SessionState,
    platform: domain::Platform,
    payment_id: id_type::PaymentId,
    client_secret: Option<String>,
) -> errors::RouterResponse<ClientPaymentMethodsListResponse> {
    // 1. Load payment intent + related context
    let payment_intent_context =
        load_payment_intent_context(&state, &platform, &payment_id).await?;

    if let Some(client_secret) = client_secret.as_ref() {
        helpers::authenticate_client_secret(
            Some(client_secret),
            &payment_intent_context.payment_intent,
        )?;
    }

    // 2. Fetch enabled payment methods (Gate 1 + Gate 2 + consolidation)
    let EnabledPmsResult {
        payment_methods_enabled,
        sdk_next_action,
        connector_supports_installments,
    } = fetch_enabled_payment_methods(&state, &platform, &payment_intent_context).await?;

    // 3. Fetch saved customer payment methods
    let customer_payment_methods =
        fetch_customer_payment_methods(&state, &platform, &payment_intent_context).await?;

    let customer_payment_methods_filtered = filter_customer_payment_methods(
        customer_payment_methods,
        &payment_methods_enabled,
        |customer_pms| {
            filter_customer_pms_by_blocklist(
                &state,
                &platform,
                payment_intent_context.business_profile.get_id(),
                customer_pms,
            )
        },
    )
    .await;

    // 6. Build intent_data
    let net_amount = payment_intent_context
        .payment_attempt
        .net_amount
        .get_total_amount();
    let payment_type = Some(
        payment_intent_context
            .payment_attempt
            .infer_payment_type(payment_intent_context.is_cit_transaction),
    );

    let intent_data_input = PaymentMethodListIntentDataInput {
        merchant_name: platform
            .get_processor()
            .get_account()
            .merchant_name
            .as_ref()
            .map(|merchant_name| merchant_name.clone().into_inner().peek().clone()),
        mandate_payment: payment_intent_context
            .payment_attempt
            .mandate_details
            .as_ref()
            .map(|mandate_details| mandate_details.to_api_mandate_data()),
        payment_type,
        capture_method: payment_intent_context.payment_attempt.capture_method,
    };

    let intent_data = payment_intent_context
        .payment_intent
        .clone()
        .into_payment_method_list_intent_data(
            net_amount,
            connector_supports_installments,
            intent_data_input,
            &payment_intent_context.business_profile,
            payment_intent_context.customer.as_ref(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to build intent_data")?;

    Ok(services::ApplicationResponse::Json(
        ClientPaymentMethodsListResponse {
            payment_methods_enabled,
            customer_payment_methods: customer_payment_methods_filtered,
            sdk_next_action,
            intent_data,
        },
    ))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, future::ready};

    use api_models::payment_methods::CardDetailFromLocker;
    use common_enums::{PaymentMethod, PaymentMethodType};

    use super::{
        cards, filter_customer_payment_methods, filter_customer_pms_by_blocked_bins, to_client_pm,
        CustomerPaymentMethod, CustomerPaymentMethodDataForClient, CustomerPaymentMethodForClient,
        ResponsePaymentMethodsEnabledForClient,
    };

    fn saved_method(
        token: &str,
        payment_method: PaymentMethod,
        payment_method_type: Option<PaymentMethodType>,
        days: Option<i64>,
    ) -> CustomerPaymentMethodForClient {
        CustomerPaymentMethodForClient {
            payment_token: token.to_owned(),
            payment_method,
            payment_method_type,
            default_payment_method_set: false,
            requires_cvv: false,
            recurring_enabled: Some(true),
            created: None,
            last_used_at: days.map(|days| time::Date::MIN.midnight() + time::Duration::days(days)),
            payment_method_data: None,
        }
    }

    fn saved_wallet(
        token: &str,
        subtype: PaymentMethodType,
        days: Option<i64>,
    ) -> CustomerPaymentMethodForClient {
        saved_method(token, PaymentMethod::Wallet, Some(subtype), days)
    }

    fn enabled_method(
        method: PaymentMethod,
        subtype: PaymentMethodType,
    ) -> ResponsePaymentMethodsEnabledForClient {
        ResponsePaymentMethodsEnabledForClient {
            payment_method: method,
            payment_method_type: subtype,
            data: None,
            payment_experience: None,
            collect_shipping_details_from_wallets: None,
            collect_billing_details_from_wallets: None,
            customer_acceptance_support: Default::default(),
        }
    }

    fn saved_card_data(card_isin: Option<&str>) -> CustomerPaymentMethodDataForClient {
        CustomerPaymentMethodDataForClient::Card(Box::new(CardDetailFromLocker {
            scheme: None,
            issuer_country: None,
            issuer_country_code: None,
            last4_digits: None,
            card_number: None,
            expiry_month: None,
            expiry_year: None,
            card_token: None,
            card_holder_name: None,
            card_fingerprint: None,
            nick_name: None,
            card_network: None,
            card_isin: card_isin.map(str::to_owned),
            card_issuer: None,
            card_type: None,
            card_subtype: None,
            card_segment_type: None,
            funding_source: None,
            saved_to_locker: true,
        }))
    }

    fn enabled_wallets() -> Vec<ResponsePaymentMethodsEnabledForClient> {
        [
            PaymentMethodType::ApplePay,
            PaymentMethodType::GooglePay,
            PaymentMethodType::Paypal,
        ]
        .into_iter()
        .map(|subtype| enabled_method(PaymentMethod::Wallet, subtype))
        .collect()
    }

    fn legacy_method(method: CustomerPaymentMethodForClient) -> CustomerPaymentMethod {
        let (card, wallets, bank) = match method.payment_method_data {
            Some(CustomerPaymentMethodDataForClient::Card(card)) => (Some(*card), None, None),
            Some(CustomerPaymentMethodDataForClient::Wallet(wallet)) => (None, Some(wallet), None),
            Some(CustomerPaymentMethodDataForClient::BankRedirect(bank)) => {
                (None, None, Some(bank))
            }
            Some(CustomerPaymentMethodDataForClient::BankDebit(_)) | None => (None, None, None),
        };
        CustomerPaymentMethod {
            payment_method_id: method.payment_token.clone(),
            payment_token: method.payment_token,
            customer_id: Default::default(),
            payment_method: method.payment_method,
            payment_method_type: method.payment_method_type,
            payment_method_issuer: None,
            payment_method_issuer_code: None,
            recurring_enabled: method.recurring_enabled,
            installment_payment_enabled: None,
            payment_experience: None,
            card,
            wallets,
            metadata: None,
            created: method.created,
            #[cfg(feature = "payouts")]
            bank_transfer: None,
            bank,
            surcharge_details: None,
            requires_cvv: method.requires_cvv,
            last_used_at: method.last_used_at,
            default_payment_method_set: method.default_payment_method_set,
            billing: None,
        }
    }

    #[tokio::test]
    async fn customer_pm_filtering_keeps_latest_of_ten_apple_pay_methods() {
        let mut methods: Vec<_> = (0..10)
            .map(|days| {
                saved_wallet(
                    &format!("apple_{days}"),
                    PaymentMethodType::ApplePay,
                    Some(days),
                )
            })
            .collect();
        methods.rotate_left(4);
        let expected = saved_wallet("apple_9", PaymentMethodType::ApplePay, Some(9));
        let result = filter_customer_payment_methods(methods, &enabled_wallets(), ready).await;
        assert_eq!(result, vec![expected]);
    }

    #[tokio::test]
    async fn customer_pm_filtering_matches_legacy_and_modular_candidates() {
        let mut blocked = saved_method(
            "blocked_card",
            PaymentMethod::Card,
            Some(PaymentMethodType::Credit),
            Some(5),
        );
        blocked.payment_method_data = Some(saved_card_data(Some("411111")));
        let allowed_card = saved_method("allowed_card", PaymentMethod::Card, None, Some(1));
        let paypal = saved_wallet("paypal", PaymentMethodType::Paypal, Some(2));
        let mut latest_apple = saved_wallet("apple_new", PaymentMethodType::ApplePay, Some(3));
        latest_apple.default_payment_method_set = true;
        latest_apple.created = latest_apple.last_used_at;
        let methods = vec![
            blocked,
            saved_wallet("apple_old", PaymentMethodType::ApplePay, Some(1)),
            allowed_card.clone(),
            latest_apple.clone(),
            saved_wallet("disabled_google", PaymentMethodType::GooglePay, Some(6)),
            paypal.clone(),
        ];
        let enabled = vec![
            enabled_method(PaymentMethod::Card, PaymentMethodType::Credit),
            enabled_method(PaymentMethod::Wallet, PaymentMethodType::ApplePay),
            enabled_method(PaymentMethod::Wallet, PaymentMethodType::Paypal),
        ];
        let legacy_methods = methods
            .clone()
            .into_iter()
            .map(legacy_method)
            .map(to_client_pm)
            .collect();
        for candidates in [methods, legacy_methods] {
            let result = filter_customer_payment_methods(candidates, &enabled, |methods| {
                assert!(methods
                    .iter()
                    .all(|pm| pm.payment_method_type != Some(PaymentMethodType::GooglePay)));
                assert_eq!(
                    methods
                        .iter()
                        .filter(|pm| pm.payment_method_type == Some(PaymentMethodType::ApplePay))
                        .count(),
                    2
                );
                ready(filter_customer_pms_by_blocked_bins(
                    methods,
                    &HashSet::from(["411111".to_owned()]),
                ))
            })
            .await;
            assert_eq!(
                result,
                vec![allowed_card.clone(), paypal.clone(), latest_apple.clone()]
            );
        }
    }

    #[tokio::test]
    async fn customer_pm_filtering_preserves_legacy_wallet_selection_and_other_methods() {
        let methods = vec![
            saved_wallet("apple_old", PaymentMethodType::ApplePay, Some(1)),
            saved_wallet("google_new", PaymentMethodType::GooglePay, Some(6)),
            saved_wallet("paypal_1", PaymentMethodType::Paypal, Some(1)),
            saved_wallet("apple_new", PaymentMethodType::ApplePay, Some(4)),
            saved_wallet("google_old", PaymentMethodType::GooglePay, Some(2)),
            saved_wallet("paypal_2", PaymentMethodType::Paypal, Some(3)),
        ];
        let legacy_methods = methods.clone().into_iter().map(legacy_method).collect();
        let legacy_result: Vec<_> = cards::filter_latest_wallet_methods(legacy_methods, |pm| {
            (pm.payment_method, pm.payment_method_type, pm.last_used_at)
        })
        .into_iter()
        .map(to_client_pm)
        .collect();
        let result = filter_customer_payment_methods(methods, &enabled_wallets(), ready).await;
        assert_eq!(result, legacy_result);
        let tokens: Vec<_> = result.into_iter().map(|pm| pm.payment_token).collect();
        assert_eq!(tokens, ["paypal_1", "paypal_2", "apple_new", "google_new"]);
    }

    #[tokio::test]
    async fn customer_pm_filtering_handles_missing_and_equal_timestamps() {
        let enabled = enabled_wallets();
        for days in [None, Some(3)] {
            let first = saved_wallet("first", PaymentMethodType::ApplePay, days);
            let second = saved_wallet("second", PaymentMethodType::ApplePay, days);
            assert_eq!(
                filter_customer_payment_methods(vec![first.clone(), second], &enabled, ready).await,
                vec![first]
            );
        }
        let missing = saved_wallet("missing", PaymentMethodType::ApplePay, None);
        let dated = saved_wallet("dated", PaymentMethodType::ApplePay, Some(1));
        assert_eq!(
            filter_customer_payment_methods(vec![missing, dated.clone()], &enabled, ready).await,
            vec![dated]
        );
    }

    #[tokio::test]
    async fn customer_pm_filtering_handles_empty_single_and_disabled_lists() {
        let enabled = enabled_wallets();
        assert!(filter_customer_payment_methods(Vec::new(), &enabled, ready)
            .await
            .is_empty());
        let method = saved_wallet("only", PaymentMethodType::ApplePay, Some(1));
        assert_eq!(
            filter_customer_payment_methods(vec![method.clone()], &enabled, ready).await,
            vec![method.clone()]
        );
        assert!(filter_customer_payment_methods(vec![method], &[], ready)
            .await
            .is_empty());
        let unknown_wallet = saved_method("unknown", PaymentMethod::Wallet, None, None);
        assert!(
            filter_customer_payment_methods(vec![unknown_wallet], &enabled, ready)
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn customer_pm_filtering_preserves_cards_without_blocked_bins() {
        let mut allowed = saved_method(
            "allowed_card",
            PaymentMethod::Card,
            Some(PaymentMethodType::Credit),
            Some(1),
        );
        allowed.payment_method_data = Some(saved_card_data(Some("555555")));
        let mut missing_bin = allowed.clone();
        missing_bin.payment_token = "missing_bin".to_owned();
        missing_bin.payment_method_data = Some(saved_card_data(None));
        let methods = vec![allowed, missing_bin];
        let enabled = [enabled_method(
            PaymentMethod::Card,
            PaymentMethodType::Credit,
        )];
        for blocked_bins in [HashSet::new(), HashSet::from(["411111".to_owned()])] {
            let result = filter_customer_payment_methods(methods.clone(), &enabled, |methods| {
                ready(filter_customer_pms_by_blocked_bins(methods, &blocked_bins))
            })
            .await;
            assert_eq!(result, methods);
        }
    }
}
