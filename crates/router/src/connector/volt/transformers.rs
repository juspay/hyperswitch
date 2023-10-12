use common_utils::pii::Email;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, AddressDetailsData, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    services,
    types::{self, api, storage::enums},
};

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

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VoltPaymentsRequest {
    amount: i64,
    // card: VoltCard,
    currency_code: String,
    #[serde(rename = "type")]
    transaction_type: String,
    merchant_internal_reference: String,
    shopper: ShopperDetails,
    // notification_url: Option<String>,
    // payment_success_url: Option<String>,
    // payment_failure_url: Option<String>,
    // payment_pending_url: Option<String>,
    // payment_cancel_url: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ShopperDetails {
    reference: String,
    email: Option<Email>,
    first_name: Secret<String>,
    last_name: Secret<String>,
    organisation_name: Secret<String>,
}

// #[derive(Default, Debug, Serialize, Eq, PartialEq)]
// #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
// pub enum TransactionType {
//    Bills,
//    Goods,
//    PersonToPerson,
//    Services,
//    #[default]
//    Other,
// }

impl TryFrom<&VoltRouterData<&types::PaymentsAuthorizeRouterData>> for VoltPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &VoltRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::BankRedirect(ref bank_redirect) => match bank_redirect {
                api_models::payments::BankRedirectData::OpenBankingUk { .. } => {
                    let amount = item.amount;
                    let currency_code = item.router_data.request.currency.to_string();
                    // let transaction_type = ;
                    let merchant_internal_reference =
                        item.router_data.connector_request_reference_id.clone();
                    // let payment_success_url = item.router_data.request.router_return_url.clone();
                    // let payment_failure_url = item.router_data.request.router_return_url.clone();
                    // let payment_pending_url = item.router_data.request.router_return_url.clone();
                    // let payment_cancel_url = item.router_data.request.router_return_url.clone();
                    let address = item.router_data.get_billing_address()?;
                    let shopper = ShopperDetails {
                        email: item.router_data.request.email.clone(),
                        first_name: address.get_first_name()?.to_owned(),
                        last_name: address.get_last_name()?.to_owned(),
                        organisation_name: address.get_full_name()?.to_owned(),
                        reference: item.router_data.get_customer_id()?.to_owned(),
                    };

                    Ok(Self {
                        amount,
                        currency_code,
                        merchant_internal_reference,
                        // payment_success_url,
                        // payment_failure_url,
                        // payment_pending_url,
                        // payment_cancel_url,
                        shopper,
                        // notification_url: Some("https://e3ae-103-159-11-202.ngrok-free.app/webhooks/{merchant-id}/volt".to_string()),
                        transaction_type: "BILL".to_string(),
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
                        connector: "Paypal",
                    }
                    .into())
                }
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

pub struct VoltAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for VoltAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl From<VoltPaymentStatus> for enums::AttemptStatus {
    fn from(item: VoltPaymentStatus) -> Self {
        match item {
            VoltPaymentStatus::Completed => Self::Charged,
            VoltPaymentStatus::NewPayment | VoltPaymentStatus::Processing => {
                Self::AuthenticationPending
            }
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
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
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VoltPaymentStatus {
    NewPayment,
    Completed,
    #[default]
    Processing,
}
#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPsyncResponse {
    status: VoltPaymentStatus,
    id: String,
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
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
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
}

impl<F> TryFrom<&VoltRouterData<&types::RefundsRouterData<F>>> for VoltRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VoltRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
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
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct VoltErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
