use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{core::errors, types};

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidLinkTokenRequest {
    client_name: String,
    country_codes: Vec<String>,
    language: String,
    products: Vec<String>,
    user: User,
}

#[derive(Debug, Serialize, Eq, PartialEq)]

pub struct User {
    pub client_user_id: String,
}

impl TryFrom<&types::LinkTokenRouterData> for PlaidLinkTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::LinkTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            client_name: item.request.client_name.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "client_name",
                },
            )?,
            country_codes: item.request.country_codes.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "country_codes",
                },
            )?,
            language: item.request.language.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "language",
                },
            )?,
            products: vec!["auth".to_string()],
            user: User {
                client_user_id: item.request.user_info.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "user.client_user_id",
                    },
                )?,
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidLinkTokenResponse {
    expiration: String,
    request_id: String,
    link_token: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PlaidLinkTokenResponse, T, types::LinkTokenResponse>>
    for types::PaymentAuthRouterData<F, T, types::LinkTokenResponse>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PlaidLinkTokenResponse, T, types::LinkTokenResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::LinkTokenResponse {
                expiration: Some(item.response.expiration),
                request_id: Some(item.response.request_id),
                link_token: Some(item.response.link_token),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidExchangeTokenRequest {
    public_token: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]

pub struct PlaidExchangeTokenResponse {
    pub access_token: String,
    pub request_id: String,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, PlaidExchangeTokenResponse, T, types::ExchangeTokenResponse>,
    > for types::PaymentAuthRouterData<F, T, types::ExchangeTokenResponse>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PlaidExchangeTokenResponse,
            T,
            types::ExchangeTokenResponse,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::ExchangeTokenResponse {
                access_token: Some(item.response.access_token),
                request_id: Some(item.response.request_id),
            }),
            ..item.data
        })
    }
}

impl TryFrom<&types::ExchangeTokenRouterData> for PlaidExchangeTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ExchangeTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            public_token: item.request.public_token.clone(),
        })
    }
}

pub struct PlaidAuthType {
    pub client_id: Secret<String>,
    pub secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for PlaidAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { client_id, secret } => Ok(Self {
                client_id: client_id.to_owned(),
                secret: secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidErrorResponse {
    pub display_message: Option<String>,
    pub error_code: Option<String>,
    pub error_message: String,
    pub error_type: Option<String>,
}
