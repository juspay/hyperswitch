use common_utils::id_type;
use hyperswitch_domain_models::{payments::PaymentIntent, platform::Processor};

use crate::{core::errors, db::StorageInterface};

pub async fn insert_payment_intent(
    db: &dyn StorageInterface,
    processor: &Processor,
    payment_intent: PaymentIntent,
) -> errors::CustomResult<PaymentIntent, errors::StorageError> {
    db.insert_payment_intent(
        payment_intent,
        processor.get_key_store(),
        processor.get_account().storage_scheme,
    )
    .await
}

#[cfg(feature = "v1")]
pub async fn find_payment_intent_by_payment_id_merchant_id(
    db: &dyn StorageInterface,
    processor: &Processor,
    payment_id: &id_type::PaymentId,
) -> errors::CustomResult<PaymentIntent, errors::StorageError> {
    db.find_payment_intent_by_payment_id_merchant_id(
        payment_id,
        processor.get_account().get_id(),
        processor.get_key_store(),
        processor.get_account().storage_scheme,
    )
    .await
}
