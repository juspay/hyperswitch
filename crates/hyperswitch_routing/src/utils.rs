use hyperswitch_domain_models::{errors::api_error_response as errors, business_profile::Profile, merchant_key_store::MerchantKeyStore};
use crate::errors::RouterResult;
use common_utils::types::keymanager::KeyManagerState;
use crate::state::RoutingStorageInterface;
use common_utils::ext_traits::AsyncExt;
use storage_impl::errors::StorageErrorExt;
pub use hyperswitch_domain_models::business_profile::{GetProfileId, filter_objects_based_on_profile_id_list, validate_profile_id_from_auth_layer};

/// Validate whether the profile_id exists and is associated with the merchant_id
pub async fn validate_and_get_business_profile(
    db: &dyn RoutingStorageInterface,
    key_manager_state: &KeyManagerState,
    merchant_key_store: &MerchantKeyStore,
    profile_id: Option<&common_utils::id_type::ProfileId>,
    merchant_id: &common_utils::id_type::MerchantId,
) -> RouterResult<Option<Profile>> {
    profile_id
        .async_map(|profile_id| async {
            db.find_business_profile_by_profile_id(
                key_manager_state,
                merchant_key_store,
                profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })
        })
        .await
        .transpose()?
        .map(|business_profile| {
            // Check if the merchant_id of business profile is same as the current merchant_id
            if business_profile.merchant_id.ne(merchant_id) {
                Err(errors::ApiErrorResponse::AccessForbidden {
                    resource: business_profile.get_id().get_string_repr().to_owned(),
                }
                .into())
            } else {
                Ok(business_profile)
            }
        })
        .transpose()
}