use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;

/// Saferpay (SIX Payment Services) authentication.
///
/// Saferpay is routed exclusively through the Unified Connector Service (UCS);
/// this type exists so that the merchant connector account auth payload can be
/// validated on the Hyperswitch side before it is forwarded to UCS.
///
/// * `api_key`    -> API username (HTTP Basic username)
/// * `key1`       -> API password (HTTP Basic password)
/// * `api_secret` -> CustomerId  (`RequestHeader.CustomerId`)
/// * `key2`       -> TerminalId  (request body `TerminalId`)
#[allow(dead_code)] // `customer_id` and `terminal_id` are consumed by UCS, not by this stub
pub struct SaferpayAuthType {
    pub(super) api_username: Secret<String>,
    pub(super) api_password: Secret<String>,
    pub(super) customer_id: Secret<String>,
    pub(super) terminal_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SaferpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Ok(Self {
                api_username: api_key.to_owned(),
                api_password: key1.to_owned(),
                customer_id: api_secret.to_owned(),
                terminal_id: key2.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
