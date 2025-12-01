use common_utils::id_type;
use hyperswitch_domain_models::{payments::PaymentIntent, platform::Provider};

use crate::{core::errors, db::StorageInterface};

pub async fn insert_payment_intent(
    db: &dyn StorageInterface,
    provider: &Provider,
    payment_intent: PaymentIntent,
) -> errors::CustomResult<PaymentIntent, errors::StorageError> {
    db.insert_payment_intent(
        payment_intent,
        provider.get_key_store(),
        provider.get_account().storage_scheme,
    )
    .await
}

#[cfg(feature = "v1")]
pub async fn find_payment_intent_by_payment_id_merchant_id(
    db: &dyn StorageInterface,
    provider: &Provider,
    payment_id: &id_type::PaymentId,
) -> errors::CustomResult<PaymentIntent, errors::StorageError> {
    db.find_payment_intent_by_payment_id_merchant_id(
        payment_id,
        provider.get_account().get_id(),
        provider.get_key_store(),
        provider.get_account().storage_scheme,
    )
    .await
}
