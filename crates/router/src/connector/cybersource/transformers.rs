use common_utils::pii::Email;
use masking::{ExposeOptionInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    pii::PeekInterface,
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ProcessingInformation {
    capture: bool,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PaymentInformation {
    card: Card,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct Card {
    number: String,
    expiration_month: String,
    expiration_year: String,
    security_code: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct OrderInformationWithBill {
    amount_details: Amount,
    bill_to: BillTo,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct OrderInformation {
    amount_details: Amount,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct Amount {
    total_amount: String,
    currency: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BillTo {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address1: Option<Secret<String>>,
    locality: Option<String>,
    administrative_area: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    country: Option<String>,
    email: Option<Secret<String, Email>>,
    phone_number: Option<Secret<String>>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CybersourcePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let order_information = OrderInformationWithBill {
                    amount_details: Amount {
                        total_amount: item.request.amount.to_string(),
                        currency: item.request.currency.to_string().to_uppercase(),
                    },
                    bill_to: match item
                        .address
                        .billing
                        .clone()
                        .map(|f| (f.address.unwrap_or_default(), f.phone))
                    {
                        Some((address, phone)) => BillTo {
                            first_name: address.first_name,
                            last_name: address.last_name,
                            address1: address.line1,
                            locality: address.city,
                            administrative_area: address.line2,
                            postal_code: address.zip,
                            country: address.country,
                            email: item.request.email.clone(),
                            phone_number: phone.map(|p| {
                                format!(
                                    "{}{}",
                                    p.country_code.unwrap_or_default(),
                                    p.number.expose_option().unwrap_or_default()
                                )
                                .into()
                            }),
                        },
                        None => BillTo::default(),
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
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
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
#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
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

#[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq)]
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
                mandate_reference: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error_information: Option<ErrorInformation>,
    pub status: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct ErrorInformation {
    pub message: String,
    pub reason: String,
}

#[derive(Default, Debug, Serialize)]
pub struct CybersourceRefundRequest {
    order_information: OrderInformation,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for CybersourceRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(CybersourceRefundRequest {
            order_information: OrderInformation {
                amount_details: Amount {
                    total_amount: item.request.amount.to_string(),
                    currency: item.request.currency.to_string(),
                },
            },
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    Processing,
}

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
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct CybersourceRefundResponse {
    pub id: String,
    pub status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, CybersourceRefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, CybersourceRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(types::RouterData {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}
