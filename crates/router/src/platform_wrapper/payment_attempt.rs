use common_utils::id_type;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptNew;
use hyperswitch_domain_models::{payments::payment_attempt::PaymentAttempt, platform::Provider};

use crate::{core::errors, db::StorageInterface};

#[cfg(feature = "v1")]
pub async fn insert_payment_attempt(
    db: &dyn StorageInterface,
    provider: &Provider,
    payment_attempt: PaymentAttemptNew,
) -> errors::CustomResult<PaymentAttempt, errors::StorageError> {
    db.insert_payment_attempt(payment_attempt, provider.get_account().storage_scheme)
        .await
}

#[cfg(feature = "v2")]
pub async fn insert_payment_attempt(
    db: &dyn StorageInterface,
    provider: &Provider,
    payment_attempt: PaymentAttempt,
) -> errors::CustomResult<PaymentAttempt, errors::StorageError> {
    db.insert_payment_attempt(
        provider.get_key_store(),
        payment_attempt,
        provider.get_account().storage_scheme,
    )
    .await
}

#[cfg(feature = "v1")]
pub async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
    db: &dyn StorageInterface,
    provider: &Provider,
    payment_id: &id_type::PaymentId,
    attempt_id: &str,
) -> errors::CustomResult<PaymentAttempt, errors::StorageError> {
    db.find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        payment_id,
        provider.get_account().get_id(),
        attempt_id,
        provider.get_account().storage_scheme,
    )
    .await
}
