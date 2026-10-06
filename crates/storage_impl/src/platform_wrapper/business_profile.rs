use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    business_profile::{Profile, ProfileInterface},
    platform::Processor,
};

use crate::StorageError;

pub async fn find_business_profile_by_profile_id<S>(
    store: &S,
    processor: &Processor,
    profile_id: &common_utils::id_type::ProfileId,
) -> CustomResult<Profile, StorageError>
where
    S: ProfileInterface<Error = StorageError> + ?Sized,
{
    store
        .find_business_profile_by_profile_id(processor.get_key_store(), profile_id)
        .await
}
