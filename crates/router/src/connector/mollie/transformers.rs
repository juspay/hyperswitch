use api_models::payments::AddressDetails;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use storage_models::enums::CaptureMethod;
use url::Url;

use crate::{
    connector::utils::{self, AddressDetailsData, RouterData},
    core::errors,
    services,
    types::{self, storage::enums},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MolliePaymentsRequest {
    amount: Amount,
    description: String,
    redirect_url: String,
    cancel_url: Option<String>,
    webhook_url: String,
    locale: Option<String>,
    method: PaymentMethod,
    metadata: Option<serde_json::Value>,
    sequence_type: SequenceType,
    mandate_id: Option<String>,
    billing_address: Option<Address>,
    card_token: Option<String>,
    shipping_address: Option<Address>,
    issuer: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Amount {
    currency: enums::Currency,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethod {
    Eps,
    Ideal,
    Giropay,
    Sofort,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SequenceType {
    #[default]
    Oneoff,
    First,
    Recurring,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub street_and_number: Secret<String>,
    pub postal_code: Secret<String>,
    pub city: String,
    pub region: Option<Secret<String>>,
    pub country: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for MolliePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.request.currency,
            value: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
        };
        let description = item.get_description()?;
        let redirect_url = item.get_return_url()?;
        let method = match item.request.capture_method.unwrap_or_default() {
            CaptureMethod::Automatic => match item.request.payment_method_data {
                api_models::payments::PaymentMethodData::BankRedirect(ref redirect_data) => {
                    let payment_method = match redirect_data {
                        api_models::payments::BankRedirectData::Eps { .. } => PaymentMethod::Eps,
                        api_models::payments::BankRedirectData::Giropay { .. } => {
                            PaymentMethod::Giropay
                        }
                        api_models::payments::BankRedirectData::Ideal { .. } => {
                            PaymentMethod::Ideal
                        }
                        api_models::payments::BankRedirectData::Sofort { .. } => {
                            PaymentMethod::Sofort
                        }
                    };
                    Ok(payment_method)
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    "Payment Method".to_string(),
                )),
            },
            _ => {
                let flow: String = format!(
                    "{} capture",
                    item.request.capture_method.unwrap_or_default()
                );
                Err(errors::ConnectorError::FlowNotSupported {
                    flow,
                    connector: "Mollie".to_string(),
                })
            }
        }?;
        let billing_address = get_billing_details(item)?;
        let shipping_address = get_shipping_details(item)?;
        Ok(Self {
            amount,
            description,
            redirect_url,
            cancel_url: None,
            webhook_url: "".to_string(),
            locale: None,
            method,
            metadata: None,
            sequence_type: SequenceType::Oneoff,
            mandate_id: None,
            shipping_address,
            billing_address,
            card_token: None,
            // To do if possible this should be from the payment request
            issuer: None,
        })
    }
}

fn get_shipping_details(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<Option<Address>, error_stack::Report<errors::ConnectorError>> {
    let shipping_address = item
        .address
        .shipping
        .as_ref()
        .and_then(|shipping| shipping.address.as_ref());
    get_address_details(shipping_address)
}

fn get_billing_details(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<Option<Address>, error_stack::Report<errors::ConnectorError>> {
    let billing_address = item
        .address
        .billing
        .as_ref()
        .and_then(|billing| billing.address.as_ref());
    get_address_details(billing_address)
}

fn get_address_details(
    address: Option<&AddressDetails>,
) -> Result<Option<Address>, error_stack::Report<errors::ConnectorError>> {
    let address_details = match address {
        Some(address) => {
            let street_and_number = Secret::new(format!(
                "{},{}",
                address.get_line1()?.peek(),
                address.get_line2()?.peek()
            ));
            let postal_code = address.get_zip()?.to_owned();
            let city = address.get_city()?.to_owned();
            let region = None;
            let country = address.get_country()?.to_owned();
            Some(Address {
                street_and_number,
                postal_code,
                city,
                region,
                country,
            })
        }
        None => None,
    };
    Ok(address_details)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MolliePaymentsResponse {
    pub resource: String,
    pub id: String,
    pub amount: Amount,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub status: MolliePaymentStatus,
    pub is_cancelable: Option<bool>,
    pub sequence_type: SequenceType,
    pub redirect_url: Option<String>,
    pub webhook_url: Option<String>,
    #[serde(rename = "_links")]
    pub links: Links,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MolliePaymentStatus {
    Open,
    Canceled,
    #[default]
    Pending,
    Authorized,
    Expired,
    Failed,
    Paid,
}

impl From<MolliePaymentStatus> for enums::AttemptStatus {
    fn from(item: MolliePaymentStatus) -> Self {
        match item {
            MolliePaymentStatus::Paid => Self::Charged,
            MolliePaymentStatus::Failed => Self::Failure,
            MolliePaymentStatus::Pending => Self::Pending,
            MolliePaymentStatus::Open => Self::AuthenticationPending,
            MolliePaymentStatus::Canceled => Self::Voided,
            MolliePaymentStatus::Authorized => Self::Authorized,
            MolliePaymentStatus::Expired => Self::Failure,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Link {
    href: Url,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Links {
    #[serde(rename = "self")]
    self_: Option<Link>,
    checkout: Option<Link>,
    dashboard: Option<Link>,
    documentation: Option<Link>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardDetails {
    pub card_number: String,
    pub card_holder: String,
    pub card_expiry_date: String,
    pub card_cvv: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BankDetails {
    billing_email: String,
}

pub struct MollieAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for MollieAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, MolliePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, MolliePaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let url = item
            .response
            .links
            .checkout
            .map(|link| services::RedirectForm::from((link.href, services::Method::Get)));
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: url,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
#[derive(Default, Debug, Serialize)]
pub struct MollieRefundRequest {
    amount: Amount,
    description: Option<String>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for MollieRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.request.currency,
            value: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
        };
        Ok(Self {
            amount,
            description: item.request.reason.to_owned(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    resource: String,
    id: String,
    amount: Amount,
    settlement_id: Option<String>,
    settlement_amount: Option<Amount>,
    status: MollieRefundStatus,
    description: Option<String>,
    metadata: serde_json::Value,
    payment_id: String,
    #[serde(rename = "_links")]
    links: Links,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MollieRefundStatus {
    Queued,
    #[default]
    Pending,
    Processing,
    Refunded,
    Failed,
    Canceled,
}

impl From<MollieRefundStatus> for enums::RefundStatus {
    fn from(item: MollieRefundStatus) -> Self {
        match item {
            MollieRefundStatus::Queued
            | MollieRefundStatus::Pending
            | MollieRefundStatus::Processing => Self::Pending,
            MollieRefundStatus::Refunded => Self::Success,
            MollieRefundStatus::Failed | MollieRefundStatus::Canceled => Self::Failure,
        }
    }
}

impl<T> TryFrom<types::RefundsResponseRouterData<T, RefundResponse>>
    for types::RefundsRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<T, RefundResponse>,
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

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub status: u16,
    pub title: Option<String>,
    pub detail: String,
    pub field: Option<String>,
    #[serde(rename = "_links")]
    pub links: Option<Links>,
}
