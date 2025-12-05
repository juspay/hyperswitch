use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    payments::{payment_intent::PaymentIntentInterface, PaymentIntent},
    platform::Processor,
};

use crate::StorageError;

pub async fn insert_payment_intent<S>(
    store: &S,
    processor: &Processor,
    payment_intent: PaymentIntent,
) -> CustomResult<PaymentIntent, StorageError>
where
    S: PaymentIntentInterface<Error = StorageError> + ?Sized,
{
    store
        .insert_payment_intent(payment_intent, processor.get_key_store(), processor.get_account().storage_scheme)
        .await
}
