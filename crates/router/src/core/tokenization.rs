use std::sync::Arc;
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use router_env::{instrument, tracing};
use serde::Serialize;
use actix_web::{web, HttpRequest, HttpResponse};
use crate::{
    core::errors::{self, RouterResult},
    routes::AppState,
    services::{self, api, authentication as auth},
    types::{
        api,
        domain,
        payment_methods as pm_types,
    },
    hyperswitch_domain_models,
};

#[instrument(skip_all, fields(flow = ?Flow::TokenizeCard))]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub async fn create_token_vault_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api::TokenizationRequest>,
) -> HttpResponse {
    let flow = Flow::TokenizeCard;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, request, _| async move {
            create_vault_token_core(
                state.into(),
                &auth.merchant_account,
                request,
            )
            .await
        },
        &auth::V2ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all)]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
async fn create_vault_token_core<T: Serialize>(
    state: Arc<AppState>,
    merchant_account: &domain::MerchantAccount,
    req: T,
) -> RouterResult<api::TokenizationResponse> {
    // Generate a unique vault ID
    let vault_id = domain::VaultId::generate(uuid::Uuid::now_v7().to_string());

    // Create vault request
    let payload = pm_types::AddVaultRequest {
        entity_id: merchant_account.get_id().to_owned(),
        vault_id: vault_id.clone(),
        data: &req,
        ttl: state.conf.locker.ttl_for_storage_in_secs,
    }
    .encode_to_vec()
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to encode AddVaultRequest")?;
    //&state.conf.cell_information.id
    // Call the vault service
    let resp = services::tokenization::call_to_vault::<pm_types::AddVault>(&state, payload)
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
        id: domain::GlobalTokenId::generate(&state.conf.cell_information.id),
        merchant_id: merchant_account.merchant_id.clone(),
        locker_id: stored_resp.vault_id.to_string(),
        created_at: common_utils::date_time::now(),
        updated_at: common_utils::date_time::now(),
        flag: storage_enums::TokenizationFlag::Active,
        version: storage_enums::ApiVersion::V2,
    };

    // Insert into database
    let tokenization = state
        .store
        .insert_tokenization(tokenization_new)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert tokenization record")?;

    // Convert to TokenizationResponse
    Ok(api::TokenizationResponse {
        token: tokenization.id.to_string(),
        message: "Token created successfully".to_string(),
    })
}

#[instrument(skip_all)]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub async fn detokenize_card(
    state: Arc<AppState>,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    token: String,
) -> RouterResult<api::DetokenizationResponse> {
    // Call the detokenization service
    let detokenization_response = services::tokenization::detokenize_card(
        state,
        merchant_account,
        key_store,
        token,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to detokenize card")?;

    Ok(detokenization_response)
}
