use std::{str::FromStr, fmt::format};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::AccessTokenRequestInfo,
    consts,
    core::errors,
    pii::{self, Secret, PeekInterface},
    types::{self, api, storage::enums},
    utils::OptionExt,
};

use super::Mollie;

//TODO: Fill the struct with respective fields
#[derive( Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MolliePaymentsRequest {
    amount: AmountData,
    description: String,
    redirect_url: String,
}

#[derive( Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmountData {
    currency: enums::Currency,
    value: String,
}

#[derive( Debug, Serialize, Eq, PartialEq)]
pub struct MollieAuthType {
    pub(super) api_key: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PaymentDetails {
    #[serde(rename = "card")]
    Card(CardDetails),
    #[serde(rename = "bank")]
    BankAccount(BankDetails),
    Wallet,
    Klarna,
    Paypal,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CardDetails {
    #[serde(rename = "card.number")]
    pub card_number: String,
    #[serde(rename = "card.holder")]
    pub card_holder: String,
    #[serde(rename = "card.expiryMonth")]
    pub card_expiry_month: String,
    #[serde(rename = "card.expiryYear")]
    pub card_expiry_year: String,
    #[serde(rename = "card.cvv")]
    pub card_cvv: String,
}

#[derive( Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BankDetails {
    billing_email: String,
}



impl TryFrom<&types::PaymentsAuthorizeRouterData> for MolliePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {

        let mut integer_amount = item.request.amount.to_string().to_owned();
        let trailing_zeroes = ".00".to_owned();
        integer_amount.push_str(&trailing_zeroes);
        let amount = AmountData {
            currency: item.request.currency,
            value: integer_amount
        };
        let description = item.description.clone().unwrap_or("Description".to_string());
        let redirect_url = item.return_url.clone().unwrap_or("  HTTP".to_string());

        Ok(MolliePaymentsRequest {
            amount,
            description,
            redirect_url
        })

    } 
}


impl TryFrom<&types::ConnectorAuthType> for MollieAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MolliePaymentStatus {
    Open,
    Canceled,
    Pending,
    Authorized,
    Expired,
    Failed,
    #[default]
    Paid,

}

impl From<MolliePaymentStatus> for enums::AttemptStatus {
    fn from(item: MolliePaymentStatus) -> Self {
        match item {
            MolliePaymentStatus::Paid => Self::Charged,
            MolliePaymentStatus::Failed => Self::Failure,
            MolliePaymentStatus::Pending => Self::Authorizing,
            MolliePaymentStatus::Open => Self::Started,
            MolliePaymentStatus::Canceled => Self::Voided,
            MolliePaymentStatus::Authorized => Self::Authorized,
            MolliePaymentStatus::Expired => Self::Failure
        }
    }
}

//TODO: Fill the struct with respective fields

#[derive(Default, Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
pub struct MollieAmountType {
    currency: String,
    value: String
}

#[derive(Default, Debug, PartialEq,Deserialize, Serialize, Clone)]
pub struct Links {
    href: String,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Default, Debug, PartialEq,Deserialize, Serialize, Clone)]
pub struct MollieLinks {
    #[serde(rename = "self")]
    self_: Links,
    checkout: Links,
    dashboard: Links,
    documentation: Links
}

#[derive(Default, Debug, PartialEq,Deserialize, Serialize, Clone)]
pub struct MolliePaymentsResponse {
    resource: String,
    id: String,
    mode: String,
    #[serde(rename = "createdAt")]
    createdat: String,
    amount: MollieAmountType,
    description: String,
    method: String,
    metadata: Option<String>,
    status: MolliePaymentStatus,
    #[serde(rename = "isCancelable")]
    iscancelable: bool,
    #[serde(rename = "expiresAt")]
    expiresat: String,
    #[serde(rename = "profileId")]
    profileid: String,
    #[serde(rename = "sequenceType")]
    sequencetype: String,
    #[serde(rename = "redirectUrl")]
    redirecturl: String,
    #[serde(rename = "settlementAmount")]
    settlementamount: MollieAmountType,
    #[serde(rename = "_links")]
    links: MollieLinks,
}




impl<F,T> TryFrom<types::ResponseRouterData<F, MolliePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, MolliePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
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
pub struct MollieRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for MollieRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
       todo!()
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
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
         todo!()
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct MollieErrorResponse {}
