use std::{borrow::Cow, string::ToString};

use actix_web::http::header::HeaderMap;
use common_utils::{
    crypto::VerifySignature,
    id_type::{ApiKeyId, MerchantId},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse;

use crate::core::errors::RouterResult;

const HEADER_AUTH_TYPE: &str = "x-auth-type";
const HEADER_MERCHANT_ID: &str = "x-merchant-id";
const HEADER_KEY_ID: &str = "x-key-id";
const HEADER_CHECKSUM: &str = "x-checksum";

#[derive(Debug)]
pub struct ExtractedPayload {
    pub payload_type: PayloadType,
    pub merchant_id: Option<MerchantId>,
    pub key_id: Option<ApiKeyId>,
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
    pub fn from_headers(headers: &HeaderMap) -> RouterResult<Self> {
        let merchant_id = headers
            .get(HEADER_MERCHANT_ID)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| ApiErrorResponse::InvalidRequestData {
                message: format!("`{HEADER_MERCHANT_ID}` header is invalid or not present"),
            })
            .map_err(error_stack::Report::from)
            .and_then(|merchant_id| {
                MerchantId::try_from(Cow::from(merchant_id.to_string())).change_context(
                    ApiErrorResponse::InvalidRequestData {
                        message: format!(
                            "`{HEADER_MERCHANT_ID}` header is invalid or not present",

                        ),
                    },
                )
            })?;

        let auth_type: PayloadType = headers
            .get(HEADER_AUTH_TYPE)
            .and_then(|inner| inner.to_str().ok())
            .ok_or_else(|| ApiErrorResponse::InvalidRequestData {
                message: format!("`{HEADER_AUTH_TYPE}` header not present"),
            })?
            .parse::<PayloadType>()
            .change_context(ApiErrorResponse::InvalidRequestData {
                message: format!("`{HEADER_AUTH_TYPE}` header not present"),
            })?;

        let key_id = headers
            .get(HEADER_KEY_ID)
            .and_then(|value| value.to_str().ok())
            .map(|key_id| ApiKeyId::try_from(Cow::from(key_id.to_string())))
            .transpose()
            .change_context(ApiErrorResponse::InvalidRequestData {
                message: format!("`{HEADER_KEY_ID}` header is invalid or not present"),
            })?;

        Ok(Self {
            payload_type: auth_type,
            merchant_id: Some(merchant_id),
            key_id,
        })
    }

    pub fn verify_checksum(
        &self,
        headers: &HeaderMap,
        algo: impl VerifySignature,
        secret: &[u8],
    ) -> bool {
        let output = || {
            let checksum = headers.get(HEADER_CHECKSUM)?.to_str().ok()?;
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
                .map(|inner| append_api_key(inner.get_string_repr(), &self.key_id)),
        )
    }
}

#[inline]
fn append_option(prefix: &str, data: &Option<String>) -> String {
    match data {
        Some(inner) => format!("{prefix}:{inner}"),
        None => prefix.to_string(),
    }
}

#[inline]
fn append_api_key(prefix: &str, data: &Option<ApiKeyId>) -> String {
    match data {
        Some(inner) => format!("{}:{}", prefix, inner.get_string_repr()),
        None => prefix.to_string(),
    }
}
