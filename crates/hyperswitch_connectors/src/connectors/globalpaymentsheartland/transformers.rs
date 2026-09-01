use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;

/// Global Payments Heartland (Portico gateway) authentication.
///
/// Heartland is routed exclusively through the Unified Connector Service (UCS);
/// this type exists so that the merchant connector account auth payload can be
/// validated on the Hyperswitch side before it is forwarded to UCS.
///
/// * `api_key` -> Portico `SecretAPIKey` (`skapi_cert_...` / `skapi_prod_...`)
///
/// The merchant's *public* key and MID are deliberately not part of this type.
/// The public key is browser-side tokenisation only, and the MID is already
/// bound to the secret key server-side — Portico echoes the resolved
/// `MerchNbr` / `SiteId` / `DeviceId` back on every response.
#[allow(dead_code)] // `secret_api_key` is consumed by UCS, not by this stub
pub struct GlobalpaymentsheartlandAuthType {
    pub(super) secret_api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for GlobalpaymentsheartlandAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                secret_api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
