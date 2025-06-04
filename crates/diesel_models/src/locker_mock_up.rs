use diesel::{Identifiable, Insertable, Queryable, Selectable};

use crate::schema::locker_mock_up;

#[derive(Clone, Debug, Eq, Identifiable, Queryable, Selectable, PartialEq)]
#[diesel(table_name = locker_mock_up, primary_key(card_id), check_for_backend(diesel::pg::Pg))]
pub struct LockerMockUp {
    pub card_id: String,
    pub external_id: String,
    pub card_fingerprint: String,
    pub card_global_fingerprint: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub card_number: String,
    pub card_exp_year: String,
    pub card_exp_month: String,
    pub name_on_card: Option<String>,
    pub nickname: Option<String>,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub duplicate: Option<bool>,
    pub card_cvc: Option<String>,
    pub payment_method_id: Option<String>,
    pub enc_card_data: Option<String>,
}

#[derive(Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = locker_mock_up)]
pub struct LockerMockUpNew {
    pub card_id: String,
    pub external_id: String,
    pub card_fingerprint: String,
    pub card_global_fingerprint: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub card_number: String,
    pub card_exp_year: String,
    pub card_exp_month: String,
    pub name_on_card: Option<String>,
    pub card_cvc: Option<String>,
    pub payment_method_id: Option<String>,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub nickname: Option<String>,
    pub enc_card_data: Option<String>,
}
