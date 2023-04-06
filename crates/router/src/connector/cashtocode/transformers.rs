

use serde::{Deserialize, Serialize};
use masking::Secret;
use router_env::logger;
use crate::
    {connector::utils::{self},
    core::errors,
    services,
    types::{self, api, storage::enums},
};
use common_utils::{pii::Email};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CashtocodePaymentsRequest {
    amount: i64,
    transaction_id: String,
    user_id: String,
    currency: String,
    first_name: Option<String>,
    last_name: Option<String>,
    user_alias: String,
    requested_url: String,
    cancel_url: String,
    email: Option<Secret<String, Email>>,
    mid: String,
}

pub struct CashToCodeMandatoryParams {
    pub user_id: String,
    pub user_alias: String,
    pub requested_url: String,
    pub cancel_url: String,
}

fn get_mid(payment_method_data : &api::payments::PaymentMethodData,) -> Result<String,errors::ConnectorError>
{
    match payment_method_data{
        api_models::payments::PaymentMethodData::Reward(reward_data) =>
        Ok(reward_data.mid.to_string()),
        _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
    }
}

fn get_mandatory_params(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<CashToCodeMandatoryParams, error_stack::Report<errors::ConnectorError>> {
    let customer_id = item
        .request
        .customer_id
        .as_ref()
        .ok_or_else(utils::missing_field_err("customer_id"))?;
    let url = item.return_url.to_owned().ok_or_else(
    utils::missing_field_err("return_url"))?;
    Ok(CashToCodeMandatoryParams {
        user_id: customer_id.to_owned(),
        user_alias: customer_id.to_owned(),
        requested_url: url.to_owned(),
        cancel_url: url,
    })
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CashtocodePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let params: CashToCodeMandatoryParams = get_mandatory_params(item)?;
        let mid_helper = get_mid(&item.request.payment_method_data);
        let mid = match mid_helper {
            Ok(mid) => mid,
            Err(err) => return Err(err.into()),
        };
        match item.payment_method {
            storage_models::enums::PaymentMethod::Reward => Ok( CashtocodePaymentsRequest {
                    amount: item.request.amount,
                    transaction_id: item.attempt_id.clone(),
                    currency: item.request.currency.to_string(),
                    user_id: params.user_id,
                    first_name: None,
                    last_name: None,
                    user_alias: params.user_alias,
                    requested_url: params.requested_url,
                    cancel_url: params.cancel_url,
                    email: item.request.email.clone(),
                    mid,
        }),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct CashtocodeAuthType {
    pub(super) api_key: String
}

// fn get_appropriate_api_key(
//     payment_method_data: api_models::payments::PaymentMethodData,
//     api_key1: String,
//     api_key2: String,
// ) -> CustomResult< String, errors::ConnectorError,
// >
// {
//     match payment_method_data{
//         api_models::payments::PaymentMethodData::Reward{payment_method_type} =>
//             if payment_method_type == "CLASSIC".to_string()
//             {
//                 Ok(api_key1)
//             }
//             else{
//                 Ok(api_key2)
//             }
//         _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
//     }
// }

impl TryFrom<&types::ConnectorAuthType> for CashtocodeAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    // need paymentMethoddata at this point
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key,} =>
            Ok(Self {
                api_key: api_key.to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CashtocodePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<CashtocodePaymentStatus> for enums::AttemptStatus {
    fn from(item: CashtocodePaymentStatus) -> Self {
        match item {
            CashtocodePaymentStatus::Succeeded => Self::Charged,
            CashtocodePaymentStatus::Failed => Self::Failure,
            CashtocodePaymentStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CashtocodeErrors {
    pub message: String,
    pub path: String,
    pub r#type: String,
}
//TODO: Fill the struct with respective fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CashtocodePaymentsResponse {
    pub pay_url: String,
}

pub struct CashtocodePaymentsSyncResponse {

}

impl<F,T> TryFrom<types::ResponseRouterData<F, CashtocodePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, CashtocodePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        logger::info!(item.response.pay_url);
        let redirection_data = services::RedirectForm {
                    endpoint: item.response.pay_url.clone(),
                    method: services::Method::Post,
                    form_fields: Default::default(),
                };
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.data.attempt_id.clone()),
                redirection_data: Some(redirection_data),
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}


impl<F,T> TryFrom<types::ResponseRouterData<F, CashtocodePaymentsSyncResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, CashtocodePaymentsSyncResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Charged,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.data.attempt_id.clone()),
                redirection_data: None,
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
pub struct CashtocodeRefundRequest {
    pub amount: i64
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for CashtocodeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            amount: item.request.amount,
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
    status: RefundStatus
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

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CashtocodeErrorResponse {
    pub error: String,
    pub error_description: String,
    pub errors: Option<Vec<Box<CashtocodeErrors>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CashtocodeIncomingWebhook {
    pub amount: i64,
    pub currency: String,
    pub foreign_transaction_id: String,
    pub r#type : String,
    pub transaction_id : String,
}
