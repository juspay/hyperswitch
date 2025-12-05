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
    storage_scheme: common_enums::MerchantStorageScheme,
) -> CustomResult<PaymentIntent, StorageError>
where
    S: PaymentIntentInterface<Error = StorageError> + ?Sized,
{
    store
        .insert_payment_intent(payment_intent, processor.get_key_store(), storage_scheme)
        .await
}
