use common_enums::enums;
use common_utils::{request::Method, types::StringMinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankDebitData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::RouterData as _,
};

//TODO: Fill the struct with respective fields
pub struct InespayRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for InespayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InespayPaymentsRequest {
    description: String,
    amount: StringMinorUnit,
    reference: String,
    debtor_account: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct InespayCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&InespayRouterData<&PaymentsAuthorizeRouterData>> for InespayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &InespayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankDebit(BankDebitData::SepaBankDebit { iban, .. }) => {
                let order_id = item.router_data.connector_request_reference_id.clone();
                Ok(Self {
                    description: item.router_data.get_description()?,
                    amount: item.amount.clone(),
                    reference: order_id,
                    debtor_account: iban,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct InespayAuthType {
    pub(super) api_key: Secret<String>,
    pub authorization: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for InespayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                authorization: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InespayPaymentsResponse {
    status: String,
    status_desc: String,
    single_payin_id: String,
    single_payin_link: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, InespayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, InespayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_url = Url::parse(item.response.single_payin_link.as_str())
            .change_context(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
        let redirection_data = RedirectForm::from((redirection_url, Method::Get));
        let status = match item.response.status_desc.as_str() {
            "Success" => common_enums::AttemptStatus::AuthenticationPending,
            _ => common_enums::AttemptStatus::Failure,
        };

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.single_payin_id.clone(),
                ),
                redirection_data: Box::new(Some(redirection_data)),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InespayPSyncStatus {
    Ok,
    Created,
    Opened,
    BankSelected,
    Initiated,
    Pending,
    Aborted,
    Unfinished,
    Rejected,
    Cancelled,
    PartiallyAccepted,
    Failed,
    Settled,
    PartRefunded,
    Refunded,
}

impl From<InespayPSyncStatus> for common_enums::AttemptStatus {
    fn from(item: InespayPSyncStatus) -> Self {
        match item {
            InespayPSyncStatus::Ok | InespayPSyncStatus::Settled => Self::Charged,
            InespayPSyncStatus::Created
            | InespayPSyncStatus::Opened
            | InespayPSyncStatus::BankSelected
            | InespayPSyncStatus::Initiated
            | InespayPSyncStatus::Pending
            | InespayPSyncStatus::Unfinished
            | InespayPSyncStatus::PartiallyAccepted => Self::AuthenticationPending,
            InespayPSyncStatus::Aborted
            | InespayPSyncStatus::Rejected
            | InespayPSyncStatus::Cancelled
            | InespayPSyncStatus::Failed => Self::Failure,
            InespayPSyncStatus::PartRefunded | InespayPSyncStatus::Refunded => Self::AutoRefunded,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InespayPSyncResponse {
    cod_status: InespayPSyncStatus,
    status_desc: String,
    single_payin_id: String,
    single_payin_link: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, InespayPSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, InespayPSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_url = Url::parse(item.response.single_payin_link.as_str())
            .change_context(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
        let redirection_data = RedirectForm::from((redirection_url, Method::Get));

        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.cod_status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.single_payin_id.clone(),
                ),
                redirection_data: Box::new(Some(redirection_data)),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct InespayRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&InespayRouterData<&RefundsRouterData<F>>> for InespayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &InespayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct InespayErrorResponse {
    pub message: String,
}
