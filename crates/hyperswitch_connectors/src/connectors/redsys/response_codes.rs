use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DsResponse {
    Authorized,
    ExpiredCard,
    PaymentDeclined,
    InsufficientFunds,
    FraudDetected,
    Unknown(String),
}

impl<'de> Deserialize<'de> for DsResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if s.starts_with("00") {
            Ok(DsResponse::Authorized)
        } else {
            match s.as_str() {
                "0" => Ok(DsResponse::ExpiredCard),
                "1" => Ok(DsResponse::PaymentDeclined),
                "2" => Ok(DsResponse::InsufficientFunds),
                "3" => Ok(DsResponse::FraudDetected),
                _ => Ok(DsResponse::Unknown(s)),
            }
        }
    }
}