#[cfg(feature = "v2")]
pub mod types {
use crate::ApiModelToDieselModelConvertor;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VaultType {
    /// VGS vault - direct token replacement without modifications
    Vgs,
}

impl ApiModelToDieselModelConvertor<api_models::injector::VaultType> for VaultType {
    fn convert_from(from: api_models::injector::VaultType) -> Self {
        match from {
            api_models::injector::VaultType::Vgs => Self::Vgs,
        }
    }

    fn convert_back(self) -> api_models::injector::VaultType {
        match self {
            Self::Vgs => api_models::injector::VaultType::Vgs,
        }
    }
}

impl From<api_models::injector::VaultType> for VaultType {
    fn from(vault_type: api_models::injector::VaultType) -> Self {
        match vault_type {
            api_models::injector::VaultType::Vgs => Self::Vgs,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl ApiModelToDieselModelConvertor<api_models::injector::HttpMethod> for HttpMethod {
    fn convert_from(from: api_models::injector::HttpMethod) -> Self {
        match from {
            api_models::injector::HttpMethod::Get => Self::Get,
            api_models::injector::HttpMethod::Post => Self::Post,
            api_models::injector::HttpMethod::Put => Self::Put,
            api_models::injector::HttpMethod::Patch => Self::Patch,
            api_models::injector::HttpMethod::Delete => Self::Delete,
            api_models::injector::HttpMethod::Head => Self::Head,
            api_models::injector::HttpMethod::Options => Self::Options,
        }
    }

    fn convert_back(self) -> api_models::injector::HttpMethod {
        match self {
            Self::Get => api_models::injector::HttpMethod::Get,
            Self::Post => api_models::injector::HttpMethod::Post,
            Self::Put => api_models::injector::HttpMethod::Put,
            Self::Patch => api_models::injector::HttpMethod::Patch,
            Self::Delete => api_models::injector::HttpMethod::Delete,
            Self::Head => api_models::injector::HttpMethod::Head,
            Self::Options => api_models::injector::HttpMethod::Options,
        }
    }
}

impl From<api_models::injector::HttpMethod> for HttpMethod {
    fn from(method: api_models::injector::HttpMethod) -> Self {
        match method {
            api_models::injector::HttpMethod::Get => Self::Get,
            api_models::injector::HttpMethod::Post => Self::Post,
            api_models::injector::HttpMethod::Put => Self::Put,
            api_models::injector::HttpMethod::Patch => Self::Patch,
            api_models::injector::HttpMethod::Delete => Self::Delete,
            api_models::injector::HttpMethod::Head => Self::Head,
            api_models::injector::HttpMethod::Options => Self::Options,
        }
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentType {
    ApplicationJson,
    ApplicationXWwwFormUrlencoded,
    ApplicationXml,
    TextXml,
    TextPlain,
}

impl ApiModelToDieselModelConvertor<api_models::injector::ContentType> for ContentType {
    fn convert_from(from: api_models::injector::ContentType) -> Self {
        match from {
            api_models::injector::ContentType::ApplicationJson => Self::ApplicationJson,
            api_models::injector::ContentType::ApplicationXWwwFormUrlencoded => Self::ApplicationXWwwFormUrlencoded,
            api_models::injector::ContentType::ApplicationXml => Self::ApplicationXml,
            api_models::injector::ContentType::TextXml => Self::TextXml,
            api_models::injector::ContentType::TextPlain => Self::TextPlain,
        }
    }

    fn convert_back(self) -> api_models::injector::ContentType {
        match self {
            Self::ApplicationJson => api_models::injector::ContentType::ApplicationJson,
            Self::ApplicationXWwwFormUrlencoded => api_models::injector::ContentType::ApplicationXWwwFormUrlencoded,
            Self::ApplicationXml => api_models::injector::ContentType::ApplicationXml,
            Self::TextXml => api_models::injector::ContentType::TextXml,
            Self::TextPlain => api_models::injector::ContentType::TextPlain,
        }
    }
}

impl From<api_models::injector::ContentType> for ContentType {
    fn from(content_type: api_models::injector::ContentType) -> Self {
        match content_type {
            api_models::injector::ContentType::ApplicationJson => Self::ApplicationJson,
            api_models::injector::ContentType::ApplicationXWwwFormUrlencoded => Self::ApplicationXWwwFormUrlencoded,
            api_models::injector::ContentType::ApplicationXml => Self::ApplicationXml,
            api_models::injector::ContentType::TextXml => Self::TextXml,
            api_models::injector::ContentType::TextPlain => Self::TextPlain,
        }
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcceptType {
    ApplicationJson,
    ApplicationXml,
    TextXml,
    TextPlain,
    Any,
}

impl ApiModelToDieselModelConvertor<api_models::injector::AcceptType> for AcceptType {
    fn convert_from(from: api_models::injector::AcceptType) -> Self {
        match from {
            api_models::injector::AcceptType::ApplicationJson => Self::ApplicationJson,
            api_models::injector::AcceptType::ApplicationXml => Self::ApplicationXml,
            api_models::injector::AcceptType::TextXml => Self::TextXml,
            api_models::injector::AcceptType::TextPlain => Self::TextPlain,
            api_models::injector::AcceptType::Any => Self::Any,
        }
    }

    fn convert_back(self) -> api_models::injector::AcceptType {
        match self {
            Self::ApplicationJson => api_models::injector::AcceptType::ApplicationJson,
            Self::ApplicationXml => api_models::injector::AcceptType::ApplicationXml,
            Self::TextXml => api_models::injector::AcceptType::TextXml,
            Self::TextPlain => api_models::injector::AcceptType::TextPlain,
            Self::Any => api_models::injector::AcceptType::Any,
        }
    }
}

impl From<api_models::injector::AcceptType> for AcceptType {
    fn from(accept_type: api_models::injector::AcceptType) -> Self {
        match accept_type {
            api_models::injector::AcceptType::ApplicationJson => Self::ApplicationJson,
            api_models::injector::AcceptType::ApplicationXml => Self::ApplicationXml,
            api_models::injector::AcceptType::TextXml => Self::TextXml,
            api_models::injector::AcceptType::TextPlain => Self::TextPlain,
            api_models::injector::AcceptType::Any => Self::Any,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SpecificTokenData {
    pub card_number: String,
    pub cvv: String,
    pub exp_month: String,
    pub exp_year: String,
}

impl ApiModelToDieselModelConvertor<api_models::injector::SpecificTokenData> for SpecificTokenData {
    fn convert_from(from: api_models::injector::SpecificTokenData) -> Self {
        Self {
            card_number: from.card_number,
            cvv: from.cvv,
            exp_month: from.exp_month,
            exp_year: from.exp_year,
        }
    }

    fn convert_back(self) -> api_models::injector::SpecificTokenData {
        api_models::injector::SpecificTokenData {
            card_number: self.card_number,
            cvv: self.cvv,
            exp_month: self.exp_month,
            exp_year: self.exp_year,
        }
    }
}

impl From<api_models::injector::SpecificTokenData> for SpecificTokenData {
    fn from(token_data: api_models::injector::SpecificTokenData) -> Self {
        Self {
            card_number: token_data.card_number,
            cvv: token_data.cvv,
            exp_month: token_data.exp_month,
            exp_year: token_data.exp_year,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TokenData {
    pub specific_token_data: SpecificTokenData,
    pub vault_type: VaultType,
}

impl ApiModelToDieselModelConvertor<api_models::injector::TokenData> for TokenData {
    fn convert_from(from: api_models::injector::TokenData) -> Self {
        Self {
            specific_token_data: SpecificTokenData::convert_from(from.specific_token_data),
            vault_type: VaultType::convert_from(from.vault_type),
        }
    }

    fn convert_back(self) -> api_models::injector::TokenData {
        api_models::injector::TokenData {
            specific_token_data: self.specific_token_data.convert_back(),
            vault_type: self.vault_type.convert_back(),
        }
    }
}

impl From<api_models::injector::TokenData> for TokenData {
    fn from(token_data: api_models::injector::TokenData) -> Self {
        Self {
            specific_token_data: token_data.specific_token_data.into(),
            vault_type: token_data.vault_type.into(),
        }
    }
}


#[derive(Clone, Debug)]
pub struct ConnectorPayload {
    pub template: String,
}

impl ApiModelToDieselModelConvertor<api_models::injector::ConnectorPayload> for ConnectorPayload {
    fn convert_from(from: api_models::injector::ConnectorPayload) -> Self {
        Self {
            template: from.template,
        }
    }

    fn convert_back(self) -> api_models::injector::ConnectorPayload {
        api_models::injector::ConnectorPayload {
            template: self.template,
        }
    }
}

impl From<api_models::injector::ConnectorPayload> for ConnectorPayload {
    fn from(payload: api_models::injector::ConnectorPayload) -> Self {
        Self {
            template: payload.template,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionConfig {
    pub base_url: String,
    pub endpoint_path: String,
    pub http_method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub proxy_url: Option<String>,
    
    // TLS/SSL Certificate Configuration
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
    pub ca_cert: Option<String>,
    pub insecure: Option<bool>,
    pub cert_password: Option<String>,
    pub cert_format: Option<String>,
}

impl ApiModelToDieselModelConvertor<api_models::injector::ConnectionConfig> for ConnectionConfig {
    fn convert_from(from: api_models::injector::ConnectionConfig) -> Self {
        Self {
            base_url: from.base_url,
            endpoint_path: from.endpoint_path,
            http_method: HttpMethod::convert_from(from.http_method),
            headers: from.headers,
            proxy_url: from.proxy_url,
            client_cert: from.client_cert,
            client_key: from.client_key,
            ca_cert: from.ca_cert,
            insecure: from.insecure,
            cert_password: from.cert_password,
            cert_format: from.cert_format,
        }
    }

    fn convert_back(self) -> api_models::injector::ConnectionConfig {
        api_models::injector::ConnectionConfig {
            base_url: self.base_url,
            endpoint_path: self.endpoint_path,
            http_method: self.http_method.convert_back(),
            headers: self.headers,
            proxy_url: self.proxy_url,
            client_cert: self.client_cert,
            client_key: self.client_key,
            ca_cert: self.ca_cert,
            insecure: self.insecure,
            cert_password: self.cert_password,
            cert_format: self.cert_format,
        }
    }
}

impl From<api_models::injector::ConnectionConfig> for ConnectionConfig {
    fn from(config: api_models::injector::ConnectionConfig) -> Self {
        Self {
            base_url: config.base_url,
            endpoint_path: config.endpoint_path,
            http_method: config.http_method.into(),
            headers: config.headers,
            proxy_url: config.proxy_url,
            client_cert: config.client_cert,
            client_key: config.client_key,
            ca_cert: config.ca_cert,
            insecure: config.insecure,
            cert_password: config.cert_password,
            cert_format: config.cert_format,
        }
    }
}


#[derive(Clone, Debug)]
pub struct InjectorRequest {
    pub token_data: TokenData,
    pub connector_payload: ConnectorPayload,
    pub connection_config: ConnectionConfig,
}

impl ApiModelToDieselModelConvertor<api_models::injector::InjectorRequest> for InjectorRequest {
    fn convert_from(from: api_models::injector::InjectorRequest) -> Self {
        Self {
            token_data: TokenData::convert_from(from.token_data),
            connector_payload: ConnectorPayload::convert_from(from.connector_payload),
            connection_config: ConnectionConfig::convert_from(from.connection_config),
        }
    }

    fn convert_back(self) -> api_models::injector::InjectorRequest {
        api_models::injector::InjectorRequest {
            token_data: self.token_data.convert_back(),
            connector_payload: self.connector_payload.convert_back(),
            connection_config: self.connection_config.convert_back(),
        }
    }
}

impl From<api_models::injector::InjectorRequest> for InjectorRequest {
    fn from(request: api_models::injector::InjectorRequest) -> Self {
        Self {
            token_data: request.token_data.into(),
            connector_payload: request.connector_payload.into(),
            connection_config: request.connection_config.into(),
        }
    }
}

pub type InjectorResponse = serde_json::Value;

}

// Re-export all types when v2 feature is enabled
#[cfg(feature = "v2")]
pub use types::*;