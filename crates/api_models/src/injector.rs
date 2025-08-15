#[cfg(feature = "v2")]
pub mod types {
    use std::collections::HashMap;

    use common_utils::pii::SecretSerdeValue;
    use masking::Secret;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        PartialEq,
        Deserialize,
        Serialize,
        strum::Display,
        strum::EnumString,
        ToSchema,
    )]
    #[serde(rename_all = "UPPERCASE")]
    #[strum(serialize_all = "UPPERCASE")]
    pub enum VaultType {
        VGS,
    }

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Deserialize,
    Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        PartialEq,
        Deserialize,
        Serialize,
        strum::Display,
        strum::EnumString,
        ToSchema,
    )]
    #[serde(rename_all = "kebab-case")]
    #[strum(serialize_all = "kebab-case")]
    pub enum ContentType {
        #[serde(rename = "application/json")]
        #[strum(serialize = "application/json")]
        ApplicationJson,
        #[serde(rename = "application/x-www-form-urlencoded")]
        #[strum(serialize = "application/x-www-form-urlencoded")]
        ApplicationXWwwFormUrlencoded,
        #[serde(rename = "application/xml")]
        #[strum(serialize = "application/xml")]
        ApplicationXml,
        #[serde(rename = "text/xml")]
        #[strum(serialize = "text/xml")]
        TextXml,
        #[serde(rename = "text/plain")]
        #[strum(serialize = "text/plain")]
        TextPlain,
    }

    #[derive(
        Clone,
        Copy,
        Debug,
        Eq,
        PartialEq,
        Deserialize,
        Serialize,
        strum::Display,
        strum::EnumString,
        ToSchema,
    )]
    #[serde(rename_all = "kebab-case")]
    #[strum(serialize_all = "kebab-case")]
    pub enum AcceptType {
        #[serde(rename = "application/json")]
        #[strum(serialize = "application/json")]
        ApplicationJson,
        #[serde(rename = "application/xml")]
        #[strum(serialize = "application/xml")]
        ApplicationXml,
        #[serde(rename = "text/xml")]
        #[strum(serialize = "text/xml")]
        TextXml,
        #[serde(rename = "text/plain")]
        #[strum(serialize = "text/plain")]
        TextPlain,
        #[serde(rename = "*/*")]
        #[strum(serialize = "*/*")]
        Any,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
    pub struct TokenData {
        pub specific_token_data: SecretSerdeValue,
        pub vault_type: VaultType,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
    pub struct ConnectorPayload {
        pub template: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
    pub struct ConnectionConfig {
        pub base_url: String,
        pub endpoint_path: String,
        pub http_method: HttpMethod,
        pub headers: HashMap<String, Secret<String>>,
        pub proxy_url: Option<String>,
        pub client_cert: Option<Secret<String>>,
        pub client_key: Option<Secret<String>>,
        pub ca_cert: Option<Secret<String>>,
        pub insecure: Option<bool>,
        pub cert_password: Option<Secret<String>>,
        pub cert_format: Option<String>,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
    pub struct InjectorRequest {
        pub token_data: TokenData,
        pub connector_payload: ConnectorPayload,
        pub connection_config: ConnectionConfig,
    }

    pub type InjectorResponse = serde_json::Value;
}

#[cfg(feature = "v2")]
pub use types::*;
