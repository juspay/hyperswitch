use std::collections::HashMap;

use crate::{
    core::errors,
    services,
    types::{self, api, storage::enums},
};
use serde::{Deserialize, Serialize};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct OpennodePaymentsRequest {
    amount: i64,
    currency: String,
    description: String,
    ttl: i64,
    auto_settle: bool,
    success_url: String,
    callback_url: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for OpennodePaymentsRequest {
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
pub struct OpennodeAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for OpennodeAuthType {
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
pub enum OpennodePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<OpennodePaymentStatus> for enums::AttemptStatus {
    fn from(item: OpennodePaymentStatus) -> Self {
        match item {
            OpennodePaymentStatus::Succeeded => Self::Charged,
            OpennodePaymentStatus::Failed => Self::Failure,
            OpennodePaymentStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpennodePaymentsResponseData {
    id: String,
    hosted_checkout_url: String,
    status: String,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpennodePaymentsResponse {
    data: OpennodePaymentsResponseData,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, OpennodePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            OpennodePaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let form_fields = HashMap::new();
        let redirection_data = services::RedirectForm {
            endpoint: item.response.data.hosted_checkout_url.to_string(),
            method: services::Method::Get,
            form_fields,
        };
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
pub struct OpennodeRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for OpennodeRefundRequest {
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
pub struct OpennodeErrorResponse {}

fn get_crypto_specific_payment_data<'a>(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<OpennodePaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let amount = item.request.amount;
    let currency = item.request.currency.to_string();
    let description = item.description.as_ref().unwrap().to_string();
    let ttl = 10i64;
    let auto_settle = true;
    let success_url = item.return_url.as_ref().unwrap().to_string();
    let callback_url = item.return_url.as_ref().unwrap().to_string();

    Ok(OpennodePaymentsRequest {
        amount,
        currency,
        description,
        ttl,
        auto_settle,
        success_url,
        callback_url,
    })
}
