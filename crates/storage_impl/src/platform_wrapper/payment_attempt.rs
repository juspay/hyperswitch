use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    payments::payment_attempt::{PaymentAttempt, PaymentAttemptInterface},
    platform::Processor,
};

use crate::StorageError;

#[cfg(feature = "v1")]
pub async fn insert_payment_attempt<S>(
    store: &S,
    processor: &Processor,
    payment_attempt: PaymentAttempt,
    storage_scheme: common_enums::MerchantStorageScheme,
) -> CustomResult<PaymentAttempt, StorageError>
where
    S: PaymentAttemptInterface<Error = StorageError> + ?Sized,
{
    store
        .insert_payment_attempt(payment_attempt, storage_scheme, processor.get_key_store())
        .await
}

#[cfg(feature = "v2")]
pub async fn insert_payment_attempt<S>(
    store: &S,
    processor: &Processor,
    payment_attempt: PaymentAttempt,
    storage_scheme: common_enums::MerchantStorageScheme,
) -> CustomResult<PaymentAttempt, StorageError>
where
    S: PaymentAttemptInterface<Error = StorageError> + ?Sized,
{
    store
        .insert_payment_attempt(processor.get_key_store(), payment_attempt, storage_scheme)
        .await
}
