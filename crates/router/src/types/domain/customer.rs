use common_utils::pii;
use masking::Secret;
use time::PrimitiveDateTime;

use super::behaviour;

#[derive(Clone, Debug)]
pub struct DCustomer {
    pub id: i32,
    pub customer_id: String,
    pub merchant_id: String,
    pub name: Option<String>,
    pub email: Option<Secret<String, pii::Email>>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub description: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
}
