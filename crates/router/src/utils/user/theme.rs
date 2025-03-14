use std::path::PathBuf;

use common_enums::EntityType;
use common_utils::{ext_traits::AsyncExt, id_type, types::theme::ThemeLineage};
use diesel_models::user::theme::Theme;
use error_stack::ResultExt;
use hyperswitch_domain_models::merchant_key_store::MerchantKeyStore;

use crate::{
    core::errors::{StorageErrorExt, UserErrors, UserResult},
    routes::SessionState,
    services::authentication::UserFromToken,
};

fn get_theme_dir_key(theme_id: &str) -> PathBuf {
    ["themes", theme_id].iter().collect()
}

pub fn get_specific_file_key(theme_id: &str, file_name: &str) -> PathBuf {
    let mut path = get_theme_dir_key(theme_id);
    path.push(file_name);
    path
}

pub fn get_theme_file_key(theme_id: &str) -> PathBuf {
    get_specific_file_key(theme_id, "theme.json")
}

fn path_buf_to_str(path: &PathBuf) -> UserResult<&str> {
    path.to_str()
        .ok_or(UserErrors::InternalServerError)
        .attach_printable(format!("Failed to convert path {:#?} to string", path))
}

pub async fn retrieve_file_from_theme_bucket(
    state: &SessionState,
    path: &PathBuf,
) -> UserResult<Vec<u8>> {
    state
        .theme_storage_client
        .retrieve_file(path_buf_to_str(path)?)
        .await
        .change_context(UserErrors::ErrorRetrievingFile)
}

pub async fn upload_file_to_theme_bucket(
    state: &SessionState,
    path: &PathBuf,
    data: Vec<u8>,
) -> UserResult<()> {
    state
        .theme_storage_client
        .upload_file(path_buf_to_str(path)?, data)
        .await
        .change_context(UserErrors::ErrorUploadingFile)
}

pub async fn validate_lineage(state: &SessionState, lineage: &ThemeLineage) -> UserResult<()> {
    match lineage {
        ThemeLineage::Tenant { tenant_id } => {
            validate_tenant(state, tenant_id)?;
            Ok(())
        }
        ThemeLineage::Organization { tenant_id, org_id } => {
            validate_tenant(state, tenant_id)?;
            validate_org(state, org_id).await?;
            Ok(())
        }
        ThemeLineage::Merchant {
            tenant_id,
            org_id,
            merchant_id,
        } => {
            validate_tenant(state, tenant_id)?;
            validate_org(state, org_id).await?;
            validate_merchant(state, org_id, merchant_id).await?;
            Ok(())
        }
        ThemeLineage::Profile {
            tenant_id,
            org_id,
            merchant_id,
            profile_id,
        } => {
            validate_tenant(state, tenant_id)?;
            validate_org(state, org_id).await?;
            let key_store = validate_merchant_and_get_key_store(state, org_id, merchant_id).await?;
            validate_profile(state, profile_id, merchant_id, &key_store).await?;
            Ok(())
        }
    }
}

fn validate_tenant(state: &SessionState, tenant_id: &id_type::TenantId) -> UserResult<()> {
    if &state.tenant.tenant_id != tenant_id {
        return Err(UserErrors::InvalidThemeLineage("tenant_id".to_string()).into());
    }
    Ok(())
}

async fn validate_org(state: &SessionState, org_id: &id_type::OrganizationId) -> UserResult<()> {
    state
        .accounts_store
        .find_organization_by_org_id(org_id)
        .await
        .to_not_found_response(UserErrors::InvalidThemeLineage("org_id".to_string()))
        .map(|_| ())
}

async fn validate_merchant_and_get_key_store(
    state: &SessionState,
    org_id: &id_type::OrganizationId,
    merchant_id: &id_type::MerchantId,
) -> UserResult<MerchantKeyStore> {
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &state.into(),
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(UserErrors::InvalidThemeLineage("merchant_id".to_string()))?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(&state.into(), merchant_id, &key_store)
        .await
        .to_not_found_response(UserErrors::InvalidThemeLineage("merchant_id".to_string()))?;

    if &merchant_account.organization_id != org_id {
        return Err(UserErrors::InvalidThemeLineage("merchant_id".to_string()).into());
    }

    Ok(key_store)
}

async fn validate_merchant(
    state: &SessionState,
    org_id: &id_type::OrganizationId,
    merchant_id: &id_type::MerchantId,
) -> UserResult<()> {
    validate_merchant_and_get_key_store(state, org_id, merchant_id)
        .await
        .map(|_| ())
}

async fn validate_profile(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    merchant_id: &id_type::MerchantId,
    key_store: &MerchantKeyStore,
) -> UserResult<()> {
    state
        .store
        .find_business_profile_by_merchant_id_profile_id(
            &state.into(),
            key_store,
            merchant_id,
            profile_id,
        )
        .await
        .to_not_found_response(UserErrors::InvalidThemeLineage("profile_id".to_string()))
        .map(|_| ())
}

pub async fn get_most_specific_theme_using_token_and_min_entity(
    state: &SessionState,
    user_from_token: &UserFromToken,
    min_entity: EntityType,
) -> UserResult<Option<Theme>> {
    get_most_specific_theme_using_lineage(
        state,
        ThemeLineage::new(
            min_entity,
            user_from_token
                .tenant_id
                .clone()
                .unwrap_or(state.tenant.tenant_id.clone()),
            user_from_token.org_id.clone(),
            user_from_token.merchant_id.clone(),
            user_from_token.profile_id.clone(),
        ),
    )
    .await
}

pub async fn get_most_specific_theme_using_lineage(
    state: &SessionState,
    lineage: ThemeLineage,
) -> UserResult<Option<Theme>> {
    match state
        .store
        .find_most_specific_theme_in_lineage(lineage)
        .await
    {
        Ok(theme) => Ok(Some(theme)),
        Err(e) => {
            if e.current_context().is_db_not_found() {
                Ok(None)
            } else {
                Err(e.change_context(UserErrors::InternalServerError))
            }
        }
    }
}

pub async fn get_theme_using_optional_theme_id(
    state: &SessionState,
    theme_id: Option<String>,
) -> UserResult<Option<Theme>> {
    match theme_id
        .async_map(|theme_id| state.store.find_theme_by_theme_id(theme_id))
        .await
        .transpose()
    {
        Ok(theme) => Ok(theme),
        Err(e) => {
            if e.current_context().is_db_not_found() {
                Ok(None)
            } else {
                Err(e.change_context(UserErrors::InternalServerError))
            }
        }
    }
}
