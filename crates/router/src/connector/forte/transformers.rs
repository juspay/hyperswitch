use api_models::payments as api_models;
use common_utils::pii;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    types::{self, api, storage::enums},
};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct FortePaymentsRequest {
    pub authorization_amount: i64,
    pub subtotal_amount: i64,
    pub billing_address: BillingAddress,
    pub card: CardDetails,
}

#[derive(Default, Debug, Clone, Serialize, Eq, PartialEq, Deserialize)]
pub struct BillingAddress {
    pub first_name: String,
    pub last_name: String,
    pub physical_address: Option<PhysicalAddress>,
}

#[derive(Default, Debug, Clone, Serialize, Eq, PartialEq, Deserialize)]
pub struct PhysicalAddress {
    street_line1: Option<String>,
    street_line2: Option<String>,
    locality: Option<String>,
    region: Option<String>,
    country: Option<String>,
    postal_code: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Eq, PartialEq, Deserialize)]
pub struct CardDetails {
    pub account_number: Secret<String, pii::CardNumber>,
    pub expire_year: Secret<String>,
    pub expire_month: Secret<String>,
    pub name_on_card: String,
    pub card_type: String,
    pub card_verification_value: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match _item.request.payment_method_data {
            api::PaymentMethod::Card(ref card) => make_card_request(&_item.request, card),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

fn make_card_request(
    req: &types::PaymentsAuthorizeData,
    ccard: &api_models::Card,
) -> Result<FortePaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let card = CardDetails {
        account_number: ccard.card_number.clone(),
        expire_month: ccard.card_exp_month.clone(),
        expire_year: ccard.card_exp_year.clone(),
        card_verification_value: ccard.card_cvc.clone(),
        name_on_card: format!("Jennifer McFly"),
        card_type: "visa".to_string(),
    };
    let billing_address = BillingAddress {
        first_name: format!("Jennifer"), // address.as_ref().and_then(|a| a.first_name.clone()),
        last_name: format!("McFly"),     //address.as_ref().and_then(|a| a.last_name.clone()),
        physical_address: None,
    };

    Ok(FortePaymentsRequest {
        authorization_amount: req.amount,
        subtotal_amount: req.amount,
        card,
        billing_address,
    })
}
//TODO: Fill the struct with respective fields
// Auth Struct
pub struct ForteAuthType {
    pub api_key: String,
    pub api_secret: String,
    pub organization_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = _auth_type
        {
            Ok(Self {
                api_key: api_key.to_string(),
                api_secret: api_secret.to_string(),
                organization_key: key1.to_string(),
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
pub enum FortePaymentStatus {
    Authorized,
    Declined,
    Review,
    Settled,
    Voided,
    #[default]
    Ready,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]

pub struct FortePaymentSyncResponse {
    pub transaction_id: String,
    pub organization_id: String,
    pub location_id: String,
    pub action: String,
    pub status: FortePaymentStatus,
    pub authorization_amount: f32,
    pub authorization_code: f32,
    pub received_date: String,
    pub billing_address: BillingAddress,
    pub card: CardResponseDetails,
    pub response: TestResponse,
    pub list: SelfLinks,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, FortePaymentSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            FortePaymentSyncResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id,
                ),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl From<FortePaymentStatus> for enums::AttemptStatus {
    fn from(item: FortePaymentStatus) -> Self {
        match item {
            FortePaymentStatus::Authorized => Self::Authorized,
            FortePaymentStatus::Declined => Self::Failure,
            FortePaymentStatus::Review => Self::Pending,
            FortePaymentStatus::Settled => Self::Charged,
            FortePaymentStatus::Voided => Self::Voided,
            FortePaymentStatus::Ready => Self::Started,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]

pub struct FortePaymentsResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub action: String,
    pub authorization_amount: f32,
    pub entered_by: String,
    pub billing_address: BillingAddress,
    pub card: CardResponseDetails,
    pub response: TestResponse,
    pub list: Option<SelfLinks>,
    pub authorization_code: String,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestResponse {
    environment: String,
    response_type: String,
    response_code: String,
    response_desc: String,
    authorization_code: String,
    avs_result: Option<String>,
    cvv_result: Option<String>,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardResponseDetails {
    pub expire_year: i32,
    pub expire_month: i32,
    pub name_on_card: String,
    pub last_4_account_number: String,
    pub masked_account_number: String,
    pub card_type: String,
    pub customer_accounting_code: Option<String>,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfLinks {
    pub disputes: String,
    pub settlements: String,
    #[serde(rename = "self")]
    pub self_: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        if item.response.response.response_type == "A" {
            Ok(Self {
                status: enums::AttemptStatus::Authorized,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transaction_id,
                    ),
                    redirection_data: None,
                    redirect: false,
                    mandate_reference: None,
                    connector_metadata: None,
                }),
                authorization_code: Some(item.response.authorization_code),
                ..item.data
            })
        } else {
            Ok(Self {
                status: enums::AttemptStatus::AuthorizationFailed,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transaction_id,
                    ),
                    redirection_data: None,
                    redirect: false,
                    mandate_reference: None,
                    connector_metadata: None,
                }),
                authorization_code: Some(item.response.authorization_code),
                ..item.data
            })
        }
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ForteRefundRequest {
    action: String,
    authorization_amount: i64,
    original_transaction_id: String,
    authorization_code: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ForteRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        if let Some(val) = _item.authorization_code.as_ref().clone() {
            Ok(ForteRefundRequest {
                action: format!("reverse"),
                authorization_amount: _item.request.amount,
                original_transaction_id: _item.request.connector_transaction_id.clone(),
                authorization_code: val.to_string(),
            })
        } else {
            Ok(ForteRefundRequest {
                action: format!("reverse"),
                authorization_amount: _item.request.amount,
                original_transaction_id: _item.request.connector_transaction_id.clone(),
                authorization_code: "auth".to_string(),
            })
        }
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
    transaction_id: String,
    location_id: String,
    original_transaction_id: String,
    action: String,
    authorization_amount: f64,
    authorization_code: String,
    entered_by: String,
    billing_address: BillingAddress,
    response: TestResponse,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        return Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: _item.response.transaction_id.clone(),
                refund_status: (enums::RefundStatus::from(self::RefundStatus::Succeeded)),
            }),
            .._item.data
        });
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FortePaymentsCancelReq {
    action: String,
    authorization_code: String,
}

impl TryFrom<&types::PaymentsCancelRouterData> for FortePaymentsCancelReq {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: &types::PaymentsCancelRouterData,
    ) -> Result<FortePaymentsCancelReq, Self::Error> {
        if let Some(val) = _item.authorization_code.as_ref().clone() {
            Ok(FortePaymentsCancelReq {
                action: format!("void"),
                authorization_code: val.to_string(),
            })
        } else {
            Ok(FortePaymentsCancelReq {
                action: format!("void"),
                authorization_code: "auth".to_string(),
            })
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FortePaymentCancelResponse {
    transaction_id: String,
    location_id: String,
    authorization_code: String,
    action: String,
    entered_by: String,
    response: TestResponse,
    links: SelfLinks,
}

impl TryFrom<types::PaymentsCancelResponseRouterData<FortePaymentCancelResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsCancelResponseRouterData<FortePaymentCancelResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(enums::AttemptStatus::Authorized),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id,
                ),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}
