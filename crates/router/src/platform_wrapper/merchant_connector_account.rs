use common_utils::id_type;

use crate::{core::errors, db::StorageInterface, types::domain};

#[cfg(feature = "v1")]
pub async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
    db: &dyn StorageInterface,
    processor: &domain::Processor,
    merchant_connector_id: &id_type::MerchantConnectorAccountId,
) -> errors::CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
    db.find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        processor.get_account().get_id(),
        merchant_connector_id,
        processor.get_key_store(),
    )
    .await
}

#[cfg(feature = "v2")]
pub async fn find_merchant_connector_account_by_id(
    db: &dyn StorageInterface,
    processor: &domain::Processor,
    merchant_connector_id: &id_type::MerchantConnectorAccountId,
) -> errors::CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
    db.find_merchant_connector_account_by_id(merchant_connector_id, processor.get_key_store())
        .await
}
