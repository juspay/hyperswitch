use crate::types::{
    BankAccountCredentialsRequest, BankAccountCredentialsResponse, ExchangeTokenRequest,
    ExchangeTokenResponse, LinkTokenRequest, LinkTokenResponse,
};

pub trait AuthService:
    super::ConnectorCommon
    + AuthServiceLinkToken
    + AuthServiceExchangeToken
    + AuthServiceBankAccountCredentials
{
}

#[derive(Debug, Clone)]
pub struct LinkToken;

pub trait AuthServiceLinkToken:
    super::ConnectorIntegration<LinkToken, LinkTokenRequest, LinkTokenResponse>
{
}

#[derive(Debug, Clone)]
pub struct ExchangeToken;

pub trait AuthServiceExchangeToken:
    super::ConnectorIntegration<ExchangeToken, ExchangeTokenRequest, ExchangeTokenResponse>
{
}

#[derive(Debug, Clone)]
pub struct BankAccountCredentials;

pub trait AuthServiceBankAccountCredentials:
    super::ConnectorIntegration<
    BankAccountCredentials,
    BankAccountCredentialsRequest,
    BankAccountCredentialsResponse,
>
{
}
