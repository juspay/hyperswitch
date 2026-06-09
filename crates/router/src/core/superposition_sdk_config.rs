use api_models::{
    enums as api_enums,
    superposition_sdk_config::{
        SdkPaymentMethod, SdkPaymentMethodType, SuperPositionConfigResponse,
    },
};
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use serde_json::Map;

use crate::{
    consts::superposition::DYNAMIC_FIELDS,
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        payment_methods::cards::{build_merchant_enabled_pms_context, MerchantEnabledPmsContext},
        payments::helpers,
    },
    routes::SessionState,
    types::domain,
};

pub async fn get_superposition_sdk_config(
    state: SessionState,
    platform: domain::Platform,
    client_secret: String,
) -> RouterResponse<SuperPositionConfigResponse> {
    let merchant_account = platform.get_processor().get_account();
    let db = &*state.store;
    let payment_intent = helpers::verify_payment_intent_time_and_client_secret(
        &state,
        &platform,
        Some(client_secret),
    )
    .await?;

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
                &state,
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
                &state,
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

    let profile_id = payment_intent
        .as_ref()
        .and_then(|pi| pi.profile_id.clone())
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Profile id not found".to_string(),
        })?;

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
        payment_intent.as_ref(),
        payment_attempt.as_ref(),
        billing_address.as_ref(),
        shipping_address.as_ref(),
        customer.as_ref(),
        is_cit_transaction,
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

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        SuperPositionConfigResponse {
            raw_configs,
            resolved_configs: None,
            context_used: dimension_filter,
            payment_methods: Some(payment_methods),
        },
    ))
}

fn translate_to_sdk_payment_methods(
    state: &SessionState,
    pms_ctx: &MerchantEnabledPmsContext,
) -> error_stack::Result<Vec<SdkPaymentMethod>, errors::ApiErrorResponse> {
    let mut payment_methods = vec![];

    // 1. Payment experiences (wallets, paylater, etc.)
    for (payment_method, pmt_map) in &pms_ctx.payment_experiences_consolidated_hm {
        let mut payment_method_types = vec![];
        for (payment_method_type, pe_map) in pmt_map {
            for (payment_experience, connectors) in pe_map {
                payment_method_types.push(SdkPaymentMethodType {
                    payment_method_type: *payment_method_type,
                    payment_experience: Some(*payment_experience),
                    eligible_connectors: connectors.clone(),
                    card_networks: None,
                    bank_names: None,
                    bank_debits: None,
                    bank_transfers: None,
                });
            }
        }
        if !payment_method_types.is_empty() {
            payment_methods.push(SdkPaymentMethod {
                payment_method: *payment_method,
                payment_method_types,
            });
        }
    }

    // 2. Card networks (cards)
    for (payment_method, pmt_map) in &pms_ctx.card_networks_consolidated_hm {
        let mut payment_method_types = vec![];
        for (payment_method_type, card_network_map) in pmt_map {
            let mut card_networks = vec![];
            let mut eligible_connectors = std::collections::HashSet::new();
            for (card_network, connectors) in card_network_map {
                card_networks.push(card_network.clone());
                for connector in connectors {
                    eligible_connectors.insert(connector.clone());
                }
            }
            payment_method_types.push(SdkPaymentMethodType {
                payment_method_type: *payment_method_type,
                payment_experience: None,
                eligible_connectors: eligible_connectors.into_iter().collect(),
                card_networks: Some(card_networks),
                bank_names: None,
                bank_debits: None,
                bank_transfers: None,
            });
        }
        if !payment_method_types.is_empty() {
            payment_methods.push(SdkPaymentMethod {
                payment_method: *payment_method,
                payment_method_types,
            });
        }
    }

    // 3. Banks (bank redirect)
    let mut bank_redirect_types = vec![];
    for (payment_method_type, connectors) in &pms_ctx.banks_consolidated_hm {
        let bank_names = crate::core::payment_methods::cards::get_banks(
            state,
            *payment_method_type,
            connectors.clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
        bank_redirect_types.push(SdkPaymentMethodType {
            payment_method_type: *payment_method_type,
            payment_experience: None,
            eligible_connectors: connectors.clone(),
            card_networks: None,
            bank_names: Some(bank_names),
            bank_debits: None,
            bank_transfers: None,
        });
    }
    if !bank_redirect_types.is_empty() {
        payment_methods.push(SdkPaymentMethod {
            payment_method: api_enums::PaymentMethod::BankRedirect,
            payment_method_types: bank_redirect_types,
        });
    }

    // 4. Bank debits
    let mut bank_debit_types = vec![];
    for (payment_method_type, connectors) in &pms_ctx.bank_debits_consolidated_hm {
        bank_debit_types.push(SdkPaymentMethodType {
            payment_method_type: *payment_method_type,
            payment_experience: None,
            eligible_connectors: connectors.clone(),
            card_networks: None,
            bank_names: None,
            bank_debits: Some(api_models::payment_methods::BankDebitTypes {
                eligible_connectors: connectors.clone(),
            }),
            bank_transfers: None,
        });
    }
    if !bank_debit_types.is_empty() {
        payment_methods.push(SdkPaymentMethod {
            payment_method: api_enums::PaymentMethod::BankDebit,
            payment_method_types: bank_debit_types,
        });
    }

    // 5. Bank transfers
    let mut bank_transfer_types = vec![];
    for (payment_method_type, connectors) in &pms_ctx.bank_transfer_consolidated_hm {
        bank_transfer_types.push(SdkPaymentMethodType {
            payment_method_type: *payment_method_type,
            payment_experience: None,
            eligible_connectors: connectors.clone(),
            card_networks: None,
            bank_names: None,
            bank_debits: None,
            bank_transfers: Some(api_models::payment_methods::BankTransferTypes {
                eligible_connectors: connectors.clone(),
            }),
        });
    }
    if !bank_transfer_types.is_empty() {
        payment_methods.push(SdkPaymentMethod {
            payment_method: api_enums::PaymentMethod::BankTransfer,
            payment_method_types: bank_transfer_types,
        });
    }

    Ok(payment_methods)
}
