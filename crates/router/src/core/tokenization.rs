use std::sync::Arc;
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::Serialize;
use actix_web::{web, HttpRequest, HttpResponse};
use crate::{
    core::{
        errors::{self, RouterResult, RouterResponse},
        tokenization,
        payment_methods::vault as pm_vault,
    },
    services::{self, api as api_service, authentication as auth,},
    types::{
        api,
        domain,
        payment_methods as pm_types,
    },
    routes::{app::StorageInterface, SessionState, AppState},
};
use router_env::{instrument, tracing, Flow, logger};
use hyperswitch_domain_models;
use api_models;
use common_utils::{
    crypto::{DecodeMessage, EncodeMessage, GcmAes256},
    ext_traits::{BytesExt, Encode, StringExt},
    id_type,
};
use common_enums::enums;


#[instrument(skip_all)]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub async fn create_vault_token_core<T: Serialize +  std::fmt::Debug>(
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
    let tokenization = db.insert_tokenization(
            tokenization_new,
            &(merchant_key_store.clone()),
            &key_manager_state,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert tokenization record")?;

    // Convert to TokenizationResponse
    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        api_models::tokenization::TokenizationResponse {
            id: tokenization.id,
            created_at: tokenization.created_at,
            flag: tokenization.flag
        }
    ))
}
