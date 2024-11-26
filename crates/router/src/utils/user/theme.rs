use common_utils::{id_type, types::theme::ThemeLineage};
use error_stack::ResultExt;
use hyperswitch_domain_models::merchant_key_store::MerchantKeyStore;

use crate::{
    core::errors::{StorageErrorExt, UserErrors, UserResult},
    routes::SessionState,
};

pub fn get_theme_dir_key(theme_id: &str) -> String {
    format!("themes/{}", theme_id)
}

pub fn get_specific_file_key(theme_id: &str, file_name: &str) -> String {
    format!("{}/{}", get_theme_dir_key(theme_id), file_name)
}

pub fn get_theme_file_key(theme_id: &str) -> String {
    get_specific_file_key(theme_id, "theme.json")
}

pub async fn retrieve_file_from_theme_bucket(
    state: &SessionState,
    path: &str,
) -> UserResult<Vec<u8>> {
    state
        .theme_storage_client
        .retrieve_file(path)
        .await
        .change_context(UserErrors::ErrorRetrievingFile)
}

pub async fn upload_file_to_theme_bucket(
    state: &SessionState,
    path: &str,
    data: Vec<u8>,
) -> UserResult<()> {
    state
        .theme_storage_client
        .upload_file(path, data)
        .await
        .change_context(UserErrors::InternalServerError)
}

pub async fn validate_lineage(state: &SessionState, lineage: &ThemeLineage) -> UserResult<()> {
    match lineage {
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

fn validate_tenant(state: &SessionState, tenant_id: &str) -> UserResult<()> {
    if state.tenant.tenant_id != tenant_id {
        return Err(UserErrors::InvalidThemeLineage("tenant_id".to_string()).into());
    }
    Ok(())
}

async fn validate_org(state: &SessionState, org_id: &id_type::OrganizationId) -> UserResult<()> {
    state
        .store
        .find_organization_by_org_id(org_id)
        .await
        .to_not_found_response(UserErrors::InvalidThemeLineage("org_id".to_string()))?;
    Ok(())
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
        .to_not_found_response(UserErrors::InvalidThemeLineage("profile_id".to_string()))?;
    Ok(())
}
