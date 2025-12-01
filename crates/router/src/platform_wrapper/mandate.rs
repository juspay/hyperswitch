use crate::{
    core::errors,
    db::StorageInterface,
    types::{domain, storage},
};

pub async fn find_by_merchant_id_and_mandate_id(
    db: &dyn StorageInterface,
    provider: &domain::Provider,
    mandate_id: &str,
) -> errors::CustomResult<storage::Mandate, errors::StorageError> {
    db.find_mandate_by_merchant_id_mandate_id(
        provider.get_account().get_id(),
        mandate_id,
        provider.get_account().storage_scheme,
    )
    .await
}

pub async fn find_mandate_by_merchant_id_mandate_id(
    db: &dyn StorageInterface,
    provider: &domain::Provider,
    mandate_id: &str,
) -> errors::CustomResult<storage::Mandate, errors::StorageError> {
    db.find_mandate_by_merchant_id_mandate_id(
        provider.get_account().get_id(),
        mandate_id,
        provider.get_account().storage_scheme,
    )
    .await
}
