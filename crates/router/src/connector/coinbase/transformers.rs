use std::collections::HashMap;

use crate::{
    core::errors,
    services,
    types::{self, api, storage::enums},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct LocalPrice {
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct Metadata {
    pub customer_id: String,
    pub customer_name: String,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CoinbasePaymentsRequest {
    pub name: String,
    pub description: String,
    pub pricing_type: String,
    pub local_price: LocalPrice,
    pub metadata: Metadata,
    pub redirect_url: String,
    pub cancel_url: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CoinbasePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        get_crypto_specific_payment_data(_item)
        // Err(errors::ConnectorError::NotImplemented(
        //     "try_from PaymentsAuthorizeRouterData".to_string(),
        // )
        // .into())
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct CoinbaseAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for CoinbaseAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = _auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
        // Err(errors::ConnectorError::NotImplemented("try_from ConnectorAuthType".to_string()).into())
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CoinbasePaymentStatus {
    #[serde(rename = "NEW")]
    New,
    Pending,
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<CoinbasePaymentStatus> for enums::AttemptStatus {
    fn from(item: CoinbasePaymentStatus) -> Self {
        match item {
            CoinbasePaymentStatus::New => Self::Charged,
            CoinbasePaymentStatus::Pending => Self::Authorizing,
            CoinbasePaymentStatus::Succeeded => Self::Charged,
            CoinbasePaymentStatus::Failed => Self::Failure,
            CoinbasePaymentStatus::Processing => Self::Authorizing,
            _ => Self::Authorizing,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Timeline {
    status: CoinbasePaymentStatus,
    time: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoinbasePaymentResponseData {
    id: String,
    hosted_url: String,
    timeline: Vec<Timeline>,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoinbasePaymentsResponse {
    // status: CoinbasePaymentStatus,
    // id: String,
    data: CoinbasePaymentResponseData,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, CoinbasePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CoinbasePaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let form_fields = HashMap::new();
        let redirection_data = services::RedirectForm {
            endpoint: item.response.data.hosted_url.to_string(),
            method: services::Method::Get,
            form_fields,
        };
        let len = item.response.data.timeline.clone().len();
        // let my_status = item.response.data.timeline[len - 1].status;
        println!("## Redirection_data: {:?}", redirection_data);
        Ok(Self {
            // my_status,
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.data.id),
                redirection_data: Some(redirection_data),
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct CoinbaseRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for CoinbaseRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented("try_from RefundsRouterData".to_string()).into())
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
pub struct RefundResponse {}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented(
            "try_from RefundsResponseRouterData".to_string(),
        )
        .into())
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented(
            "try_from RefundsResponseRouterData".to_string(),
        )
        .into())
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CoinbaseErrorResponse {}

fn get_crypto_specific_payment_data<'a>(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<CoinbasePaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let name = "John".to_string();
    let description = item.description.as_ref().unwrap().to_string();
    let pricing_type = match item.request.payment_method_data {
        api::PaymentMethodData::Crypto(ref crypto_data) => match crypto_data {
            api_models::payments::CryptoData::Coinbase { pricing_type } => pricing_type.into(),
            _ => "fixed_type".to_string(),
        },
        _ => "fixed_type".to_string(),
    };
    let local_price = get_local_price(item);
    let metadata = get_metadata(item);
    let redirect_url = item.return_url.as_ref().unwrap().to_string();
    let cancel_url = item.return_url.as_ref().unwrap().to_string();

    Ok(CoinbasePaymentsRequest {
        name,
        description,
        pricing_type,
        local_price,
        metadata,
        redirect_url,
        cancel_url,
    })
}

fn get_local_price(item: &types::PaymentsAuthorizeRouterData) -> LocalPrice {
    LocalPrice {
        amount: format!("{:?}", item.request.amount),
        currency: item.request.currency.to_string(),
    }
}

fn get_metadata(item: &types::PaymentsAuthorizeRouterData) -> Metadata {
    Metadata {
        customer_id: "112".to_string(),
        customer_name: "John".to_string(),
    }
}
