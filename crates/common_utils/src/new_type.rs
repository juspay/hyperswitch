//! Contains new types with restrictions
use crate::consts::MAX_ALLOWED_MERCHANT_NAME_LENGTH;

#[nutype::nutype(
    derive(Clone, Serialize, Deserialize, Debug),
    validate(len_char_min = 1, len_char_max = MAX_ALLOWED_MERCHANT_NAME_LENGTH)
)]
pub struct MerchantName(String);

impl masking::SerializableSecret for MerchantName {}
