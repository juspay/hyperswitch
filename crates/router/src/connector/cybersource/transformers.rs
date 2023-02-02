use api_models::payments;
use common_utils::pii;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, AddressDetailsData, PaymentsRequestData, PhoneDetailsData},
    consts,
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
#[serde(rename_all = "camelCase")]
pub struct ProcessingInformation {
    capture: bool,
    capture_options: Option<CaptureOptions>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureOptions {
    capture_sequence_number: u32,
    total_capture_count: u32,
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
    address_details: &payments::Address,
    email: Secret<String, pii::Email>,
    phone_number: Secret<String>,
) -> Result<BillTo, error_stack::Report<errors::ConnectorError>> {
    let address = address_details
        .address
        .as_ref()
        .ok_or_else(utils::missing_field_err("billing.address"))?;
    Ok(BillTo {
        first_name: address.get_first_name()?.to_owned(),
        last_name: address.get_last_name()?.to_owned(),
        address1: address.get_line1()?.to_owned(),
        locality: address.get_city()?.to_owned(),
        administrative_area: address.get_line2()?.to_owned(),
        postal_code: address.get_zip()?.to_owned(),
        country: address.get_country()?.to_owned(),
        email,
        phone_number,
    })
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CybersourcePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let phone = item.get_billing_phone()?;
                let phone_number = phone.get_number()?;
                let country_code = phone.get_country_code()?;
                let number_with_code =
                    Secret::new(format!("{}{}", country_code, phone_number.peek()));
                let email = item
                    .request
                    .email
                    .clone()
                    .ok_or_else(utils::missing_field_err("email"))?;
                let bill_to = build_bill_to(item.get_billing()?, email, number_with_code)?;

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
                    capture_options: None,
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

impl TryFrom<&types::PaymentsCaptureRouterData> for CybersourcePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            processing_information: ProcessingInformation {
                capture_options: Some(CaptureOptions {
                    capture_sequence_number: 1,
                    total_capture_count: 1,
                }),
                ..Default::default()
            },
            order_information: OrderInformationWithBill {
                amount_details: Amount {
                    total_amount: value
                        .request
                        .amount_to_capture
                        .map(|amount| amount.to_string())
                        .ok_or_else(utils::missing_field_err("amount_to_capture"))?,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
    }
}

impl TryFrom<&types::RefundExecuteRouterData> for CybersourcePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &types::RefundExecuteRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            order_information: OrderInformationWithBill {
                amount_details: Amount {
                    total_amount: value.request.refund_amount.to_string(),
                    currency: value.request.currency.to_string(),
                },
                ..Default::default()
            },
            ..Default::default()
        })
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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourcePaymentStatus {
    Authorized,
    Succeeded,
    Failed,
    Voided,
    Reversed,
    Pending,
    Declined,
    AuthorizedPendingReview,
    Transmitted,
    #[default]
    Processing,
}

impl From<CybersourcePaymentStatus> for enums::AttemptStatus {
    fn from(item: CybersourcePaymentStatus) -> Self {
        match item {
            CybersourcePaymentStatus::Authorized
            | CybersourcePaymentStatus::AuthorizedPendingReview => Self::Authorized,
            CybersourcePaymentStatus::Succeeded | CybersourcePaymentStatus::Transmitted => {
                Self::Charged
            }
            CybersourcePaymentStatus::Voided | CybersourcePaymentStatus::Reversed => Self::Voided,
            CybersourcePaymentStatus::Failed | CybersourcePaymentStatus::Declined => Self::Failure,
            CybersourcePaymentStatus::Processing => Self::Authorizing,
            CybersourcePaymentStatus::Pending => Self::Pending,
        }
    }
}

impl From<CybersourcePaymentStatus> for enums::RefundStatus {
    fn from(item: CybersourcePaymentStatus) -> Self {
        match item {
            CybersourcePaymentStatus::Succeeded | CybersourcePaymentStatus::Transmitted => {
                Self::Success
            }
            CybersourcePaymentStatus::Failed => Self::Failure,
            _ => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsResponse {
    id: String,
    status: CybersourcePaymentStatus,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct CybersourceErrorInformation {
    reason: String,
    message: String,
}

impl<F, T>
    TryFrom<(
        types::ResponseRouterData<F, CybersourcePaymentsResponse, T, types::PaymentsResponseData>,
        bool,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            types::ResponseRouterData<
                F,
                CybersourcePaymentsResponse,
                T,
                types::PaymentsResponseData,
            >,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let is_capture = data.1;
        Ok(Self {
            status: get_payment_status(is_capture, item.response.status.into()),
            response: match item.response.error_information {
                Some(error) => Err(types::ErrorResponse {
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: error.message,
                    reason: Some(error.reason),
                    status_code: item.http_code,
                }),
                _ => Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                }),
            },
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceTransactionResponse {
    id: String,
    application_information: ApplicationInformation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInformation {
    status: CybersourcePaymentStatus,
}

fn get_payment_status(is_capture: bool, status: enums::AttemptStatus) -> enums::AttemptStatus {
    let is_authorized = matches!(status, enums::AttemptStatus::Authorized);
    if is_capture && is_authorized {
        return enums::AttemptStatus::Pending;
    }
    status
}

impl<F, T>
    TryFrom<(
        types::ResponseRouterData<
            F,
            CybersourceTransactionResponse,
            T,
            types::PaymentsResponseData,
        >,
        bool,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            types::ResponseRouterData<
                F,
                CybersourceTransactionResponse,
                T,
                types::PaymentsResponseData,
            >,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let is_capture = data.1;
        Ok(Self {
            status: get_payment_status(
                is_capture,
                item.response.application_information.status.into(),
            ),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error_information: Option<ErrorInformation>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ErrorInformation {
    pub message: String,
    pub reason: String,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceRefundRequest {
    order_information: OrderInformation,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for CybersourceRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            order_information: OrderInformation {
                amount_details: Amount {
                    total_amount: item.request.refund_amount.to_string(),
                    currency: item.request.currency.to_string(),
                },
            },
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, CybersourcePaymentsResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, CybersourcePaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, CybersourceTransactionResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, CybersourceTransactionResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(
                    item.response.application_information.status,
                ),
            }),
            ..item.data
        })
    }
}
