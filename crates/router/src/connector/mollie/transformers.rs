use api_models::payments;
use error_stack::IntoReport;
use masking::Secret;
use serde::{Deserialize, Serialize};
use storage_models::enums;
use url::Url;

use crate::{
    connector::utils::{self, AddressDetailsData, RouterData},
    core::errors,
    services, types,
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MolliePaymentsRequest {
    amount: Amount,
    description: String,
    redirect_url: String,
    cancel_url: Option<String>,
    webhook_url: String,
    locale: Option<String>,
    #[serde(flatten)]
    payment_method_data: PaymentMethodData,
    metadata: Option<serde_json::Value>,
    sequence_type: SequenceType,
    mandate_id: Option<String>,
    card_token: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Amount {
    currency: enums::Currency,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodData {
    Applepay(Box<ApplePayMethodData>),
    Eps,
    Giropay,
    Ideal(Box<IdealMethodData>),
    Paypal(Box<PaypalMethodData>),
    Sofort,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayMethodData {
    apple_pay_payment_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdealMethodData {
    issuer: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaypalMethodData {
    billing_address: Option<Address>,
    shipping_address: Option<Address>,
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
    type Error = Error;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.request.currency,
            value: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
        };
        let description = item.get_description()?;
        let redirect_url = item.get_return_url()?;
        let payment_method_data = match item.request.capture_method.unwrap_or_default() {
            enums::CaptureMethod::Automatic => match item.request.payment_method_data {
                api_models::payments::PaymentMethodData::BankRedirect(ref redirect_data) => {
                    get_payment_method_for_bank_redirect(item, redirect_data)
                }
                api_models::payments::PaymentMethodData::Wallet(ref wallet_data) => {
                    get_payment_method_for_wallet(item, wallet_data)
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    "Payment Method".to_string(),
                ))
                .into_report(),
            },
            _ => Err(errors::ConnectorError::FlowNotSupported {
                flow: format!(
                    "{} capture",
                    item.request.capture_method.unwrap_or_default()
                ),
                connector: "Mollie".to_string(),
            })
            .into_report(),
        }?;
        Ok(Self {
            amount,
            description,
            redirect_url,
            cancel_url: None,
            /* webhook_url is a mandatory field.
            But we can't support webhook in our core hence keeping it as empty string */
            webhook_url: "".to_string(),
            locale: None,
            payment_method_data,
            metadata: None,
            sequence_type: SequenceType::Oneoff,
            mandate_id: None,
            card_token: None,
        })
    }
}

fn get_payment_method_for_bank_redirect(
    _item: &types::PaymentsAuthorizeRouterData,
    redirect_data: &api_models::payments::BankRedirectData,
) -> Result<PaymentMethodData, Error> {
    let payment_method_data = match redirect_data {
        api_models::payments::BankRedirectData::Eps { .. } => PaymentMethodData::Eps,
        api_models::payments::BankRedirectData::Giropay { .. } => PaymentMethodData::Giropay,
        api_models::payments::BankRedirectData::Ideal { .. } => {
            PaymentMethodData::Ideal(Box::new(IdealMethodData {
                // To do if possible this should be from the payment request
                issuer: None,
            }))
        }
        api_models::payments::BankRedirectData::Sofort { .. } => PaymentMethodData::Sofort,
    };
    Ok(payment_method_data)
}

fn get_payment_method_for_wallet(
    item: &types::PaymentsAuthorizeRouterData,
    wallet_data: &api_models::payments::WalletData,
) -> Result<PaymentMethodData, Error> {
    match wallet_data {
        api_models::payments::WalletData::PaypalRedirect { .. } => {
            Ok(PaymentMethodData::Paypal(Box::new(PaypalMethodData {
                billing_address: get_billing_details(item)?,
                shipping_address: get_shipping_details(item)?,
            })))
        }
        api_models::payments::WalletData::ApplePay(applepay_wallet_data) => {
            Ok(PaymentMethodData::Applepay(Box::new(ApplePayMethodData {
                apple_pay_payment_token: applepay_wallet_data.payment_data.to_owned(),
            })))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment Method".to_string(),
        ))
        .into_report(),
    }
}

fn get_shipping_details(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<Option<Address>, Error> {
    let shipping_address = item
        .address
        .shipping
        .as_ref()
        .and_then(|shipping| shipping.address.as_ref());
    get_address_details(shipping_address)
}

fn get_billing_details(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<Option<Address>, Error> {
    let billing_address = item
        .address
        .billing
        .as_ref()
        .and_then(|billing| billing.address.as_ref());
    get_address_details(billing_address)
}

fn get_address_details(
    address: Option<&payments::AddressDetails>,
) -> Result<Option<Address>, Error> {
    let address_details = match address {
        Some(address) => {
            let street_and_number = address.get_combined_address_line()?;
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
    type Error = Error;
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
    type Error = Error;
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
    type Error = Error;
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
    type Error = Error;
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
