use std::fmt::Debug;

#[derive(serde::Serialize, Debug)]
pub struct CardInfoResponse {
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<String>,
    pub card_type: Option<String>,
    pub card_sub_type: Option<String>,
    pub card_issuing_country: Option<String>,
}
