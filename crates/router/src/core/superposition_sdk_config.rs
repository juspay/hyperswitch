use api_models::{
    enums as api_enums,
    superposition_sdk_config::{
        AccountConfig, PaymentMethodCriteria, ProfileAccountConfig, SdkCriteriaRule,
        SdkPaymentMethod, SdkPaymentMethodType, SuperPositionConfigResponse, VaultingAction,
    },
};
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use serde_json::Map;

use crate::{
    consts::superposition::DYNAMIC_FIELDS,
    core::{
        configs::dimension_state::Dimensions,
        errors::{self, RouterResponse, StorageErrorExt},
        payment_methods::cards::{
            build_merchant_enabled_pms_context, get_banks, MerchantEnabledPmsContext,
        },
        payments::helpers,
    },
    routes::SessionState,
    types::domain,
};

pub struct SdkPaymentContext {
    pub payment_attempt: Option<domain::PaymentAttempt>,
    pub shipping_address: Option<domain::Address>,
    pub billing_address: Option<domain::Address>,
    pub customer: Option<domain::Customer>,
    pub is_cit_transaction: bool,
}

async fn get_payment_context(
    state: &SessionState,
    platform: &domain::Platform,
    payment_intent: Option<&hyperswitch_domain_models::payments::PaymentIntent>,
) -> error_stack::Result<SdkPaymentContext, errors::ApiErrorResponse> {
    let db = &*state.store;

    let payment_attempt = payment_intent
        .as_ref()
        .async_map(|pi| async {
            db.find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
                &pi.payment_id,
                &pi.processor_merchant_id,
                &pi.active_attempt.get_id(),
                platform.get_processor().get_account().storage_scheme,
                platform.get_processor().get_key_store(),
            )
            .await
            .change_context(errors::ApiErrorResponse::PaymentNotFound)
        })
        .await
        .transpose()?;

    let shipping_address = payment_intent
        .as_ref()
        .async_map(|pi| async {
            helpers::get_address_by_id(
                state,
                pi.shipping_address_id.clone(),
                platform.get_processor().get_key_store(),
                &pi.payment_id,
                platform.get_processor().get_account().get_id(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
        })
        .await
        .transpose()?
        .flatten();

    let billing_address = payment_intent
        .as_ref()
        .async_map(|pi| async {
            helpers::get_address_by_id(
                state,
                pi.billing_address_id.clone(),
                platform.get_processor().get_key_store(),
                &pi.payment_id,
                platform.get_processor().get_account().get_id(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
        })
        .await
        .transpose()?
        .flatten();

    let customer = payment_intent
        .as_ref()
        .async_and_then(|pi| async {
            pi.customer_id
                .as_ref()
                .async_and_then(|cust| async {
                    db.find_customer_by_customer_id_merchant_id(
                        cust,
                        &pi.merchant_id,
                        platform.get_provider().get_key_store(),
                        platform.get_provider().get_account().storage_scheme,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
                    .ok()
                })
                .await
        })
        .await;

    let setup_future_usage = payment_intent.as_ref().and_then(|pi| pi.setup_future_usage);

    let is_cit_transaction = payment_attempt
        .as_ref()
        .map(|pa| pa.mandate_details.is_some())
        .unwrap_or(false)
        || setup_future_usage
            .map(|future_usage| future_usage == common_enums::FutureUsage::OffSession)
            .unwrap_or(false);

    Ok(SdkPaymentContext {
        payment_attempt,
        shipping_address,
        billing_address,
        customer,
        is_cit_transaction,
    })
}

pub async fn get_superposition_sdk_config(
    state: SessionState,
    platform: domain::Platform,
    payment_id: common_utils::id_type::PaymentId,
) -> RouterResponse<SuperPositionConfigResponse> {
    let merchant_account = platform.get_processor().get_account();
    let db = &*state.store;
    let payment_intent = db
        .find_payment_intent_by_payment_id_processor_merchant_id(
            &payment_id,
            merchant_account.get_id(),
            platform.get_processor().get_key_store(),
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

    let payment_context = get_payment_context(&state, &platform, Some(&payment_intent)).await?;

    let profile_id = payment_intent.profile_id.clone().ok_or(
        errors::ApiErrorResponse::GenericNotFoundError {
            message: "Profile id not found".to_string(),
        },
    )?;

    let business_profile = db
        .find_business_profile_by_profile_id(platform.get_processor().get_key_store(), &profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let merchant_enabled_context = Box::pin(build_merchant_enabled_pms_context(
        &state,
        &platform,
        &business_profile,
        Some(&payment_intent),
        payment_context.payment_attempt.as_ref(),
        payment_context.billing_address.as_ref(),
        payment_context.shipping_address.as_ref(),
        payment_context.customer.as_ref(),
        payment_context.is_cit_transaction,
    ))
    .await?;

    // Build dimension filter for superposition context
    let mut dimension_filter = Map::new();
    dimension_filter.insert(
        "profile_id".to_string(),
        serde_json::Value::String(profile_id.get_string_repr().to_string()),
    );
    dimension_filter.insert(
        "merchant_id".to_string(),
        serde_json::Value::String(merchant_account.get_id().get_string_repr().to_string()),
    );
    dimension_filter.insert(
        "organization_id".to_string(),
        serde_json::Value::String(merchant_account.get_org_id().get_string_repr().to_string()),
    );
    dimension_filter.insert(
        "connector".to_string(),
        serde_json::Value::Array(
            merchant_enabled_context
                .get_eligible_connectors()
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ),
    );

    let raw_configs = state
        .superposition_service
        .get_cached_config(
            Some(vec![DYNAMIC_FIELDS.to_string()]),
            Some(dimension_filter.clone()),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed to fetch superposition config for dimension filter: {dimension_filter:?}"
            )
        })?;

    let payment_methods = translate_to_sdk_payment_methods(&state, &merchant_enabled_context)?;

    let account_config = build_account_config(&state, &platform, &business_profile).await;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        SuperPositionConfigResponse {
            raw_configs,
            resolved_configs: None,
            context_used: dimension_filter,
            payment_methods: Some(payment_methods),
            account_config,
        },
    ))
}

/// Build the account level configuration surfaced to the SDK.
async fn build_account_config(
    state: &SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
) -> AccountConfig {
    let dimensions = Dimensions::new()
        .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id());
    let feature_config = crate::core::utils::get_feature_config(state, platform, &dimensions).await;

    AccountConfig {
        profile: build_profile_account_config(
            business_profile,
            feature_config.is_payment_method_modular_allowed,
        ),
    }
}

/// Build the profile level configuration surfaced to the SDK.
fn build_profile_account_config(
    business_profile: &domain::Profile,
    is_payment_method_modular_allowed: bool,
) -> ProfileAccountConfig {
    ProfileAccountConfig {
        collect_shipping_details_from_wallet_connector: business_profile
            .collect_shipping_details_from_wallet_connector
            .unwrap_or(false),
        collect_billing_details_from_wallet_connector: business_profile
            .collect_billing_details_from_wallet_connector
            .unwrap_or(false),
        always_collect_billing_details_from_wallet_connector: business_profile
            .always_collect_billing_details_from_wallet_connector
            .unwrap_or(false),
        always_collect_shipping_details_from_wallet_connector: business_profile
            .always_collect_shipping_details_from_wallet_connector
            .unwrap_or(false),
        vaulting_action: resolve_vaulting_action(is_payment_method_modular_allowed),
    }
}

/// Determine whether the SDK should tokenize payment method details.
///
/// Driven solely by whether the modular vaulting flow should be invoked: `Tokenize` when it
/// should, `Skip` otherwise.
fn resolve_vaulting_action(should_call_modular: bool) -> VaultingAction {
    if should_call_modular {
        VaultingAction::Tokenize
    } else {
        VaultingAction::Skip
    }
}

fn format_criteria_value(
    card_network: Option<String>,
    bank_name: Option<String>,
    payment_experience: Option<String>,
) -> String {
    let mut parts = Vec::new();
    if let Some(cn) = card_network {
        parts.push(cn);
    }
    if let Some(bn) = bank_name {
        parts.push(bn);
    }
    if let Some(pe) = payment_experience {
        parts.push(pe);
    }
    if parts.is_empty() {
        "default".to_string()
    } else {
        parts.join("/")
    }
}

fn translate_to_sdk_payment_methods(
    state: &SessionState,
    pms_ctx: &MerchantEnabledPmsContext,
) -> error_stack::Result<Vec<SdkPaymentMethod>, errors::ApiErrorResponse> {
    let mut consolidated_rules: std::collections::HashMap<
        (api_enums::PaymentMethod, api_enums::PaymentMethodType),
        (Option<PaymentMethodCriteria>, Vec<SdkCriteriaRule>),
    > = std::collections::HashMap::new();

    // 1. Payment experiences (wallets, paylater, etc.)
    for (payment_method, pmt_map) in &pms_ctx.payment_experiences_consolidated_hm {
        for (payment_method_type, pe_map) in pmt_map {
            let (_, rules) = consolidated_rules
                .entry((*payment_method, *payment_method_type))
                .or_insert_with(|| (Some(PaymentMethodCriteria::PaymentExperience), Vec::new()));
            for (payment_experience, connectors) in pe_map {
                let criteria_value =
                    format_criteria_value(None, None, Some(payment_experience.to_string()));
                rules.push(SdkCriteriaRule {
                    criteria_value,
                    eligible_connectors: connectors.clone(),
                });
            }
        }
    }

    // 2. Card networks (cards)
    for (payment_method, pmt_map) in &pms_ctx.card_networks_consolidated_hm {
        for (payment_method_type, card_network_map) in pmt_map {
            let (_, rules) = consolidated_rules
                .entry((*payment_method, *payment_method_type))
                .or_insert_with(|| (Some(PaymentMethodCriteria::CardNetwork), Vec::new()));
            for (card_network, connectors) in card_network_map {
                let criteria_value =
                    format_criteria_value(Some(card_network.to_string()), None, None);
                rules.push(SdkCriteriaRule {
                    criteria_value,
                    eligible_connectors: connectors.clone(),
                });
            }
        }
    }

    // 3. Banks (bank redirect)
    for (payment_method_type, connectors) in &pms_ctx.banks_consolidated_hm {
        let bank_names = get_banks(state, *payment_method_type, connectors.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let (_, rules) = consolidated_rules
            .entry((api_enums::PaymentMethod::BankRedirect, *payment_method_type))
            .or_insert_with(|| (Some(PaymentMethodCriteria::BankName), Vec::new()));
        for bank_code_res in bank_names {
            for bank_name in bank_code_res.bank_name {
                let criteria_value = format_criteria_value(None, Some(bank_name.to_string()), None);
                rules.push(SdkCriteriaRule {
                    criteria_value,
                    eligible_connectors: bank_code_res.eligible_connectors.clone(),
                });
            }
        }
    }

    // 4. Bank debits
    for (payment_method_type, connectors) in &pms_ctx.bank_debits_consolidated_hm {
        let (_, rules) = consolidated_rules
            .entry((api_enums::PaymentMethod::BankDebit, *payment_method_type))
            .or_insert_with(|| (None, Vec::new()));
        let criteria_value = format_criteria_value(None, None, None);
        rules.push(SdkCriteriaRule {
            criteria_value,
            eligible_connectors: connectors.clone(),
        });
    }

    // 5. Bank transfers
    for (payment_method_type, connectors) in &pms_ctx.bank_transfer_consolidated_hm {
        let (_, rules) = consolidated_rules
            .entry((api_enums::PaymentMethod::BankTransfer, *payment_method_type))
            .or_insert_with(|| (None, Vec::new()));
        let criteria_value = format_criteria_value(None, None, None);
        rules.push(SdkCriteriaRule {
            criteria_value,
            eligible_connectors: connectors.clone(),
        });
    }

    let mut payment_methods_map: std::collections::HashMap<
        api_enums::PaymentMethod,
        Vec<SdkPaymentMethodType>,
    > = std::collections::HashMap::new();

    for ((payment_method, payment_method_type), (payment_method_criteria, rules)) in
        consolidated_rules
    {
        if !rules.is_empty() {
            let method_types = payment_methods_map.entry(payment_method).or_default();
            method_types.push(SdkPaymentMethodType {
                payment_method_type,
                payment_method_criteria,
                criteria_rules: rules,
            });
        }
    }

    let mut payment_methods = vec![];
    for (payment_method, mut payment_method_types) in payment_methods_map {
        payment_method_types.sort_by_key(|pmt| pmt.payment_method_type.to_string());
        payment_methods.push(SdkPaymentMethod {
            payment_method,
            payment_method_types,
        });
    }
    payment_methods.sort_by_key(|pm| pm.payment_method.to_string());

    Ok(payment_methods)
}
