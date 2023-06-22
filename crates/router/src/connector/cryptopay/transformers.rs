use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    services,
    types::{self, api, storage::enums},
};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CryptopayPaymentsRequest {
    price_amount: i64,
    price_currency: enums::Currency,
    pay_currency: String,
    success_redirect_url: Option<String>,
    unsuccess_redirect_url: Option<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CryptopayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let pay_currency = match item.request.payment_method_data {
            api::PaymentMethodData::Crypto(ref cryptodata) => {
                let pay_currency = cryptodata.pay_currency.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "pay_currency",
                    },
                )?;
                Ok(Self {
                    price_amount: item.request.amount,
                    price_currency: item.request.currency,
                    pay_currency,
                    success_redirect_url: item.clone().request.router_return_url,
                    unsuccess_redirect_url: item.clone().request.router_return_url,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "payment method".to_string(),
            )),
        }?;
        Ok(pay_currency)
    }
}

// Auth Struct
pub struct CryptopayAuthType {
    pub(super) api_key: String,
    pub(super) api_secret: String,
}

impl TryFrom<&types::ConnectorAuthType> for CryptopayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
                api_secret: key1.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CryptopayPaymentStatus {
    #[default]
    New,
    Completed,
    Unresolved,
    Refunded,
    Cancelled,
}

impl From<CryptopayPaymentStatus> for enums::AttemptStatus {
    fn from(item: CryptopayPaymentStatus) -> Self {
        match item {
            CryptopayPaymentStatus::New => Self::AuthenticationPending,
            CryptopayPaymentStatus::Completed | CryptopayPaymentStatus::Refunded => Self::Charged,
            CryptopayPaymentStatus::Cancelled => Self::Failure,
            CryptopayPaymentStatus::Unresolved => Self::Unresolved,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CryptopayPaymentsResponse {
    data: CryptopayPaymentResponseData,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, CryptopayPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CryptopayPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .data
            .hosted_page_url
            .map(|x| services::RedirectForm::from((x, services::Method::Get)));
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.data.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.data.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct CryptopayRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for CryptopayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount,
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

//TODO: Fill the struct with respective fields
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

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CryptopayErrorData {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CryptopayErrorResponse {
    pub error: CryptopayErrorData,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CryptopayPaymentResponseData {
    pub id: String,
    pub customer_id: Option<String>,
    pub status: CryptopayPaymentStatus,
    pub status_context: Option<String>,
    pub address: Option<String>,
    pub network: Option<String>,
    pub uri: Option<String>,
    pub price_amount: Option<String>,
    pub price_currency: Option<String>,
    pub pay_amount: Option<String>,
    pub pay_currency: Option<String>,
    pub fee: Option<String>,
    pub fee_currency: Option<String>,
    pub paid_amount: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub success_redirect_url: Option<String>,
    pub unsuccess_redirect_url: Option<String>,
    pub hosted_page_url: Option<Url>,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
}
