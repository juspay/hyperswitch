use api_models::payments;
use common_utils::pii;
use masking::Secret;
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
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: String,
    expiration_month: String,
    expiration_year: String,
    security_code: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformationWithBill {
    amount_details: Amount,
    bill_to: BillTo,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformation {
    amount_details: Amount,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    total_amount: String,
    currency: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BillTo {
    first_name: Secret<String>,
    last_name: Secret<String>,
    address1: Secret<String>,
    locality: String,
    administrative_area: Secret<String>,
    postal_code: Secret<String>,
    country: String,
    email: Secret<String, pii::Email>,
    phone_number: Secret<String>,
}

// for cybersource each item in Billing is mandatory
fn build_bill_to(
    address_details: payments::Address,
    email: Secret<String, pii::Email>,
    phone_number: Secret<String>,
) -> Option<BillTo> {
    if let Some(api_models::payments::AddressDetails {
        first_name: Some(f_name),
        last_name: Some(last_name),
        line1: Some(address1),
        city: Some(city),
        line2: Some(administrative_area),
        zip: Some(postal_code),
        country: Some(country),
        ..
    }) = address_details.address
    {
        Some(BillTo {
            first_name: f_name,
            last_name,
            address1,
            locality: city,
            administrative_area,
            postal_code,
            country,
            email,
            phone_number,
        })
    } else {
        None
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CybersourcePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let address = item
                    .address
                    .billing
                    .clone()
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
                let phone = address
                    .clone()
                    .phone
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
                let phone_number = phone
                    .number
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
                let country_code = phone
                    .country_code
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
                let number_with_code =
                    Secret::new(format!("{}{}", country_code, phone_number.peek()));
                let email = item
                    .request
                    .email
                    .clone()
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
                let bill_to = build_bill_to(address, email, number_with_code)
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?;

                let order_information = OrderInformationWithBill {
                    amount_details: Amount {
                        total_amount: item.request.amount.to_string(),
                        currency: item.request.currency.to_string().to_uppercase(),
                    },
                    bill_to,
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

                Ok(Self {
                    processing_information,
                    payment_information,
                    order_information,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
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
#[derive(Debug, Default, Clone, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CybersourcePaymentStatus {
    Authorized,
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<CybersourcePaymentStatus> for enums::AttemptStatus {
    fn from(item: CybersourcePaymentStatus) -> Self {
        match item {
            CybersourcePaymentStatus::Authorized => Self::Authorized,
            CybersourcePaymentStatus::Succeeded => Self::Charged,
            CybersourcePaymentStatus::Failed => Self::Failure,
            CybersourcePaymentStatus::Processing => Self::Authorizing,
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
        Ok(Self {
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
    pub message: Option<String>,
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
        Ok(Self {
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
#[derive(Debug, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            self::RefundStatus::Succeeded => Self::Success,
            self::RefundStatus::Failed => Self::Failure,
            self::RefundStatus::Processing => Self::Pending,
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
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}
