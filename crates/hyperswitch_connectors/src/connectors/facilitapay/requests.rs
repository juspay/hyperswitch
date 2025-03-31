use common_utils::types::StringMinorUnit;
use serde::Serialize;
use masking::Secret;

//TODO: Fill the struct with respective fields
pub struct FacilitapayRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

#[derive(Debug, Serialize)]
pub struct FacilitapaySignInRequest {
    user: FacilitapayUserCredentials,
}

#[derive(Debug, Serialize)]
pub struct FacilitapayUserCredentials {
    username: Secret<String>, // email_id
    password: Secret<String>,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct FacilitapayPaymentsRequest {
    pub amount: StringMinorUnit,
    pub card: FacilitapayCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct FacilitapayCard {
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
pub struct FacilitapayRefundRequest {
    pub amount: StringMinorUnit,
}
