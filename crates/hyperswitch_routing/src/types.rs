use hyperswitch_domain_models::payouts;

pub const CONTENT_TYPE: &str = "Content-Type";
#[derive(Clone)]
pub struct PayoutData {
    pub payouts: payouts::payouts::Payouts,
    pub billing_country: Option<common_enums::enums::Country>,
    pub payment_method: Option<common_enums::enums::PaymentMethod>,
    pub payout_attempt: payouts::payout_attempt::PayoutAttempt,
    pub payout_method_type: Option<common_enums::enums::PaymentMethodType>,
    pub profile_id: common_utils::id_type::ProfileId,
}