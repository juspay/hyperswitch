use common_enums::enums;
use common_utils::types::MinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

//TODO: Fill the struct with respective fields
pub struct GlobalpaymentsRealexRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for GlobalpaymentsRealexRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct GlobalpaymentsRealexPaymentsRequest {
    amount: MinorUnit,
    card: GlobalpaymentsRealexCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct GlobalpaymentsRealexCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&GlobalpaymentsRealexRouterData<&PaymentsAuthorizeRouterData>>
    for GlobalpaymentsRealexPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &GlobalpaymentsRealexRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(_) => Err(errors::ConnectorError::NotImplemented(
                "Card payment method not implemented".to_string(),
            )
            .into()),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Auth Struct
//
// Global Payments RealEx (legacy XML API) is routed through the Unified Connector
// Service (UCS); this struct only validates the shape of the credentials stored on
// the merchant connector account.
// - `api_key`    => Shared Secret (input to the sha1 digest; never sent verbatim)
// - `key1`       => Merchant Id (Client Id), sent as the `<merchantid>` element
// - `key2`       => Account, the RealEx sub-account, sent as the `<account>` element
// - `api_secret` => Refund (rebate) password, hashed into `<refundhash>`
pub struct GlobalpaymentsRealexAuthType {
    #[allow(dead_code)]
    pub(super) shared_secret: Secret<String>,
    #[allow(dead_code)]
    pub(super) merchant_id: Secret<String>,
    #[allow(dead_code)]
    pub(super) account: Secret<String>,
    #[allow(dead_code)]
    pub(super) refund_password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for GlobalpaymentsRealexAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                key2,
                api_secret,
            } => Ok(Self {
                shared_secret: api_key.to_owned(),
                merchant_id: key1.to_owned(),
                account: key2.to_owned(),
                refund_password: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GlobalpaymentsRealexPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<GlobalpaymentsRealexPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: GlobalpaymentsRealexPaymentStatus) -> Self {
        match item {
            GlobalpaymentsRealexPaymentStatus::Succeeded => Self::Charged,
            GlobalpaymentsRealexPaymentStatus::Failed => Self::Failure,
            GlobalpaymentsRealexPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlobalpaymentsRealexPaymentsResponse {
    status: GlobalpaymentsRealexPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<ResponseRouterData<F, GlobalpaymentsRealexPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, GlobalpaymentsRealexPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
                payment_account_reference: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct GlobalpaymentsRealexRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&GlobalpaymentsRealexRouterData<&RefundsRouterData<F>>>
    for GlobalpaymentsRealexRefundRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &GlobalpaymentsRealexRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct GlobalpaymentsRealexErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}
