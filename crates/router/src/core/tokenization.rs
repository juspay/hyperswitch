use std::sync::Arc;
use error_stack::ResultExt;
use masking::Secret;
use serde::Serialize;
use actix_web::{web, HttpRequest, HttpResponse};
use api_models;
use common_enums::enums;
use common_utils::{
    crypto::{DecodeMessage, EncodeMessage, GcmAes256},
    ext_traits::{BytesExt, Encode, StringExt},
    id_type,
};
use error_stack::{IntoReport, ResultExt};
use hyperswitch_domain_models;
use masking::Secret;
use router_env::{instrument, logger, tracing, Flow};
use serde::Serialize;

use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult},
        payment_methods::vault as pm_vault,
        tokenization,
    },
    routes::{app::StorageInterface, AppState, SessionState},
    services::{self, api as api_service, authentication as auth},
    types::{api, domain, payment_methods as pm_types},
};

#[instrument(skip_all)]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub async fn create_vault_token_core<T: Serialize + std::fmt::Debug>(
    state: SessionState,
    merchant_account: &domain::MerchantAccount,
    merchant_key_store: &domain::MerchantKeyStore,
    req: T,
) -> RouterResponse<api_models::tokenization::TokenizationResponse> {
    // Generate a unique vault ID
    let vault_id = domain::VaultId::generate(uuid::Uuid::now_v7().to_string());
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    // Create vault request
    let payload = pm_types::AddVaultRequest {
        entity_id: merchant_account.get_id().to_owned(),
        vault_id: vault_id.clone(),
        data: req,
        ttl: state.conf.locker.ttl_for_storage_in_secs,
    }
    .encode_to_vec()
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to encode Request")?;
    // Call the vault service
    let resp = pm_vault::call_to_vault::<pm_types::AddVault>(&state, payload.clone())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Call to vault failed")?;
    // Parse the response
    let stored_resp: pm_types::AddVaultResponse = resp
        .parse_struct("AddVaultResponse")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse data into AddVaultResponse")?;

    // Create new tokenization record
    let tokenization_new = hyperswitch_domain_models::tokenization::Tokenization {
        id: id_type::GlobalTokenId::generate(&state.conf.cell_information.id),
        merchant_id: merchant_account.get_id().clone(),
        locker_id: stored_resp.vault_id.get_string_repr().to_string(),
        created_at: common_utils::date_time::now(),
        updated_at: common_utils::date_time::now(),
        flag: enums::TokenizationFlag::Enabled,
        version: enums::ApiVersion::V2,
    };

    // Insert into database
    let tokenization = db
        .insert_tokenization(
            tokenization_new,
            &(merchant_key_store.clone()),
            key_manager_state,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert tokenization record")?;

    // Convert to TokenizationResponse
    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        api_models::tokenization::TokenizationResponse {
            id: tokenization.id,
            created_at: tokenization.created_at,
            flag: tokenization.flag,
        },
    ))
}

#[instrument(skip_all)]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub async fn get_token_vault_core(
    state: SessionState,
    merchant_account: &domain::MerchantAccount,
    merchant_key_store: &domain::MerchantKeyStore,
    query_params: (id_type::GlobalTokenId, bool),
) -> RouterResponse<serde_json::Value> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    // Get the tokenization record from the database
    let tokenization = db
        .get_entity_id_vault_id_by_token_id(
            &query_params.0,
            &(merchant_key_store.clone()),
            key_manager_state,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get tokenization record")?;

    if tokenization.flag == enums::TokenizationFlag::Disabled {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Tokenization is disabled".to_string(),
        }
        .into());
    }

    let vault_request = pm_types::VaultRetrieveRequest {
        entity_id: tokenization.merchant_id.clone(),
        vault_id: hyperswitch_domain_models::payment_methods::VaultId::generate(
            tokenization.locker_id.clone(),
        ),
    };

    let vault_data = pm_vault::retrive_value_from_vault(&state, vault_request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve vault data")?;

    let data_json = vault_data
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    // Mask sensitive data if needed
    let response_data = if !query_params.1 {
        mask_sensitive_data(data_json)
    } else {
        data_json
    };

    // Create the response
    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response_data,
    ))
}

fn mask_sensitive_data(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut masked = serde_json::Map::new();
            for (key, val) in map {
                // Recursively mask nested objects
                masked.insert(key, mask_sensitive_data(val));
            }
            serde_json::Value::Object(masked)
        }
        serde_json::Value::Array(arr) => {
            // Recursively mask each element in arrays
            let masked_arr = arr.into_iter().map(mask_sensitive_data).collect();
            serde_json::Value::Array(masked_arr)
        }
        // For primitive values (string, number, boolean), replace with "masked"
        serde_json::Value::String(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Bool(_) => serde_json::Value::String("masked".to_string()),
        // Keep null as is
        serde_json::Value::Null => serde_json::Value::Null,
    }
}
