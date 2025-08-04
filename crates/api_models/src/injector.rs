#[cfg(feature = "v2")]
pub mod types {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
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
        /// VGS vault - direct token replacement without modifications
        Vgs
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
        Get,
        Post,
        Put,
        Patch,
        Delete,
        Head,
        Options,
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
    pub struct SpecificTokenData {
        pub card_number: String,
        pub cvv: String,
        pub exp_month: String,
        pub exp_year: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
    pub struct TokenData {
        pub specific_token_data: SpecificTokenData,
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
        pub headers: HashMap<String, String>,
        /// Optional proxy URL for environments without direct internet access
        /// Equivalent to curl's -x parameter (e.g., "http://proxy.company.com:8080")
        pub proxy_url: Option<String>,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
    pub struct InjectorRequest {
        pub token_data: TokenData,
        pub connector_payload: ConnectorPayload,
        pub connection_config: ConnectionConfig,
    }

    // Direct serde_json::Value response for connector-agnostic handling
    pub type InjectorResponse = serde_json::Value;
}

// Re-export all types when v2 feature is enabled
#[cfg(feature = "v2")]
pub use types::*;