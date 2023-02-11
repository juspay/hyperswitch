use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use storage_models::enums as storage_enums;

use crate::{
    connector::utils::AccessTokenRequestInfo,
    // consts,
    core::errors,
    pii::{self, Secret},
    types::{self, api, storage::enums},
    // utils::OptionExt,
};
//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BamboraPaymentsRequest {
    pub amount: i64,
    #[serde(rename = "payment_method")]
    pub payment_method: String,
    pub card: BamboraCard,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BamboraCard {
    pub name: Secret<String>,
    pub number: Secret<String, pii::CardNumber>,
    #[serde(rename = "expiry_month")]
    pub expiry_month: Secret<String>,
    #[serde(rename = "expiry_year")]
    pub expiry_year: Secret<String>,
    pub cvd: Secret<String>,
    pub complete: bool,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BamboraPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let auth_type = BamboraAuthType::try_from(&item.connector_auth_type)?;
        let payment_method_detail = match item.request.payment_method_data.clone() {
            api::PaymentMethod::Card(ccard) => Ok(BamboraCard {
                    name: ccard.card_holder_name,
                    number: ccard.card_number,
                    expiry_month: ccard.card_exp_month,
                    expiry_year: ccard.card_exp_year,
                    cvd: ccard.card_cvc,
                    complete: item.request.capture_method == Some(storage_enums::CaptureMethod::Manual),
                }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Unknown payment method".to_string(),
            )),
        }?;
        Ok(Self {
            
            card: payment_method_detail,
            amount: item.request.amount,
            payment_method: String::from("card"),
        })
    }
}

// impl From<Option<enums::CaptureMethod>> for bool {
//     fn from(item: Option<enums::CaptureMethod>) -> Self {
//         match item {
//             Some(p) => match p {
//                 enums::CaptureMethod::ManualMultiple |
//                 enums::CaptureMethod::Manual |
//                 enums::CaptureMethod::Scheduled => false,
//                 enums::CaptureMethod::Automatic => true,
//             },
//             None => true,
//         }
//     }
// }

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct BamboraAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for BamboraAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BamboraPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<BamboraPaymentStatus> for enums::AttemptStatus {
    fn from(item: BamboraPaymentStatus) -> Self {
        match item {
            BamboraPaymentStatus::Succeeded => Self::Charged,
            BamboraPaymentStatus::Failed => Self::Failure,
            BamboraPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BamboraPaymentsResponse {
    status: BamboraPaymentStatus,
    id: String,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
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
pub struct BamboraRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BamboraRefundRequest {
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
pub struct BamboraErrorResponse {}
