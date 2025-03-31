use serde::{Deserialize, Serialize};
use masking::Secret;


// Response body for POST /sign_in
#[derive(Debug, Deserialize)]
pub struct FacilitapaySignInResponse {
    username: String,
    name: String,
    jwt: Secret<String>, // The JWT token
}

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FacilitapayPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FacilitapayPaymentsResponse {
    pub status: FacilitapayPaymentStatus,
    pub id: String,
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub status: RefundStatus,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacilitapayErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
