use api_models::user::theme as theme_api;
use common_utils::{ext_traits::Encode, types::theme::ThemeLineage};
use diesel_models::user::theme::ThemeNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::api::ApplicationResponse;
use uuid::Uuid;

use crate::{
    core::errors::{StorageErrorExt, UserErrors, UserResponse},
    routes::SessionState,
    utils::user::theme as theme_utils,
};

pub async fn get_theme_using_lineage(
    state: SessionState,
    lineage: ThemeLineage,
) -> UserResponse<theme_api::GetThemeResponse> {
    let theme = state
        .global_store
        .find_theme_by_lineage(lineage)
        .await
        .to_not_found_response(UserErrors::ThemeNotFound)?;

    let file = theme_utils::retrieve_file_from_theme_bucket(
        &state,
        &theme_utils::get_theme_file_key(&theme.theme_id),
    )
    .await?;

    let parsed_data =
        serde_json::from_slice(&file).change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::Json(theme_api::GetThemeResponse {
        theme_id: theme.theme_id,
        theme_name: theme.theme_name,
        entity_type: theme.entity_type,
        tenant_id: theme.tenant_id,
        org_id: theme.org_id,
        merchant_id: theme.merchant_id,
        profile_id: theme.profile_id,
        theme_data: parsed_data,
    }))
}

pub async fn get_theme_using_theme_id(
    state: SessionState,
    theme_id: String,
) -> UserResponse<theme_api::GetThemeResponse> {
    let theme = state
        .global_store
        .find_theme_by_theme_id(theme_id.clone())
        .await
        .to_not_found_response(UserErrors::ThemeNotFound)?;

    let file = theme_utils::retrieve_file_from_theme_bucket(
        &state,
        &theme_utils::get_theme_file_key(&theme_id),
    )
    .await?;

    let parsed_data =
        serde_json::from_slice(&file).change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::Json(theme_api::GetThemeResponse {
        theme_id: theme.theme_id,
        theme_name: theme.theme_name,
        entity_type: theme.entity_type,
        tenant_id: theme.tenant_id,
        org_id: theme.org_id,
        merchant_id: theme.merchant_id,
        profile_id: theme.profile_id,
        theme_data: parsed_data,
    }))
}

pub async fn upload_file_to_theme_storage(
    state: SessionState,
    theme_id: String,
    request: theme_api::UploadFileRequest,
) -> UserResponse<()> {
    let db_theme = state
        .global_store
        .find_theme_by_lineage(request.lineage)
        .await
        .to_not_found_response(UserErrors::ThemeNotFound)?;

    if theme_id != db_theme.theme_id {
        return Err(UserErrors::ThemeNotFound.into());
    }

    theme_utils::upload_file_to_theme_bucket(
        &state,
        &theme_utils::get_specific_file_key(&theme_id, &request.asset_name),
        request.asset_data.clone(),
    )
    .await?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn create_theme(
    state: SessionState,
    request: theme_api::CreateThemeRequest,
) -> UserResponse<theme_api::GetThemeResponse> {
    theme_utils::validate_lineage(&state, &request.lineage).await?;

    let new_theme = ThemeNew::new(
        Uuid::new_v4().to_string(),
        request.theme_name,
        request.lineage,
    );

    let db_theme = state
        .global_store
        .insert_theme(new_theme)
        .await
        .to_duplicate_response(UserErrors::ThemeAlreadyExists)?;

    theme_utils::upload_file_to_theme_bucket(
        &state,
        &theme_utils::get_theme_file_key(&db_theme.theme_id),
        request
            .theme_data
            .encode_to_vec()
            .change_context(UserErrors::InternalServerError)?,
    )
    .await?;

    let file = theme_utils::retrieve_file_from_theme_bucket(
        &state,
        &theme_utils::get_theme_file_key(&db_theme.theme_id),
    )
    .await?;

    let parsed_data =
        serde_json::from_slice(&file).change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::Json(theme_api::GetThemeResponse {
        theme_id: db_theme.theme_id,
        entity_type: db_theme.entity_type,
        tenant_id: db_theme.tenant_id,
        org_id: db_theme.org_id,
        merchant_id: db_theme.merchant_id,
        profile_id: db_theme.profile_id,
        theme_name: db_theme.theme_name,
        theme_data: parsed_data,
    }))
}

pub async fn update_theme(
    state: SessionState,
    theme_id: String,
    request: theme_api::UpdateThemeRequest,
) -> UserResponse<theme_api::GetThemeResponse> {
    let db_theme = state
        .global_store
        .find_theme_by_lineage(request.lineage)
        .await
        .to_not_found_response(UserErrors::ThemeNotFound)?;

    if theme_id != db_theme.theme_id {
        return Err(UserErrors::ThemeNotFound.into());
    }

    theme_utils::upload_file_to_theme_bucket(
        &state,
        &theme_utils::get_theme_file_key(&db_theme.theme_id),
        request
            .theme_data
            .encode_to_vec()
            .change_context(UserErrors::InternalServerError)?,
    )
    .await?;

    let file = theme_utils::retrieve_file_from_theme_bucket(
        &state,
        &theme_utils::get_theme_file_key(&db_theme.theme_id),
    )
    .await?;

    let parsed_data =
        serde_json::from_slice(&file).change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::Json(theme_api::GetThemeResponse {
        theme_id: db_theme.theme_id,
        entity_type: db_theme.entity_type,
        tenant_id: db_theme.tenant_id,
        org_id: db_theme.org_id,
        merchant_id: db_theme.merchant_id,
        profile_id: db_theme.profile_id,
        theme_name: db_theme.theme_name,
        theme_data: parsed_data,
    }))
}

pub async fn delete_theme(
    state: SessionState,
    theme_id: String,
    lineage: ThemeLineage,
) -> UserResponse<()> {
    state
        .global_store
        .delete_theme_by_lineage_and_theme_id(theme_id.clone(), lineage)
        .await
        .to_not_found_response(UserErrors::ThemeNotFound)?;

    // TODO: Delete theme folder from the theme storage.
    // Currently there is no simple or easy way to delete a whole folder from S3.
    // So, we are not deleting the theme folder from the theme storage.

    Ok(ApplicationResponse::StatusOk)
}
