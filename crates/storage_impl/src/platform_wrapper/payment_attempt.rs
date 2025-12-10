use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    payments::payment_attempt::{PaymentAttempt, PaymentAttemptInterface, PaymentAttemptUpdate},
    platform::Processor,
};

use crate::StorageError;

#[cfg(feature = "v1")]
pub async fn insert_payment_attempt<S>(
    store: &S,
    processor: &Processor,
    payment_attempt: PaymentAttempt,
) -> CustomResult<PaymentAttempt, StorageError>
where
    S: PaymentAttemptInterface<Error = StorageError> + ?Sized,
{
    store
        .insert_payment_attempt(
            payment_attempt,
            processor.get_account().storage_scheme,
            processor.get_key_store(),
        )
        .await
}

#[cfg(feature = "v2")]
pub async fn insert_payment_attempt<S>(
    store: &S,
    processor: &Processor,
    payment_attempt: PaymentAttempt,
) -> CustomResult<PaymentAttempt, StorageError>
where
    S: PaymentAttemptInterface<Error = StorageError> + ?Sized,
{
    store
        .insert_payment_attempt(
            processor.get_key_store(),
            payment_attempt,
            processor.get_account().storage_scheme,
        )
        .await
}

#[cfg(feature = "v2")]
pub async fn update_payment_attempt<S>(
    store: &S,
    processor: &Processor,
    payment_attempt: PaymentAttempt,
    payment_attempt_update: PaymentAttemptUpdate,
) -> CustomResult<PaymentAttempt, StorageError>
where
    S: PaymentAttemptInterface<Error = StorageError> + ?Sized,
{
    store
        .update_payment_attempt(
            processor.get_key_store(),
            payment_attempt,
            payment_attempt_update,
            processor.get_account().storage_scheme,
        )
        .await
}

#[cfg(feature = "v1")]
pub async fn update_payment_attempt_with_attempt_id<S>(
    store: &S,
    processor: &Processor,
    payment_attempt: PaymentAttempt,
    payment_attempt_update: PaymentAttemptUpdate,
) -> CustomResult<PaymentAttempt, StorageError>
where
    S: PaymentAttemptInterface<Error = StorageError> + ?Sized,
{
    store
        .update_payment_attempt_with_attempt_id(
            payment_attempt,
            payment_attempt_update,
            processor.get_account().storage_scheme,
            processor.get_key_store(),
        )
        .await
}
