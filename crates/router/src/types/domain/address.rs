use common_utils::pii;
use masking::Secret;
use storage_models::enums;
use time::PrimitiveDateTime;

use super::behaviour;

#[derive(Clone, Debug)]
pub struct Address {
    // #[serde(skip_serializing)]
    pub id: i32,
    // #[serde(skip_serializing)]
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<enums::CountryCode>,
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub line3: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub phone_number: Option<Secret<String>>,
    pub country_code: Option<String>,
    // #[serde(skip_serializing)]
    // #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    // #[serde(skip_serializing)]
    // #[serde(with = "custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub customer_id: String,
    pub merchant_id: String,
}
// TODO: Do we need this to implement serde?
