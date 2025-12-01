use common_utils::id_type;

use crate::{core::errors, db::StorageInterface, types::domain};

#[cfg(feature = "v1")]
pub async fn find_payment_method(
    db: &dyn StorageInterface,
    provider: &domain::Provider,
    payment_method_id: &str,
) -> errors::CustomResult<domain::PaymentMethod, errors::StorageError> {
    db.find_payment_method(
        provider.get_key_store(),
        payment_method_id,
        provider.get_account().storage_scheme,
    )
    .await
}

#[cfg(feature = "v2")]
pub async fn find_payment_method(
    db: &dyn StorageInterface,
    provider: &domain::Provider,
    payment_method_id: &id_type::GlobalPaymentMethodId,
) -> errors::CustomResult<domain::PaymentMethod, errors::StorageError> {
    db.find_payment_method(
        provider.get_key_store(),
        payment_method_id,
        provider.get_account().storage_scheme,
    )
    .await
}

#[cfg(feature = "v1")]
pub async fn find_payment_method_by_customer_id_merchant_id_list(
    db: &dyn StorageInterface,
    provider: &domain::Provider,
    customer_id: &id_type::CustomerId,
    limit: Option<i64>,
) -> errors::CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
    db.find_payment_method_by_customer_id_merchant_id_list(
        provider.get_key_store(),
        customer_id,
        provider.get_account().get_id(),
        limit,
    )
    .await
}
