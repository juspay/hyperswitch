#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use std::sync::Arc;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use api_models;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use common_enums::enums;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use common_utils::{
    crypto::{DecodeMessage, EncodeMessage, GcmAes256},
    errors::CustomResult,
    ext_traits::{BytesExt, Encode, StringExt},
    id_type,
};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use error_stack::ResultExt;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use hyperswitch_domain_models;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use masking::{JsonMaskStrategy, Secret};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use router_env::{instrument, logger, tracing, Flow};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use serde::Serialize;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
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
pub async fn create_vault_token_core(
    state: SessionState,
    merchant_account: &domain::MerchantAccount,
    merchant_key_store: &domain::MerchantKeyStore,
    req: api_models::tokenization::GenericTokenizationRequest,
) -> RouterResponse<api_models::tokenization::GenericTokenizationResponse> {
    // Generate a unique vault ID
    let vault_id = domain::VaultId::generate(uuid::Uuid::now_v7().to_string());
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let customer_id = req.customer_id.clone();
    // Create vault request
    let payload = pm_types::AddVaultRequest {
        entity_id: merchant_account.get_id().to_owned(),
        vault_id: vault_id.clone(),
        data: req.token_request.clone(),
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
        customer_id: customer_id.clone(),
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
        api_models::tokenization::GenericTokenizationResponse {
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
    query: id_type::GlobalTokenId,
) -> CustomResult<serde_json::Value, errors::ApiErrorResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let tokenization_record = db
        .get_entity_id_vault_id_by_token_id(
            &query,
            &(merchant_key_store.clone()),
            key_manager_state,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get tokenization record")?;

    if tokenization_record.flag == enums::TokenizationFlag::Disabled {
        return Err(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Tokenization is disabled for the id".to_string(),
        }
        .into());
    }

    let vault_request = pm_types::VaultRetrieveRequest {
        entity_id: tokenization_record.merchant_id.clone(),
        vault_id: hyperswitch_domain_models::payment_methods::VaultId::generate(
            tokenization_record.locker_id.clone(),
        ),
    };

    let vault_data = pm_vault::retrieve_value_from_vault(&state, vault_request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve vault data")?;

    let data_json = vault_data
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    // Create the response
    Ok(data_json)
}
