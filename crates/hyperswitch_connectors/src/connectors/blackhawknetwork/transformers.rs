use common_enums::{enums, Currency};
use common_utils::types::{MinorUnit, StringMajorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{GiftCardData, PaymentMethodData},
    router_data::{
        AccessToken, ConnectorAuthType, ErrorResponse, PaymentMethodBalance, RouterData,
    },
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, PreprocessingResponseId, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsPreProcessingRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{consts::NO_ERROR_MESSAGE, errors};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::types::{
    PaymentsPreprocessingResponseRouterData, RefundsResponseRouterData, ResponseRouterData,
};

pub struct BlackhawknetworkRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for BlackhawknetworkRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlackhawknetworkAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
    pub(super) product_line_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BlackhawknetworkAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                client_id: api_key.clone(),
                client_secret: api_secret.clone(),
                product_line_id: key1.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)
                .attach_printable("Unsupported authentication type for Blackhawk Network"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlackhawknetworkAccessTokenRequest {
    pub grant_type: String,
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub scope: String,
}
impl<F, T> TryFrom<ResponseRouterData<F, BlackhawknetworkTokenResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BlackhawknetworkTokenResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlackhawknetworkVerifyAccountRequest {
    pub account_number: Secret<String>,
    pub product_line_id: Secret<String>,
    pub account_type: AccountType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvv2: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<Secret<String>>,
}
impl TryFrom<&PaymentsPreProcessingRouterData> for BlackhawknetworkVerifyAccountRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsPreProcessingRouterData) -> Result<Self, Self::Error> {
        let auth = BlackhawknetworkAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let gift_card_data = match &item.request.payment_method_data {
            Some(PaymentMethodData::GiftCard(gc)) => match gc.as_ref() {
                GiftCardData::BhnCardNetwork(data) => data,
                _ => {
                    return Err(errors::ConnectorError::FlowNotSupported {
                        flow: "Balance".to_string(),
                        connector: "BlackHawkNetwork".to_string(),
                    }
                    .into())
                }
            },
            _ => {
                return Err(errors::ConnectorError::FlowNotSupported {
                    flow: "Balance".to_string(),
                    connector: "BlackHawkNetwork".to_string(),
                }
                .into())
            }
        };

        Ok(Self {
            account_number: gift_card_data.account_number.clone(),
            product_line_id: auth.product_line_id,
            account_type: AccountType::GiftCard,
            pin: gift_card_data.pin.clone(),
            cvv2: gift_card_data.cvv2.clone(),
            expiration_date: gift_card_data.expiration_date.clone().map(Secret::new),
        })
    }
}

impl TryFrom<PaymentsPreprocessingResponseRouterData<BlackhawknetworkVerifyAccountResponse>>
    for PaymentsPreProcessingRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsPreprocessingResponseRouterData<BlackhawknetworkVerifyAccountResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::PreProcessingResponse {
                pre_processing_id: PreprocessingResponseId::PreProcessingId(
                    item.response.account.entity_id,
                ),
                connector_metadata: None,
                session_token: None,
                connector_response_reference_id: None,
            }),
            payment_method_balance: Some(PaymentMethodBalance {
                currency: item.response.account.currency,
                amount: item.response.account.balance,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlackhawknetworkTokenResponse {
    pub access_token: Secret<String>,
    pub expires_in: i64,
}
#[derive(Serialize, Debug)]
pub struct BlackhawknetworkPaymentsRequest {
    pub account_id: String,
    pub amount: StringMajorUnit,
    pub currency: Currency,
}

impl TryFrom<&BlackhawknetworkRouterData<&PaymentsAuthorizeRouterData>>
    for BlackhawknetworkPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &BlackhawknetworkRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match &item.router_data.request.payment_method_data {
            PaymentMethodData::GiftCard(_gift_card) => {
                let account_id = item
                    .router_data
                    .preprocessing_id
                    .to_owned()
                    .ok_or_else(|| {
                        errors::ConnectorError::MissingConnectorRelatedTransactionID {
                            id: "entity_id".to_string(),
                        }
                    })?;

                Ok(Self {
                    account_id,
                    amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                })
            }
            _ => Err(error_stack::Report::new(
                errors::ConnectorError::NotImplemented("Non-gift card payment method".to_string()),
            )),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BlackhawknetworkRedeemResponse {
    Success(BlackhawknetworkPaymentsResponse),
    Error(BlackhawknetworkErrorResponse),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlackhawknetworkPaymentsResponse {
    pub id: String,
    #[serde(rename = "transactionStatus")]
    pub status: BlackhawknetworkAttemptStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BlackhawknetworkAttemptStatus {
    Approved,
    Declined,
    Pending,
}

impl<F, T> TryFrom<ResponseRouterData<F, BlackhawknetworkRedeemResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, BlackhawknetworkRedeemResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BlackhawknetworkRedeemResponse::Success(response) => Ok(Self {
                status: match response.status {
                    BlackhawknetworkAttemptStatus::Approved => enums::AttemptStatus::Charged,
                    BlackhawknetworkAttemptStatus::Declined => enums::AttemptStatus::Failure,
                    BlackhawknetworkAttemptStatus::Pending => enums::AttemptStatus::Pending,
                },
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(response.id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            BlackhawknetworkRedeemResponse::Error(error_response) => Ok(Self {
                response: Err(ErrorResponse {
                    status_code: item.http_code,
                    code: error_response.error.clone(),
                    message: error_response
                        .error_description
                        .clone()
                        .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                    reason: error_response.error_description,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct BlackhawknetworkRefundRequest {
    pub amount: StringMajorUnit,
}

impl<F> TryFrom<&BlackhawknetworkRouterData<&RefundsRouterData<F>>>
    for BlackhawknetworkRefundRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BlackhawknetworkRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
    CreditCard,
    GiftCard,
    LoyaltyCard,
    PhoneCard,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountStatus {
    New,
    Activated,
    Closed,
}

impl From<AccountStatus> for common_enums::AttemptStatus {
    fn from(item: AccountStatus) -> Self {
        match item {
            AccountStatus::New | AccountStatus::Activated => Self::Pending,
            AccountStatus::Closed => Self::Failure,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountInformation {
    pub entity_id: String,
    pub balance: MinorUnit,
    pub currency: Currency,
    pub status: AccountStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlackhawknetworkVerifyAccountResponse {
    account: AccountInformation,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlackhawknetworkErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}
