//! Contains new types with restrictions

#[nutype::nutype(
    derive(Clone, Serialize, Deserialize, Debug),
    validate(len_char_min = 1, len_char_max = 64)
)]
pub struct MerchantName(String);

impl masking::SerializableSecret for MerchantName {}
