use masking::Secret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    connector::utils::PaymentsAuthorizeRequestData,
    core::errors,
    services,
    types::{self, api, storage::enums},
};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct IatapayAuthUpdateRequest {
    grant_type: String,
    scope: String,
}
impl TryFrom<&types::RefreshTokenRouterData> for IatapayAuthUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            grant_type: "client_credentials".to_string(),
            scope: "payment".to_string(),
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct IatapayAuthUpdateResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
    pub jti: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, IatapayAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, IatapayAuthUpdateResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RedirectUrls {
    success_url: String,
    failure_url: String,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IatapayPaymentsRequest {
    merchant_id: String,
    amount: i64,
    currency: String,
    country: String,
    locale: String,
    redirect_urls: RedirectUrls,
    notification_url: String,
    departure_date: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct IatapayCard {
    name: Secret<String>,
    number: Secret<String, common_utils::pii::CardNumber>,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for IatapayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        fn get_redirect_url(item: &types::PaymentsAuthorizeRouterData) -> RedirectUrls {
            RedirectUrls {
                success_url: "https://hyperswitch.io/".to_string(),
                failure_url: "https://www.google.com/".to_string(),
            }
        }
        let payload = Self {
            merchant_id: IatapayAuthType::try_from(&item.connector_auth_type)?.merchant_id,
            amount: item.request.amount,
            currency: item.request.currency.to_string(),
            country: "DE".to_string(),
            locale: "en-GB".to_string(),
            redirect_urls: get_redirect_url(item),
            notification_url: "https://enymh9hvh5dxd.x.pipedream.net/".to_string(),
            departure_date: "2023-12-24".to_string(),
        };
        println!("## payload => {:?}", payload);
        Ok(payload)
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct IatapayAuthType {
    pub(super) client_id: String,
    pub(super) merchant_id: String,
    pub(super) client_secret: String,
}

impl TryFrom<&types::ConnectorAuthType> for IatapayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::SignatureKey { api_key, key1 , api_secret} => Ok(Self {
                client_id: api_key.to_string(),
                merchant_id: key1.to_string(),
                client_secret: api_secret.to_string()
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum IatapayPaymentStatus {
    #[default]
    Created,
    Initiated,
    Authorized,
    Settled,
    Tobeinvestigated,
    Blocked,
    Cleared,
    Failed,
    #[serde(rename = "UNEXPECTED SETTLED")]
    UnexpectedSettled,
}

impl From<IatapayPaymentStatus> for enums::AttemptStatus {
    fn from(item: IatapayPaymentStatus) -> Self {
        match item {
            IatapayPaymentStatus::Authorized | IatapayPaymentStatus::Settled => Self::Charged,
            IatapayPaymentStatus::Failed | IatapayPaymentStatus::UnexpectedSettled => Self::Failure,
            IatapayPaymentStatus::Created => Self::AuthenticationPending,
            IatapayPaymentStatus::Initiated => Self::Pending,
            _ => Self::Voided,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RedirectUrl {
    pub redirect_url: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckoutMethod {
    pub redirect: RedirectUrl,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IatapayPaymentsResponse {
    pub status: IatapayPaymentStatus,
    pub iata_payment_id: String,
    pub merchant_id: String,
    pub merchant_payment_id: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub country: String,
    pub locale: String,
    pub bank_transfer_description: Option<String>,
    pub checkout_methods: Option<CheckoutMethod>,
    pub failure_code: Option<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, IatapayPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, IatapayPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let form_fields = HashMap::new();
        let id = item.response.iata_payment_id;
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: item.response.checkout_methods.map_or(
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(id.clone()),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                }),
                |checkout_methods| {
                    Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(id),
                        redirection_data: Some(services::RedirectForm::Form {
                            endpoint: checkout_methods.redirect.redirect_url.to_string(),
                            method: services::Method::Get,
                            form_fields,
                        }),
                        mandate_reference: None,
                        connector_metadata: None,
                    })
                },
            ),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IatapayRefundRequest {
    pub merchant_id: String,
    pub merchant_refund_id: String,
    pub amount: i64,
    pub currency: String,
    pub bank_transfer_description: Option<String>,
    pub notification_url: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for IatapayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount,
            merchant_id: IatapayAuthType::try_from(&item.connector_auth_type)?.merchant_id,
            merchant_refund_id: match item.request.connector_refund_id.clone() {
                Some(val) => val,
                None => item.request.refund_id.clone(),
            },
            currency: item.request.currency.to_string(),
            bank_transfer_description: item.request.reason.clone(),
            notification_url: "https://enymh9hvh5dxd.x.pipedream.net/".to_string(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    #[default] Created,
    Locked,
    Initiated,
    Authorized,
    Settled,
    Failed
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Created => Self::Pending,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Locked => Self::Pending,
            RefundStatus::Initiated => Self::Pending,
            RefundStatus::Authorized => Self::Pending,
            RefundStatus::Settled => Self::Success,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    iata_refund_id: String,
    status: RefundStatus,
    merchant_refund_id: String,
    amount: f64,
    currency: String,
    bank_transfer_description: Option<String>,
    failure_code: Option<String>,
    failure_details: Option<String>,
    lock_reason: Option<String>,
    creation_date_time: Option<String>,
    finish_date_time: Option<String>,
    update_date_time: Option<String>,
    clearance_date_time: Option<String>,
    iata_payment_id: Option<String>,
    merchant_payment_id: Option<String>,
    payment_amount: Option<f64>,
    merchant_id: Option<String>,
    account_country: Option<String>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.iata_refund_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.iata_refund_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct IatapayErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct IatapayAccessTokenErrorResponse {
    pub error: String,
    pub error_description: String,
}
