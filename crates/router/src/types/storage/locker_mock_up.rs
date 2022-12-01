#[cfg(feature = "diesel")]
use diesel::{Identifiable, Insertable, Queryable};

#[cfg(feature = "diesel")]
use crate::schema::locker_mock_up;

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "diesel", derive(Identifiable, Queryable))]
#[cfg_attr(feature = "diesel", diesel(table_name = locker_mock_up))]
pub struct LockerMockUp {
    pub id: i32,
    pub card_id: String,
    pub external_id: String,
    pub card_fingerprint: String,
    pub card_global_fingerprint: String,
    pub merchant_id: String,
    pub card_number: String,
    pub card_exp_year: String,
    pub card_exp_month: String,
    pub name_on_card: Option<String>,
    pub nickname: Option<String>,
    pub customer_id: Option<String>,
    pub duplicate: Option<bool>,
}

#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = locker_mock_up))]
pub struct LockerMockUpNew {
    pub card_id: String,
    pub external_id: String,
    pub card_fingerprint: String,
    pub card_global_fingerprint: String,
    pub merchant_id: String,
    pub card_number: String,
    pub card_exp_year: String,
    pub card_exp_month: String,
}
