use hyperswitch_domain_models::{
    payments::payment_attempt::{PaymentAttempt, PaymentAttemptNew},
    platform::Provider,
};

use crate::{core::errors, db::StorageInterface};

// Insert a new payment attempt using Provider context
#[cfg(feature = "v1")]
pub async fn insert(
    db: &dyn StorageInterface,
    provider: &Provider,
    payment_attempt: PaymentAttemptNew,
) -> errors::CustomResult<PaymentAttempt, errors::StorageError> {
    db.insert_payment_attempt(payment_attempt, provider.get_account().storage_scheme)
        .await
}

// Insert a new payment attempt using Provider context
#[cfg(feature = "v2")]
pub async fn insert(
    db: &dyn StorageInterface,
    provider: &Provider,
    payment_attempt: PaymentAttemptNew,
) -> errors::CustomResult<PaymentAttempt, errors::StorageError> {
    db.insert_payment_attempt(
        orovider.get_key_store(),
        payment_attempt,
        provider.get_account().storage_scheme,
    )
    .await
}
