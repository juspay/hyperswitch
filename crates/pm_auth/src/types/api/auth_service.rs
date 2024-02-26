use crate::types::{
    BankAccountCredentialsRequest, BankAccountCredentialsResponse, ExchangeTokenRequest,
    ExchangeTokenResponse, LinkTokenRequest, LinkTokenResponse, RecipientCreateRequest,
    RecipientCreateResponse,
};

pub trait AuthService:
    super::ConnectorCommon
    + AuthServiceLinkToken
    + AuthServiceExchangeToken
    + AuthServiceBankAccountCredentials
{
}

pub trait PaymentInitiation: super::ConnectorCommon + PaymentInitiationRecipientCreate {}

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

#[derive(Debug, Clone)]
pub struct RecipientCreate;

pub trait PaymentInitiationRecipientCreate:
    super::ConnectorIntegration<RecipientCreate, RecipientCreateRequest, RecipientCreateResponse>
{
}
