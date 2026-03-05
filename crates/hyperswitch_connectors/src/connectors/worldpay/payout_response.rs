use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayPayoutResponse {
    pub outcome: PayoutOutcome,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PayoutOutcome {
    RequestReceived,
    Refused,
    Error,
    QueryRequired,
}
