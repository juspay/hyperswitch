use hyperswitch_domain_models::platform::Provider;

use crate::{core::errors, db::StorageInterface, types::storage};

// Find mandate by merchant_id and mandate_id using Provider context
pub async fn find_by_merchant_id_and_mandate_id(
    db: &dyn StorageInterface,
    provider: &Provider,
    mandate_id: &str,
) -> errors::CustomResult<storage::Mandate, errors::StorageError> {
    db.find_mandate_by_merchant_id_mandate_id(
        provider.get_account().get_id(),
        mandate_id,
        provider.get_account().storage_scheme,
    )
    .await
}
