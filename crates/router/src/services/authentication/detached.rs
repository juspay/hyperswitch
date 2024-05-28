use std::string::ToString;

use actix_web::http::header::HeaderMap;
use common_utils::crypto::VerifySignature;

#[derive(Debug)]
pub struct ExtractedPayload {
    pub payload_type: PayloadType,
    pub merchant_id: Option<String>,
    pub key_id: Option<String>,
}

#[derive(strum::EnumString, strum::Display, PartialEq, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum PayloadType {
    ApiKey,
    PublishableKey,
}

pub trait GetAuthType {
    fn get_auth_type(&self) -> PayloadType;
}

impl ExtractedPayload {
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        let merchant_id = headers.get("x-merchant-id")?.to_str().ok()?;
        let auth_type: PayloadType = headers.get("x-auth-type")?.to_str().ok()?.parse().ok()?;

        Some(Self {
            payload_type: auth_type,
            merchant_id: Some(merchant_id.to_string()),
            key_id: headers
                .get("x-key-id")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string()),
        })
    }

    pub fn verify_checksum(
        &self,
        headers: &HeaderMap,
        algo: impl VerifySignature,
        secret: &[u8],
    ) -> bool {
        let output = || {
            let checksum = headers.get("x-checksum")?.to_str().ok()?;
            let payload = self.generate_payload();

            algo.verify_signature(secret, &hex::decode(checksum).ok()?, payload.as_bytes())
                .ok()
        };

        output().unwrap_or(false)
    }

    // The payload should be `:` separated strings of all the fields
    fn generate_payload(&self) -> String {
        append_option(
            &self.payload_type.to_string(),
            &self
                .merchant_id
                .as_ref()
                .map(|inner| append_option(inner, &self.key_id)),
        )
    }
}

fn append_option(prefix: &str, data: &Option<String>) -> String {
    match data {
        Some(inner) => format!("{}:{}", prefix, inner),
        None => prefix.to_string(),
    }
}
