/// GET /v1/payments/{payment_id}/client
///
/// Returns a combined list of:
///   - merchant-enabled payment methods (filtered via Euclid constraint graph + session flow routing)
///   - customer saved payment methods (fetched from DB via `CustomerPaymentMethodsFetcher` trait)
///
/// Auth: `SdkAuthorizationAuth` only (Authorization: base64(publishable_key:client_secret))
use api_models::payment_methods::{
    ClientPaymentMethodsListResponse, CustomerPaymentMethod, CustomerPaymentMethodDataForClient,
    CustomerPaymentMethodForClient, PaymentMethodListIntentDataInput,
    ResponsePaymentMethodsEnabledForClient,
};
use common_utils::{consts, ext_traits::AsyncExt, generate_id, id_type};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing, Flow};

use crate::{
    core::{
        configs::dimension_state,
        errors::{self, StorageErrorExt},
        payment_methods::{
            cards, transformers::list_customer_payment_methods_from_modular_service,
        },
        payments::helpers,
    },
    pii::PeekInterface,
    routes::{self, payment_methods::ParentPaymentMethodToken},
    services,
    types::{
        domain::{self, Profile},
        storage::{self, BankDebitTokenData, PaymentTokenData},
    },
};

// ---------------------------------------------------------------------------
// Trait: CustomerPaymentMethodsFetcher
// ---------------------------------------------------------------------------

/// Abstraction over saved-PM retrieval — allows future swap to a remote PM service.
#[async_trait::async_trait]
pub trait CustomerPaymentMethodsFetcher: Send + Sync {
    async fn fetch(
        &self,
        state: &routes::SessionState,
        platform: &domain::Platform,
        payment_intent: Option<&storage::PaymentIntent>,
        customer_id: &id_type::CustomerId,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> errors::RouterResult<Vec<CustomerPaymentMethodForClient>>;
}

/// Convert a legacy `CustomerPaymentMethod` into the slimmer client-facing type.
fn to_client_pm(pm: CustomerPaymentMethod) -> CustomerPaymentMethodForClient {
    let payment_method_data = pm
        .card
        .map(|card| CustomerPaymentMethodDataForClient::Card(Box::new(card)));

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
pub struct DbCustomerPaymentMethodsFetcher;

#[async_trait::async_trait]
impl CustomerPaymentMethodsFetcher for DbCustomerPaymentMethodsFetcher {
    async fn fetch(
        &self,
        state: &routes::SessionState,
        platform: &domain::Platform,
        payment_intent: Option<&storage::PaymentIntent>,
        customer_id: &id_type::CustomerId,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> errors::RouterResult<Vec<CustomerPaymentMethodForClient>> {
        let customer_payment_methods_response = Box::pin(cards::list_customer_payment_method(
            state,
            platform.clone(),
            payment_intent.cloned(),
            customer_id,
            None, // limit
            dimensions,
        ))
        .await?;

        let response_body = match customer_payment_methods_response {
            services::ApplicationResponse::Json(response_body) => response_body,
            _ => {
                return Err(errors::ApiErrorResponse::InternalServerError.into());
            }
        };

        Ok(response_body
            .customer_payment_methods
            .into_iter()
            .map(to_client_pm)
            .collect())
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
        _payment_intent: Option<&storage::PaymentIntent>,
        customer_id: &id_type::CustomerId,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> errors::RouterResult<Vec<CustomerPaymentMethodForClient>> {
        let merchant_id = platform.get_processor().get_account().get_id().clone();

        let items = list_customer_payment_methods_from_modular_service(
            state,
            &merchant_id,
            &self.profile_id,
            customer_id.clone(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

        // Base requires_cvv from merchant/superposition config (same as legacy PM list).
        let requires_cvv_base = dimensions
            .get_requires_cvv(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                Some(customer_id),
            )
            .await;

        let mut customer_payment_methods = Vec::with_capacity(items.len());

        for pm in items {
            // Compute requires_cvv using the same logic as the legacy API
            // (using psp_tokenization_enabled in place of connector_mandate_details
            //  and network_tokenization in place of network_transaction_id).
            let requires_cvv = if self.is_connector_agnostic_mit_enabled {
                requires_cvv_base
                    && !(self.off_session_payment_flag
                        && (pm.psp_tokenization_enabled || pm.network_tokenization.is_some()))
            } else {
                requires_cvv_base && !(self.off_session_payment_flag && pm.psp_tokenization_enabled)
            };

            // recurring_enabled is inferred from psp_tokenization_enabled.
            let recurring_enabled = Some(pm.psp_tokenization_enabled);

            let payment_token = Self::store_payment_token_in_redis(
                state,
                pm.payment_method_type,
                pm.id.clone(),
                self.intent_fulfillment_time,
            )
            .await?;

            // Build the client-facing response item.
            let payment_method_data = pm.payment_method_data.and_then(|d| d.into());

            customer_payment_methods.push(CustomerPaymentMethodForClient {
                payment_token,
                payment_method: pm.payment_method_type,
                payment_method_type: Some(pm.payment_method_subtype),
                default_payment_method_set: pm.is_default,
                requires_cvv,
                recurring_enabled,
                created: Some(pm.created),
                last_used_at: Some(pm.last_used_at),
                payment_method_data,
            });
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
            platform.get_processor().get_account().get_id(),
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let payment_attempt = db
        .find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
            &payment_intent.payment_id,
            &payment_intent.processor_merchant_id,
            &payment_intent.active_attempt.get_id(),
            platform.get_processor().get_account().storage_scheme,
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
            db.find_customer_by_customer_id_merchant_id(
                customer_id,
                &payment_intent.merchant_id,
                platform.get_provider().get_key_store(),
                platform.get_provider().get_account().storage_scheme,
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

    let mut flat_pms = merchant_enabled_pms_context.payment_experience_pms_for_client();
    flat_pms.extend(merchant_enabled_pms_context.card_network_pms_for_client());
    flat_pms.extend(merchant_enabled_pms_context.bank_redirect_pms_for_client(state)?);
    flat_pms.extend(merchant_enabled_pms_context.bank_debit_pms_for_client());
    flat_pms.extend(merchant_enabled_pms_context.bank_transfer_pms_for_client());

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

async fn fetch_customer_payment_methods(
    state: &routes::SessionState,
    platform: &domain::Platform,
    payment_intent_context: &PaymentIntentContext,
) -> errors::RouterResult<Vec<CustomerPaymentMethodForClient>> {
    let customer_id = match payment_intent_context.payment_intent.customer_id.as_ref() {
        Some(customer_id) => customer_id,
        None => return Ok(vec![]),
    };

    let dimensions = dimension_state::Dimensions::new()
        .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id());

    let feature_config = crate::core::utils::get_feature_config(state, platform, &dimensions).await;

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
            customer_id,
            &dimensions,
        )
        .await
    } else {
        logger::info!("Fetching customer payment methods from DB");
        DbCustomerPaymentMethodsFetcher
            .fetch(
                state,
                platform,
                Some(&payment_intent_context.payment_intent),
                customer_id,
                &dimensions,
            )
            .await
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
) -> errors::RouterResponse<ClientPaymentMethodsListResponse> {
    // 1. Load payment intent + related context
    let payment_intent_context =
        load_payment_intent_context(&state, &platform, &payment_id).await?;

    // 2. Fetch enabled payment methods (Gate 1 + Gate 2 + consolidation)
    let EnabledPmsResult {
        payment_methods_enabled,
        sdk_next_action,
        connector_supports_installments,
    } = fetch_enabled_payment_methods(&state, &platform, &payment_intent_context).await?;

    // 3. Fetch saved customer payment methods
    let customer_payment_methods =
        fetch_customer_payment_methods(&state, &platform, &payment_intent_context).await?;

    // 4. Filter customer PMs to only those whose (payment_method, payment_method_type)
    //    combination is present in the merchant-enabled list.
    let customer_payment_methods_filtered =
        filter_customer_pms_by_enabled(customer_payment_methods, &payment_methods_enabled);

    // 5. Build intent_data
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
