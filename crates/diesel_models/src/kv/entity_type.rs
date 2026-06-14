/// Associates a database type name with an entity type.
pub(crate) trait EntityType {
    const ENTITY_TYPE: &'static str;
}

macro_rules! entity_type {
    ($($entity_name:literal => { $($type:path),* $(,)? })*) => {
        $(
            $(
                impl EntityType for $type {
                    const ENTITY_TYPE: &'static str = $entity_name;
                }
            )*
        )*
    };
}
entity_type! {
    "payment_intent" => {
        crate::payment_intent::PaymentIntentNew,
        crate::payment_intent::PaymentIntentUpdateInternal,
    }
    "payment_attempt" => {
        crate::payment_attempt::PaymentAttemptNew,
        crate::payment_attempt::PaymentAttemptUpdateInternal,
    }
    "customer" => {
        crate::customers::CustomerNew,
        crate::customers::CustomerUpdateInternal,
    }
    "refund" => {
        crate::refund::RefundNew,
        crate::refund::RefundUpdateInternal,
    }
    "mandate" => {
        crate::mandate::MandateNew,
        crate::mandate::MandateUpdateInternal,
    }
    "address" => {
        crate::address::AddressNew,
        crate::address::AddressUpdateInternal,
    }
    "payout_attempt" => {
        crate::payout_attempt::PayoutAttemptNew,
        crate::payout_attempt::PayoutAttemptUpdateInternal,
    }
    "payout" => {
        crate::payouts::PayoutsNew,
        crate::payouts::PayoutsUpdateInternal,
    }
    "payment_method" => {
        crate::payment_method::PaymentMethodNew,
        crate::payment_method::PaymentMethodUpdateInternal,
    }
    "reverse_lookup" => {
        crate::reverse_lookup::ReverseLookupNew,
    }
    "capture" => {
        crate::capture::CaptureNew,
        crate::capture::CaptureUpdateInternal,
    }
}
