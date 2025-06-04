use serde::{self, Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CardTestingGuardData {
    pub is_card_ip_blocking_enabled: bool,
    pub card_ip_blocking_cache_key: String,
    pub is_guest_user_card_blocking_enabled: bool,
    pub guest_user_card_blocking_cache_key: String,
    pub is_customer_id_blocking_enabled: bool,
    pub customer_id_blocking_cache_key: String,
    pub card_testing_guard_expiry: i32,
}
