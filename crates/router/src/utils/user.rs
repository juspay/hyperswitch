use diesel_models::enums::UserStatus;
use error_stack::ResultExt;

use crate::{
    core::errors::{UserErrors, UserResult},
    routes::AppState,
    services::authentication::UserFromToken,
    types::domain::MerchantAccount,
};

pub mod dashboard_metadata;
pub mod password;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;

impl UserFromToken {
    pub async fn get_merchant_account(&self, state: AppState) -> UserResult<MerchantAccount> {
        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &self.merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(UserErrors::MerchantIdNotFound)
                } else {
                    e.change_context(UserErrors::InternalServerError)
                }
            })?;
        let merchant_account = state
            .store
            .find_merchant_account_by_merchant_id(&self.merchant_id, &key_store)
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(UserErrors::MerchantIdNotFound)
                } else {
                    e.change_context(UserErrors::InternalServerError)
                }
            })?;
        Ok(merchant_account)
    }

    pub async fn get_user(&self, state: AppState) -> UserResult<diesel_models::user::User> {
        let user = state
            .store
            .find_user_by_id(&self.user_id)
            .await
            .change_context(UserErrors::InternalServerError)?;
        Ok(user)
    }
}

pub async fn get_merchant_ids_for_user(state: AppState, user_id: &str) -> UserResult<Vec<String>> {
    Ok(state
        .store
        .list_user_roles_by_user_id(user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into_iter()
        .filter_map(|ele| {
            if ele.status == UserStatus::Active {
                return Some(ele.merchant_id);
            }
            None
        })
        .collect())
}
