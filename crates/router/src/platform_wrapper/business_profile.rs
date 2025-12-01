use hyperswitch_domain_models::business_profile::Profile;

use crate::{core::errors, db::StorageInterface, types::domain};

pub async fn find_business_profile_by_profile_id(
    db: &dyn StorageInterface,
    processor: &domain::Processor,
    profile_id: &common_utils::id_type::ProfileId,
) -> errors::CustomResult<Profile, errors::StorageError> {
    db.find_business_profile_by_profile_id(processor.get_key_store(), profile_id)
        .await
}
