use api_models::{
    enums as api_enums,
    superposition_sdk_config::{
        AccountConfig, PaymentMethodCriteria, ProfileAccountConfig, SdkCriteriaRule,
        SdkPaymentMethod, SdkPaymentMethodType, SuperPositionConfigResponse, VaultingAction,
    },
};
use common_enums::ConnectorType;
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
    pii::ExposeInterface,
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

#[derive(Debug)]
struct PaymentContextSuperposition {
    pub currency: String,
    pub payment_method: String,
    pub country: String,
    pub capture_method: String,
    pub connector: String,
}

const UNSPECIFIED_CONTEXT_VALUE: &str = "__unspecified__";
const PAYMENT_ACTION_CONFIG_KEY: &str = "payment_action";
const PAYMENTS_ENABLED_CONFIG_KEY: &str = "payments_enabled";
const IS_PROCESSING_CONFIG_KEY: &str = "is_processing";
const PAYMENT_ACTION_ALLOW: &str = "allow";

fn resolve_payment_action(resolved_config: &Map<String, serde_json::Value>) -> bool {
    resolved_config
        .get(PAYMENT_ACTION_CONFIG_KEY)
        .and_then(serde_json::Value::as_str)
        .map(|payment_action| payment_action == PAYMENT_ACTION_ALLOW)
        .or_else(|| {
            resolved_config
                .get(PAYMENTS_ENABLED_CONFIG_KEY)
                .and_then(serde_json::Value::as_bool)
        })
        .or_else(|| {
            resolved_config
                .get(IS_PROCESSING_CONFIG_KEY)
                .and_then(serde_json::Value::as_bool)
        })
        .unwrap_or(false)
}

fn get_eligible_connectors_for_payment_context(
    merchant_enabled_context: &MerchantEnabledPmsContext,
    payment_method: api_enums::PaymentMethod,
    payment_method_type: api_enums::PaymentMethodType,
) -> Vec<String> {
    let mut connectors = std::collections::HashSet::new();

    if let Some(payment_method_type_hm) = merchant_enabled_context
        .payment_experiences_consolidated_hm
        .get(&payment_method)
    {
        if let Some(criteria_hm) = payment_method_type_hm.get(&payment_method_type) {
            for connector_list in criteria_hm.values() {
                connectors.extend(connector_list.iter().cloned());
            }
        }
    }

    if let Some(payment_method_type_hm) = merchant_enabled_context
        .card_networks_consolidated_hm
        .get(&payment_method)
    {
        if let Some(criteria_hm) = payment_method_type_hm.get(&payment_method_type) {
            for connector_list in criteria_hm.values() {
                connectors.extend(connector_list.iter().cloned());
            }
        }
    }

    match payment_method {
        api_enums::PaymentMethod::BankRedirect => {
            if let Some(connector_list) = merchant_enabled_context
                .banks_consolidated_hm
                .get(&payment_method_type)
            {
                connectors.extend(connector_list.iter().cloned());
            }
        }
        api_enums::PaymentMethod::BankDebit => {
            if let Some(connector_list) = merchant_enabled_context
                .bank_debits_consolidated_hm
                .get(&payment_method_type)
            {
                connectors.extend(connector_list.iter().cloned());
            }
        }
        api_enums::PaymentMethod::BankTransfer => {
            if let Some(connector_list) = merchant_enabled_context
                .bank_transfer_consolidated_hm
                .get(&payment_method_type)
            {
                connectors.extend(connector_list.iter().cloned());
            }
        }
        _ => {}
    }

    connectors.into_iter().collect()
}

/// Build a random `PaymentsRequest` from the `[pm_filters.worldpayxml]` country/currency lists.
///
/// Picks a random amount, currency, country (billing address) and payment method, then materialises
/// the request through JSON (the same shape a real create call uses) so we don't have to construct
/// the 80+ field `PaymentsRequest` by hand. Currency/country are validated against their enums and
/// fall back to `USD`/`US` so the request always deserializes.
#[cfg(feature = "v1")]
pub fn build_random_payment_create_request(
) -> error_stack::Result<api_models::payments::PaymentsRequest, errors::ApiErrorResponse> {
    use rand::{seq::SliceRandom, Rng};

    // CountryAlpha2 codes configured for worldpayxml.
    const COUNTRIES: &[&str] = &[
        "AF", "DZ", "AW", "AU", "AZ", "BS", "BH", "BD", "BB", "BZ", "BM", "BT", "BO", "BA", "BW",
        "BR", "BN", "BG", "BI", "KH", "CA", "CV", "KY", "CL", "CO", "KM", "CD", "CR", "CZ", "DK",
        "DJ", "ST", "DO", "EC", "EG", "SV", "ER", "ET", "FK", "FJ", "GM", "GE", "GH", "GI", "GT",
        "GN", "GY", "HT", "HN", "HK", "HU", "IS", "IN", "ID", "IR", "IQ", "IE", "IL", "IT", "JM",
        "JP", "JO", "KZ", "KE", "KW", "LA", "LB", "LS", "LR", "LY", "LT", "MO", "MK", "MG", "MW",
        "MY", "MV", "MR", "MU", "MX", "MD", "MN", "MA", "MZ", "MM", "NA", "NZ", "NI", "NG", "KP",
        "NO", "AR", "PK", "PG", "PY", "PE", "UY", "PH", "PL", "GB", "QA", "OM", "RO", "RU", "RW",
        "WS", "SG", "ZA", "KR", "LK", "SH", "SD", "SR", "SZ", "SE", "CH", "SY", "TW", "TJ", "TZ",
        "TH", "TT", "TN", "TR", "UG", "UA", "US", "UZ", "VU", "VE", "VN", "ZM", "ZW",
    ];
    // ISO 4217 currency codes configured for worldpayxml.
    const CURRENCIES: &[&str] = &[
        "AFN", "DZD", "ANG", "AWG", "AUD", "AZN", "BSD", "BHD", "BDT", "BBD", "BZD", "BMD", "BTN",
        "BOB", "BAM", "BWP", "BRL", "BND", "BGN", "BIF", "KHR", "CAD", "CVE", "KYD", "XOF", "XAF",
        "XPF", "CLP", "COP", "KMF", "CDF", "CRC", "EUR", "CZK", "DKK", "DJF", "DOP", "XCD", "EGP",
        "SVC", "ERN", "ETB", "FKP", "FJD", "GMD", "GEL", "GHS", "GIP", "GTQ", "GNF", "GYD", "HTG",
        "HNL", "HKD", "HUF", "ISK", "INR", "IDR", "IRR", "IQD", "ILS", "JMD", "JPY", "JOD", "KZT",
        "KES", "KWD", "LAK", "LBP", "LSL", "LRD", "LYD", "MOP", "MKD", "MGA", "MWK", "MYR", "MVR",
        "MRU", "MUR", "MXN", "MDL", "MNT", "MAD", "MZN", "MMK", "NAD", "NPR", "NZD", "NIO", "NGN",
        "KPW", "NOK", "ARS", "PKR", "PAB", "PGK", "PYG", "PEN", "UYU", "PHP", "PLN", "GBP", "QAR",
        "OMR", "RON", "RUB", "RWF", "WST", "SAR", "RSD", "SCR", "SLL", "SGD", "STN", "SBD", "SOS",
        "ZAR", "KRW", "LKR", "SHP", "SDG", "SRD", "SZL", "SEK", "CHF", "SYP", "TWD", "TJS", "TZS",
        "THB", "TOP", "TTD", "TND", "TRY", "TMT", "AED", "UGX", "UAH", "USD", "UZS", "VUV", "VND",
        "YER", "CNY", "ZMW", "ZWL",
    ];
    const PAYMENT_METHODS: &[&str] = &["card", "wallet"];

    let mut rng = rand::thread_rng();

    let amount = rng.gen_range(100..1_000_000_u64);
    let currency = CURRENCIES
        .choose(&mut rng)
        .copied()
        .filter(|code| code.parse::<api_enums::Currency>().is_ok())
        .unwrap_or("USD");
    let country = COUNTRIES
        .choose(&mut rng)
        .copied()
        .filter(|code| code.parse::<common_enums::CountryAlpha2>().is_ok())
        .unwrap_or("US");
    let _payment_method = PAYMENT_METHODS.choose(&mut rng).copied().unwrap_or("card");

    let request_json = serde_json::json!({
        "amount": amount,
        "currency": currency,
        // "payment_method": payment_method,
        "billing": { "address": { "country": country } },
    });

    serde_json::from_value::<api_models::payments::PaymentsRequest>(request_json)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to build random payment create request")
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

pub async fn get_profile_superposition_sdk_config(
    state: SessionState,
    platform: domain::Platform,
    profile_id: String,
) -> RouterResponse<SuperPositionConfigResponse> {
    let merchant_account = platform.get_processor().get_account();
    let db = &*state.store;

    let profile_id_typed =
        common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from(profile_id.clone()))
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "profile_id",
            })?;

    let business_profile = db
        .find_business_profile_by_profile_id(
            platform.get_processor().get_key_store(),
            &profile_id_typed,
        )
        .await
        .change_context(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.to_owned(),
        })?;

    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            platform.get_processor().get_account().get_id(),
            false,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let filtered_mcas: Vec<_> = all_mcas
        .into_iter()
        .filter(|mca| mca.profile_id == profile_id_typed)
        .collect();

    let mut payment_experiences_consolidated_hm: std::collections::HashMap<
        api_enums::PaymentMethod,
        std::collections::HashMap<
            api_enums::PaymentMethodType,
            std::collections::HashMap<api_enums::PaymentExperience, Vec<String>>,
        >,
    > = std::collections::HashMap::new();

    let mut card_networks_consolidated_hm: std::collections::HashMap<
        api_enums::PaymentMethod,
        std::collections::HashMap<
            api_enums::PaymentMethodType,
            std::collections::HashMap<api_enums::CardNetwork, Vec<String>>,
        >,
    > = std::collections::HashMap::new();

    let mut banks_consolidated_hm: std::collections::HashMap<
        api_enums::PaymentMethodType,
        Vec<String>,
    > = std::collections::HashMap::new();
    let mut bank_debits_consolidated_hm: std::collections::HashMap<
        api_enums::PaymentMethodType,
        Vec<String>,
    > = std::collections::HashMap::new();
    let mut bank_transfer_consolidated_hm: std::collections::HashMap<
        api_enums::PaymentMethodType,
        Vec<String>,
    > = std::collections::HashMap::new();

    for mca in &filtered_mcas {
        if let Some(payment_methods_enabled_list) = &mca.payment_methods_enabled {
            for pm_value in payment_methods_enabled_list {
                if let Ok(pm_enabled) = serde_json::from_value::<
                    api_models::admin::PaymentMethodsEnabled,
                >(pm_value.clone().expose())
                {
                    let payment_method = pm_enabled.payment_method;
                    for pmt_info in pm_enabled.payment_method_types.unwrap_or_default() {
                        let payment_method_type = pmt_info.payment_method_type;
                        let connector = mca.connector_name.clone();

                        if let Some(payment_experience) = pmt_info.payment_experience {
                            let payment_method_hm = payment_experiences_consolidated_hm
                                .entry(payment_method)
                                .or_default();
                            let payment_method_type_hm =
                                payment_method_hm.entry(payment_method_type).or_default();
                            let vector_of_connectors = payment_method_type_hm
                                .entry(payment_experience)
                                .or_default();
                            if !vector_of_connectors.contains(&connector) {
                                vector_of_connectors.push(connector.clone());
                            }
                        }

                        if let Some(card_networks) = pmt_info.card_networks {
                            let payment_method_hm = card_networks_consolidated_hm
                                .entry(payment_method)
                                .or_default();
                            let payment_method_type_hm =
                                payment_method_hm.entry(payment_method_type).or_default();
                            for card_network in card_networks {
                                let vector_of_connectors =
                                    payment_method_type_hm.entry(card_network).or_default();
                                if !vector_of_connectors.contains(&connector) {
                                    vector_of_connectors.push(connector.clone());
                                }
                            }
                        }

                        if payment_method == api_enums::PaymentMethod::BankRedirect {
                            let vector_of_connectors = banks_consolidated_hm
                                .entry(payment_method_type)
                                .or_default();
                            if !vector_of_connectors.contains(&connector) {
                                vector_of_connectors.push(connector.clone());
                            }
                        }

                        if payment_method == api_enums::PaymentMethod::BankDebit {
                            let vector_of_connectors = bank_debits_consolidated_hm
                                .entry(payment_method_type)
                                .or_default();
                            if !vector_of_connectors.contains(&connector) {
                                vector_of_connectors.push(connector.clone());
                            }
                        }

                        if payment_method == api_enums::PaymentMethod::BankTransfer {
                            let vector_of_connectors = bank_transfer_consolidated_hm
                                .entry(payment_method_type)
                                .or_default();
                            if !vector_of_connectors.contains(&connector) {
                                vector_of_connectors.push(connector.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    let merchant_enabled_context = MerchantEnabledPmsContext {
        payment_experiences_consolidated_hm,
        card_networks_consolidated_hm,
        banks_consolidated_hm,
        bank_debits_consolidated_hm,
        bank_transfer_consolidated_hm,
        required_fields_hm: std::collections::HashMap::new(),
        pmt_to_auth_connector: std::collections::HashMap::new(),
        connector_supports_installments: false,
        collect_shipping_details_from_wallets: None,
        collect_billing_details_from_wallets: None,
        sdk_next_action: api_models::payments::SdkNextAction {
            next_action: api_models::payments::NextActionCall::Confirm,
            should_block_confirm: None,
        },
    };

    let mut dimension_filter = Map::new();
    dimension_filter.insert(
        "profile_id".to_string(),
        serde_json::Value::String(profile_id.to_string()),
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
            filtered_mcas
                .iter()
                .map(|mca| serde_json::Value::String(mca.connector_name.clone()))
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

    let merchant_connector_accounts = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            merchant_account.get_id(),
            false,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let all_active_connectors = merchant_connector_accounts
        .iter()
        .filter(|mca| {
            mca.profile_id == profile_id && mca.connector_type == ConnectorType::PaymentProcessor
        })
        .map(|mca| mca.connector_name.clone())
        .collect::<Vec<String>>();

    let mut active_payment_methods = Vec::new();
    for mca in &merchant_connector_accounts {
        if mca.profile_id == profile_id {
            if let Some(payment_methods_enabled_list) = &mca.payment_methods_enabled {
                for pm_value in payment_methods_enabled_list {
                    if let Ok(pm_enabled) = serde_json::from_value::<
                        api_models::admin::PaymentMethodsEnabled,
                    >(pm_value.clone().expose())
                    {
                        let payment_method = pm_enabled.payment_method;
                        for pmt_info in pm_enabled.payment_method_types.unwrap_or_default() {
                            let payment_method_type = pmt_info.payment_method_type;
                            if !active_payment_methods
                                .contains(&(payment_method, payment_method_type))
                            {
                                active_payment_methods.push((payment_method, payment_method_type));
                            }
                        }
                    }
                }
            }
        }
    }

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

    println!(
        "merchant_enabled_context owqnd {:?}",
        merchant_enabled_context
    );

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
                .collect::<Vec<serde_json::Value>>(),
        ),
    );

    // Fetch the dimension keys declared by the imported config so that the
    // evaluation context only carries fields the SuperTOML actually consults.
    // If the fetch fails we fall back to the current behaviour (pass
    // everything) — `filter_to_dimensions` is a no-op on an empty set.
    let dimension_keys = state
        .superposition_service
        .get_dimension_keys()
        .await
        .unwrap_or_default();

    let eligible_connector_set: std::collections::HashSet<String> = merchant_enabled_context
        .get_eligible_connectors()
        .into_iter()
        .collect();

    let mut superposition_evaluated_connectors: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for (payment_method, payment_method_type) in active_payment_methods {
        for connector in &all_active_connectors {
            let capture_method = payment_context
                .payment_attempt
                .as_ref()
                .and_then(|pa| pa.capture_method)
                .map(|capture_method| capture_method.to_string())
                .unwrap_or_else(|| UNSPECIFIED_CONTEXT_VALUE.to_string());

            let payment_context_for_superposition = PaymentContextSuperposition {
                currency: payment_intent
                    .currency
                    .map(|currency| currency.to_string())
                    .unwrap_or_else(|| UNSPECIFIED_CONTEXT_VALUE.to_string()),
                // The generated payment-filter SuperTOML uses the pm_filters leaf key
                // (`credit`, `debit`, `sepa`, etc.) as the `payment_method` dimension.
                // That is Hyperswitch's payment_method_type at runtime.
                payment_method: payment_method_type.to_string(),
                country: payment_context
                    .billing_address
                    .as_ref()
                    .and_then(|c| c.country)
                    .map(|country| country.to_string())
                    .unwrap_or_else(|| UNSPECIFIED_CONTEXT_VALUE.to_string()),
                capture_method,
                connector: connector.clone(),
            };

            println!(
                "payment_context_for_superposition {:?}",
                payment_context_for_superposition
            );

            // Build the evaluation context for this connector from the shared dimensions plus the
            // per-payment context fields.
            let mut config_context = external_services::superposition::ConfigContext::new()
                .with("connector", &payment_context_for_superposition.connector);
            config_context =
                config_context.with("currency", &payment_context_for_superposition.currency);
            config_context = config_context.with(
                "payment_method",
                &payment_context_for_superposition.payment_method,
            );
            config_context =
                config_context.with("country", &payment_context_for_superposition.country);
            config_context = config_context.with(
                "capture_method",
                &payment_context_for_superposition.capture_method,
            );

            // Drop context keys that the imported config does not declare as
            // dimensions (e.g. `amount`, `payment_method_type` if absent).
            // `filter_to_dimensions` is a no-op when `dimension_keys` is empty,
            // so a failed dimension fetch preserves the previous pass-all behaviour.
            config_context = config_context.filter_to_dimensions(&dimension_keys);

            let resolved_config = state
                .superposition_service
                .resolve_full_config(Some(&config_context), None)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable_lazy(|| {
                    format!("Failed to resolve superposition config for connector: {connector}")
                })?;

            // If Superposition resolved this connector as allowed/processing, record it.
            // `payment_action` is the target contract, while `payments_enabled` and
            // `is_processing` are accepted for compatibility with the generated file and
            // earlier validator experiments.
            let is_connector_allowed = resolve_payment_action(&resolved_config);

            if is_connector_allowed {
                superposition_evaluated_connectors.insert(connector.clone());
            }
        }
    }

    println!("eligible_connector_set {:?}", eligible_connector_set);
    println!(
        "evaluated_connector_set {:?}",
        superposition_evaluated_connectors
    );

    let mut missing_in_superposition = eligible_connector_set
        .difference(&superposition_evaluated_connectors)
        .cloned()
        .collect::<Vec<String>>();
    missing_in_superposition.sort();

    let mut extra_in_superposition = superposition_evaluated_connectors
        .difference(&eligible_connector_set)
        .cloned()
        .collect::<Vec<String>>();
    extra_in_superposition.sort();

    if !missing_in_superposition.is_empty() || !extra_in_superposition.is_empty() {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::PreconditionFailed {
                message: format!(
                    "Connector mismatch between eligible connectors and superposition evaluated connectors. \
                     eligible_connector_set: {eligible_connector_set:?}, \
                     evaluated_connector_set: {superposition_evaluated_connectors:?}, \
                     missing_in_superposition: [{}], extra_in_superposition: [{}]",
                    missing_in_superposition.join(", "),
                    extra_in_superposition.join(", ")
                ),
            }
        ));
    }

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
