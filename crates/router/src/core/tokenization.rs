#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use common_enums::enums;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use common_utils::{
    crypto::{DecodeMessage, EncodeMessage, GcmAes256},
    errors::CustomResult,
    ext_traits::{BytesExt, Encode, StringExt},
    fp_utils::when,
    id_type,
};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use error_stack::ResultExt;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use router_env::{instrument, logger, tracing, Flow};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use serde::Serialize;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult},
        payment_methods::vault as pm_vault,
    },
    db::errors::StorageErrorExt,
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
    let customer_id = req.customer_id.clone();
    // Create vault request
    let payload = pm_types::AddVaultRequest {
        entity_id: req.customer_id.to_owned(),
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
        .insert_tokenization(tokenization_new, &(merchant_key_store.clone()))
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

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn delete_tokenized_data_core(
    state: SessionState,
    platform: domain::Platform,
    token_id: &id_type::GlobalTokenId,
    payload: api_models::tokenization::DeleteTokenDataRequest,
) -> RouterResponse<api_models::tokenization::DeleteTokenDataResponse> {
    let db = &*state.store;

    // Retrieve the tokenization record
    let tokenization_record = db
        .get_entity_id_vault_id_by_token_id(token_id, platform.get_processor().get_key_store())
        .await
        .to_not_found_response(errors::ApiErrorResponse::TokenizationRecordNotFound {
            id: token_id.get_string_repr().to_string(),
        })
        .attach_printable("Failed to get tokenization record")?;

    when(
        tokenization_record.customer_id != payload.customer_id,
        || {
            Err(errors::ApiErrorResponse::UnprocessableEntity {
                message: "Tokenization record does not belong to the customer".to_string(),
            })
        },
    )?;

    when(tokenization_record.is_disabled(), || {
        Err(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Tokenization is already disabled for the id".to_string(),
        })
    })?;

    //fetch locker id
    let vault_id = domain::VaultId::generate(tokenization_record.locker_id.clone());
    //delete card from vault
    pm_vault::delete_payment_method_data_from_vault_internal(
        &state,
        &platform,
        vault_id,
        &tokenization_record.customer_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to delete payment method from vault")?;

    //update the status with Disabled
    let tokenization_update = hyperswitch_domain_models::tokenization::TokenizationUpdate::DeleteTokenizationRecordUpdate {
        flag: Some(enums::TokenizationFlag::Disabled),
    };
    db.update_tokenization_record(
        tokenization_record,
        tokenization_update,
        platform.get_processor().get_key_store(),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update tokenization record")?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        api_models::tokenization::DeleteTokenDataResponse {
            id: token_id.clone(),
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

    let tokenization_record = db
        .get_entity_id_vault_id_by_token_id(&query, &(merchant_key_store.clone()))
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
        entity_id: tokenization_record.customer_id.clone(),
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
