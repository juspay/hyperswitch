use common_utils::pii::Email;
use diesel_models::enums;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, AddressDetailsData, RouterData},
    core::errors,
    services,
    types::{self, api, storage::enums as storage_enums},
};

const PASSWORD: &str = "password";

pub struct VoltRouterData<T> {
    pub amount: i64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for VoltRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPaymentsRequest {
    amount: i64,
    currency_code: storage_enums::Currency,
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    merchant_internal_reference: String,
    shopper: ShopperDetails,
    notification_url: Option<String>,
    payment_success_url: Option<String>,
    payment_failure_url: Option<String>,
    payment_pending_url: Option<String>,
    payment_cancel_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionType {
    Bills,
    Goods,
    PersonToPerson,
    Other,
    Services,
}

#[derive(Debug, Serialize)]
pub struct ShopperDetails {
    reference: String,
    email: Option<Email>,
    first_name: Secret<String>,
    last_name: Secret<String>,
}

impl TryFrom<&VoltRouterData<&types::PaymentsAuthorizeRouterData>> for VoltPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &VoltRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::BankRedirect(ref bank_redirect) => match bank_redirect {
                api_models::payments::BankRedirectData::OpenBankingUk { .. } => {
                    let amount = item.amount;
                    let currency_code = item.router_data.request.currency;
                    let merchant_internal_reference =
                        item.router_data.connector_request_reference_id.clone();
                    let payment_success_url = item.router_data.request.router_return_url.clone();
                    let payment_failure_url = item.router_data.request.router_return_url.clone();
                    let payment_pending_url = item.router_data.request.router_return_url.clone();
                    let payment_cancel_url = item.router_data.request.router_return_url.clone();
                    let notification_url = item.router_data.request.webhook_url.clone();
                    let address = item.router_data.get_billing_address()?;
                    let shopper = ShopperDetails {
                        email: item.router_data.request.email.clone(),
                        first_name: address.get_first_name()?.to_owned(),
                        last_name: address.get_last_name()?.to_owned(),
                        reference: item.router_data.get_customer_id()?.to_owned(),
                    };
                    let transaction_type = TransactionType::Services; //transaction_type is a form of enum, it is pre defined and value for this can not be taken from user so we are keeping it as Services as this transaction is type of service.

                    Ok(Self {
                        amount,
                        currency_code,
                        merchant_internal_reference,
                        payment_success_url,
                        payment_failure_url,
                        payment_pending_url,
                        payment_cancel_url,
                        notification_url,
                        shopper,
                        transaction_type,
                    })
                }
                api_models::payments::BankRedirectData::BancontactCard { .. }
                | api_models::payments::BankRedirectData::Bizum {}
                | api_models::payments::BankRedirectData::Blik { .. }
                | api_models::payments::BankRedirectData::Eps { .. }
                | api_models::payments::BankRedirectData::Giropay { .. }
                | api_models::payments::BankRedirectData::Ideal { .. }
                | api_models::payments::BankRedirectData::Interac { .. }
                | api_models::payments::BankRedirectData::OnlineBankingCzechRepublic { .. }
                | api_models::payments::BankRedirectData::OnlineBankingFinland { .. }
                | api_models::payments::BankRedirectData::OnlineBankingPoland { .. }
                | api_models::payments::BankRedirectData::OnlineBankingSlovakia { .. }
                | api_models::payments::BankRedirectData::Przelewy24 { .. }
                | api_models::payments::BankRedirectData::Sofort { .. }
                | api_models::payments::BankRedirectData::Trustly { .. }
                | api_models::payments::BankRedirectData::OnlineBankingFpx { .. }
                | api_models::payments::BankRedirectData::OnlineBankingThailand { .. } => {
                    Err(errors::ConnectorError::NotSupported {
                        message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                        connector: "Volt",
                    }
                    .into())
                }
            },
            api_models::payments::PaymentMethodData::Card(_)
            | api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::Wallet(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Volt",
                }
                .into())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VoltAuthUpdateRequest {
    grant_type: String,
    client_id: Secret<String>,
    client_secret: Secret<String>,
    username: Secret<String>,
    password: Secret<String>,
}

impl TryFrom<&types::RefreshTokenRouterData> for VoltAuthUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth = VoltAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            grant_type: PASSWORD.to_string(),
            username: auth.username,
            password: auth.password,
            client_id: auth.client_id,
            client_secret: auth.client_secret,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct VoltAuthUpdateResponse {
    pub access_token: Secret<String>,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, VoltAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, VoltAuthUpdateResponse, T, types::AccessToken>,
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

pub struct VoltAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for VoltAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Ok(Self {
                username: api_key.to_owned(),
                password: api_secret.to_owned(),
                client_id: key1.to_owned(),
                client_secret: key2.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl From<VoltPaymentStatus> for enums::AttemptStatus {
    fn from(item: VoltPaymentStatus) -> Self {
        match item {
            VoltPaymentStatus::Completed
            | VoltPaymentStatus::Received
            | VoltPaymentStatus::Settled => Self::Charged,
            VoltPaymentStatus::DelayedAtBank => Self::Pending,
            VoltPaymentStatus::NewPayment
            | VoltPaymentStatus::BankRedirect
            | VoltPaymentStatus::AwaitingCheckoutAuthorisation => Self::AuthenticationPending,
            VoltPaymentStatus::RefusedByBank
            | VoltPaymentStatus::RefusedByRisk
            | VoltPaymentStatus::NotReceived
            | VoltPaymentStatus::ErrorAtBank
            | VoltPaymentStatus::CancelledByUser
            | VoltPaymentStatus::AbandonedByUser
            | VoltPaymentStatus::Failed => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPaymentsResponse {
    checkout_url: String,
    id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, VoltPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, VoltPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let url = item.response.checkout_url;
        let redirection_data = Some(services::RedirectForm::Form {
            endpoint: url,
            method: services::Method::Get,
            form_fields: Default::default(),
        });
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VoltPaymentStatus {
    NewPayment,
    Completed,
    Received,
    NotReceived,
    BankRedirect,
    DelayedAtBank,
    AwaitingCheckoutAuthorisation,
    RefusedByBank,
    RefusedByRisk,
    ErrorAtBank,
    CancelledByUser,
    AbandonedByUser,
    Failed,
    Settled,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPsyncResponse {
    status: VoltPaymentStatus,
    id: String,
    merchant_internal_reference: Option<String>,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, VoltPsyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, VoltPsyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .merchant_internal_reference
                    .or(Some(item.response.id)),
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct VoltRefundRequest {
    pub amount: i64,
    pub external_reference: String,
}

impl<F> TryFrom<&VoltRouterData<&types::RefundsRouterData<F>>> for VoltRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VoltRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.router_data.request.refund_amount,
            external_reference: item.router_data.request.refund_id.clone(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
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

#[derive(Default, Debug, Clone, Deserialize)]
pub struct RefundResponse {
    id: String,
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
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::Pending, //We get Refund Status only by Webhooks
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct VoltErrorResponse {
    pub exception: VoltErrorException,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VoltErrorException {
    pub code: u64,
    pub message: String,
    pub error_list: Option<Vec<VoltErrorList>>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct VoltErrorList {
    pub property: String,
    pub message: String,
}
