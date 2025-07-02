use serde::{Deserialize, Serialize};
use common_utils::types::StringMinorUnit;
use masking::Secret;

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PayloadPaymentsRequest {
    pub amount: StringMinorUnit,
    pub card: PayloadCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayloadCard {
    pub number: cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvc: Secret<String>,
    pub complete: bool,
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct PayloadRefundRequest {
    pub amount: StringMinorUnit,
}
