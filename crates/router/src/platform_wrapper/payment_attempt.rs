use common_utils::id_type;
use hyperswitch_domain_models::{payments::payment_attempt::PaymentAttempt, platform::Processor};

use crate::{core::errors, db::StorageInterface};

#[cfg(feature = "v1")]
pub async fn insert_payment_attempt(
    db: &dyn StorageInterface,
    processor: &Processor,
    payment_attempt: PaymentAttempt,
) -> errors::CustomResult<PaymentAttempt, errors::StorageError> {
    db.insert_payment_attempt(
        payment_attempt,
        processor.get_account().storage_scheme,
        processor.get_key_store(),
    )
    .await
}

#[cfg(feature = "v2")]
pub async fn insert_payment_attempt(
    db: &dyn StorageInterface,
    processor: &Processor,
    payment_attempt: PaymentAttempt,
) -> errors::CustomResult<PaymentAttempt, errors::StorageError> {
    db.insert_payment_attempt(
        processor.get_key_store(),
        payment_attempt,
        processor.get_account().storage_scheme,
    )
    .await
}

#[cfg(feature = "v1")]
pub async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
    db: &dyn StorageInterface,
    processor: &Processor,
    payment_id: &id_type::PaymentId,
    attempt_id: &str,
) -> errors::CustomResult<PaymentAttempt, errors::StorageError> {
    db.find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        payment_id,
        processor.get_account().get_id(),
        attempt_id,
        processor.get_account().storage_scheme,
        processor.get_key_store(),
    )
    .await
}
