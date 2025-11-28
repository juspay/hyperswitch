use hyperswitch_domain_models::{payments::PaymentIntent, platform::Provider};

use crate::{core::errors, db::StorageInterface};

// Insert a new payment intent using Provider context
pub async fn insert(
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
