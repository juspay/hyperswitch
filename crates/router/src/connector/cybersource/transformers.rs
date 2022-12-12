use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    pii::PeekInterface,
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformation,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct ProcessingInformation {
    capture: bool,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PaymentInformation {
    card: Card,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Card {
    number: String,
    expiration_month: String,
    expiration_year: String,
    security_code: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct OrderInformation {
    amount_details: Amount,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct Amount {
    total_amount: String,
    currency: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CybersourcePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let order_information = OrderInformation {
                    amount_details: Amount {
                        total_amount: item.request.amount.to_string(),
                        currency: item.request.currency.to_string().to_uppercase(),
                    },
                };

                let payment_information = PaymentInformation {
                    card: Card {
                        number: ccard.card_number.peek().clone(),
                        expiration_month: ccard.card_exp_month.peek().clone(),
                        expiration_year: ccard.card_exp_year.peek().clone(),
                        security_code: ccard.card_cvc.peek().clone(),
                    },
                };

                let processing_information = ProcessingInformation {
                    capture: matches!(
                        item.request.capture_method,
                        Some(enums::CaptureMethod::Automatic) | None
                    ),
                };

                Ok(CybersourcePaymentsRequest {
                    processing_information,
                    payment_information,
                    order_information,
                })
            }
            _ => Err(errors::ConnectorError::RequestEncodingFailed.into()),
        }
    }
}

pub struct CybersourceAuthType {
    pub(super) api_key: String,
    pub(super) merchant_account: String,
    pub(super) api_secret: String,
}

impl TryFrom<&types::ConnectorAuthType> for CybersourceAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey {
            api_key,
            key1,
            api_secret,
        } = item
        {
            Ok(Self {
                api_key: api_key.to_string(),
                merchant_account: key1.to_string(),
                api_secret: api_secret.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CybersourcePaymentStatus {
    Authorized,
    Succeeded,
    Failed,
    Processing,
}

// Default should be Processing
impl Default for CybersourcePaymentStatus {
    fn default() -> Self {
        CybersourcePaymentStatus::Processing
    }
}

impl From<CybersourcePaymentStatus> for enums::AttemptStatus {
    fn from(item: CybersourcePaymentStatus) -> Self {
        match item {
            CybersourcePaymentStatus::Authorized => enums::AttemptStatus::Authorized,
            CybersourcePaymentStatus::Succeeded => enums::AttemptStatus::Charged,
            CybersourcePaymentStatus::Failed => enums::AttemptStatus::Failure,
            CybersourcePaymentStatus::Processing => enums::AttemptStatus::Authorizing,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CybersourcePaymentsResponse {
    id: String,
    status: CybersourcePaymentStatus,
}

impl TryFrom<types::PaymentsResponseRouterData<CybersourcePaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<CybersourcePaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(types::RouterData {
            status: item.response.status.into(),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct CybersourceRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for CybersourceRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    Processing,
}

// Default should be Processing
impl Default for RefundStatus {
    fn default() -> Self {
        RefundStatus::Processing
    }
}

impl From<self::RefundStatus> for enums::RefundStatus {
    fn from(item: self::RefundStatus) -> Self {
        match item {
            self::RefundStatus::Succeeded => enums::RefundStatus::Success,
            self::RefundStatus::Failed => enums::RefundStatus::Failure,
            self::RefundStatus::Processing => enums::RefundStatus::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CybersourceRefundResponse {}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, CybersourceRefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::RSync, CybersourceRefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CybersourceErrorResponse {}
