use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{enums as api_enums, payments};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MandateId {
    pub mandate_id: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MandateRevokedResponse {
    pub mandate_id: String,
    pub status: api_enums::MandateStatus,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MandateResponse {
    pub mandate_id: String,
    pub status: api_enums::MandateStatus,
    pub payment_method_id: String,
    pub payment_method: String,
    pub card: Option<MandateCardDetails>,
    pub customer_acceptance: Option<payments::CustomerAcceptance>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MandateCardDetails {
    pub last4_digits: Option<String>,
    pub card_exp_month: Option<Secret<String>>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_token: Option<Secret<String>>,
    pub scheme: Option<String>,
    pub issuer_country: Option<String>,
    pub card_fingerprint: Option<Secret<String>>,
}
