use api_models::payments::{Address, Card};
use common_utils::pii::Email;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use storage_models::enums::RefundStatus;

use crate::{
    connector::utils::{self, CardData},
    core::errors,
    types::{
        self, api,
        storage::enums,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
};

#[derive(Debug, Serialize)]
pub struct PowertranzPaymentsRequest {
    transaction_identifier: String,
    total_amount: f64,
    currency_code: String,
    three_d_s_ecure: bool,
    source: Source,
    order_identifier: String,
    billing_address: Option<PowertranzAddressDetails>,
    shiping_address: Option<PowertranzAddressDetails>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Source {
    Card(PowertranzCard),
}

#[derive(Debug, Serialize)]
pub struct PowertranzCard {
    cardholder_name: Secret<String>,
    card_pan: cards::CardNumber,
    card_expiration: Secret<String>,
    card_cvv: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct PowertranzAddressDetails {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    line1: Option<Secret<String>>,
    line2: Option<Secret<String>>,
    city: Option<String>,
    country: Option<enums::CountryAlpha2>,
    state: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    email_address: Option<Email>,
    phone_number: Option<Secret<String>>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PowertranzPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let source = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(card) => Ok(Source::from(&card)),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }?;
        let billing_address = Some(PowertranzAddressDetails::try_from((
            item.address.billing,
            item.request.email,
        ))?);
        let shiping_address = Some(PowertranzAddressDetails::try_from((
            item.address.shipping,
            item.request.email,
        ))?);
        Ok(Self {
            transaction_identifier: item.attempt_id,
            total_amount: utils::to_currency_base_unit_asf64(
                item.request.amount,
                item.request.currency,
            )?,
            currency_code: String::try_from(item.request.currency),
            three_d_s_ecure: false,
            source,
            order_identifier: item.payment_id,
            billing_address,
            shiping_address,
        })
    }
}

impl TryFrom<enums::Currency> for String {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(currency: enums::Currency) -> Result<Self, Self::Error> {
        match currency {
            storage_models::enums::Currency::AED => 784,
            storage_models::enums::Currency::ALL => 008,
            storage_models::enums::Currency::AMD => todo!(),
            storage_models::enums::Currency::ANG => todo!(),
            storage_models::enums::Currency::ARS => todo!(),
            storage_models::enums::Currency::AUD => todo!(),
            storage_models::enums::Currency::AWG => todo!(),
            storage_models::enums::Currency::AZN => todo!(),
            storage_models::enums::Currency::BBD => todo!(),
            storage_models::enums::Currency::BDT => todo!(),
            storage_models::enums::Currency::BHD => todo!(),
            storage_models::enums::Currency::BMD => todo!(),
            storage_models::enums::Currency::BND => todo!(),
            storage_models::enums::Currency::BOB => todo!(),
            storage_models::enums::Currency::BRL => todo!(),
            storage_models::enums::Currency::BSD => todo!(),
            storage_models::enums::Currency::BWP => todo!(),
            storage_models::enums::Currency::BZD => todo!(),
            storage_models::enums::Currency::CAD => todo!(),
            storage_models::enums::Currency::CHF => todo!(),
            storage_models::enums::Currency::CNY => todo!(),
            storage_models::enums::Currency::COP => todo!(),
            storage_models::enums::Currency::CRC => todo!(),
            storage_models::enums::Currency::CUP => todo!(),
            storage_models::enums::Currency::CZK => todo!(),
            storage_models::enums::Currency::DKK => todo!(),
            storage_models::enums::Currency::DOP => todo!(),
            storage_models::enums::Currency::DZD => todo!(),
            storage_models::enums::Currency::EGP => todo!(),
            storage_models::enums::Currency::ETB => todo!(),
            storage_models::enums::Currency::EUR => todo!(),
            storage_models::enums::Currency::FJD => todo!(),
            storage_models::enums::Currency::GBP => todo!(),
            storage_models::enums::Currency::GHS => todo!(),
            storage_models::enums::Currency::GIP => todo!(),
            storage_models::enums::Currency::GMD => todo!(),
            storage_models::enums::Currency::GTQ => todo!(),
            storage_models::enums::Currency::GYD => todo!(),
            storage_models::enums::Currency::HKD => todo!(),
            storage_models::enums::Currency::HNL => todo!(),
            storage_models::enums::Currency::HRK => todo!(),
            storage_models::enums::Currency::HTG => todo!(),
            storage_models::enums::Currency::HUF => todo!(),
            storage_models::enums::Currency::IDR => todo!(),
            storage_models::enums::Currency::ILS => todo!(),
            storage_models::enums::Currency::INR => todo!(),
            storage_models::enums::Currency::JMD => todo!(),
            storage_models::enums::Currency::JOD => todo!(),
            storage_models::enums::Currency::JPY => todo!(),
            storage_models::enums::Currency::KES => todo!(),
            storage_models::enums::Currency::KGS => todo!(),
            storage_models::enums::Currency::KHR => todo!(),
            storage_models::enums::Currency::KRW => todo!(),
            storage_models::enums::Currency::KWD => todo!(),
            storage_models::enums::Currency::KYD => todo!(),
            storage_models::enums::Currency::KZT => todo!(),
            storage_models::enums::Currency::LAK => todo!(),
            storage_models::enums::Currency::LBP => todo!(),
            storage_models::enums::Currency::LKR => todo!(),
            storage_models::enums::Currency::LRD => todo!(),
            storage_models::enums::Currency::LSL => todo!(),
            storage_models::enums::Currency::MAD => todo!(),
            storage_models::enums::Currency::MDL => todo!(),
            storage_models::enums::Currency::MKD => todo!(),
            storage_models::enums::Currency::MMK => todo!(),
            storage_models::enums::Currency::MNT => todo!(),
            storage_models::enums::Currency::MOP => todo!(),
            storage_models::enums::Currency::MUR => todo!(),
            storage_models::enums::Currency::MVR => todo!(),
            storage_models::enums::Currency::MWK => todo!(),
            storage_models::enums::Currency::MXN => todo!(),
            storage_models::enums::Currency::MYR => todo!(),
            storage_models::enums::Currency::NAD => todo!(),
            storage_models::enums::Currency::NGN => todo!(),
            storage_models::enums::Currency::NIO => todo!(),
            storage_models::enums::Currency::NOK => todo!(),
            storage_models::enums::Currency::NPR => todo!(),
            storage_models::enums::Currency::NZD => todo!(),
            storage_models::enums::Currency::OMR => todo!(),
            storage_models::enums::Currency::PEN => todo!(),
            storage_models::enums::Currency::PGK => todo!(),
            storage_models::enums::Currency::PHP => todo!(),
            storage_models::enums::Currency::PKR => todo!(),
            storage_models::enums::Currency::PLN => todo!(),
            storage_models::enums::Currency::QAR => todo!(),
            storage_models::enums::Currency::RON => todo!(),
            storage_models::enums::Currency::RUB => todo!(),
            storage_models::enums::Currency::SAR => todo!(),
            storage_models::enums::Currency::SCR => todo!(),
            storage_models::enums::Currency::SEK => todo!(),
            storage_models::enums::Currency::SGD => todo!(),
            storage_models::enums::Currency::SLL => todo!(),
            storage_models::enums::Currency::SOS => todo!(),
            storage_models::enums::Currency::SSP => todo!(),
            storage_models::enums::Currency::SVC => todo!(),
            storage_models::enums::Currency::SZL => todo!(),
            storage_models::enums::Currency::THB => todo!(),
            storage_models::enums::Currency::TRY => todo!(),
            storage_models::enums::Currency::TTD => todo!(),
            storage_models::enums::Currency::TWD => todo!(),
            storage_models::enums::Currency::TZS => todo!(),
            storage_models::enums::Currency::USD => todo!(),
            storage_models::enums::Currency::UYU => todo!(),
            storage_models::enums::Currency::UZS => todo!(),
            storage_models::enums::Currency::YER => todo!(),
            storage_models::enums::Currency::ZAR => todo!(),
        }
    }
}

impl ForeignTryFrom<(Option<Address>, Option<Email>)> for Option<PowertranzAddressDetails> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (address, email): (Option<Address>, Option<Email>),
    ) -> Result<Self, Self::Error> {
        let phone_number = address.and_then(|address| address.phone).and_then(|phone| {
            phone.number.and_then(|number| {
                phone
                    .country_code
                    .map(|country_code| Secret::new(format!("{}{}", country_code, number.expose())))
            })
        });

        Ok(address
            .and_then(|address| address.address)
            .map(|address_details| PowertranzAddressDetails {
                first_name: address_details.first_name,
                last_name: address_details.last_name,
                line1: address_details.line1,
                line2: address_details.line2,
                city: address_details.city,
                country: address_details.country,
                state: address_details.state,
                postal_code: address_details.zip,
                email_address: email,
                phone_number,
            }))
    }
}

impl From<&Card> for Source {
    fn from(card: &Card) -> Self {
        let card = PowertranzCard {
            cardholder_name: card.card_holder_name,
            card_pan: card.card_number,
            card_expiration: card.get_card_expiry_month_year_2_digit_with_delimiter("".to_string()),
            card_cvv: card.card_cvc,
        };
        Self::Card(card)
    }
}

// Auth Struct
pub struct PowertranzAuthType {
    pub(super) power_tranz_id: Secret<String>,
    pub(super) power_tranz_password: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for PowertranzAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                power_tranz_id: Secret::new(key1.to_string()),
                power_tranz_password: Secret::new(api_key.to_string()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// Common struct used in Payment, Capture, Void, Refund
#[derive(Debug, Deserialize)]
pub struct PowertranzBaseResponse {
    transaction_type: TransactionType,
    approved: bool,
    transaction_identifier: String,
}

#[derive(Debug, Deserialize)]
pub enum TransactionType {
    #[serde(rename = "1")]
    Auth,
    #[serde(rename = "2")]
    Sale,
    #[serde(rename = "3")]
    Capture,
    #[serde(rename = "4")]
    Void,
    #[serde(rename = "5")]
    Refund,
}

impl ForeignFrom<(TransactionType, bool)> for enums::AttemptStatus {
    fn foreign_from((transaction_type, approved): (TransactionType, bool)) -> Self {
        match transaction_type {
            TransactionType::Auth => match approved {
                true => Self::Authorized,
                false => Self::Failure,
            },
            TransactionType::Sale | TransactionType::Capture => match approved {
                true => Self::Charged,
                false => Self::Failure,
            },
            TransactionType::Void => match approved {
                true => Self::Voided,
                false => Self::VoidFailed,
            },
            TransactionType::Refund => match approved {
                true => Self::AutoRefunded,
                false => Self::Failure,
            },
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PowertranzBaseResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PowertranzBaseResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.transaction_type,
                item.response.approved,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_identifier,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

// Type definition for Capture, Void, Refund Request
#[derive(Default, Debug, Serialize)]
pub struct PowertranzBaseRequest {
    transaction_identifier: String,
    total_amount: Option<f64>,
    refund: Option<bool>,
}

impl TryFrom<&types::PaymentsCancelData> for PowertranzBaseRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelData) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_identifier: item.connector_transaction_id,
            total_amount: None,
            refund: None,
        })
    }
}

impl TryFrom<&types::PaymentsCaptureData> for PowertranzBaseRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureData) -> Result<Self, Self::Error> {
        let total_amount = Some(utils::to_currency_base_unit_asf64(
            item.amount_to_capture,
            item.currency,
        )?);
        Ok(Self {
            transaction_identifier: item.connector_transaction_id,
            total_amount,
            refund: None,
        })
    }
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for PowertranzBaseRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let total_amount = Some(utils::to_currency_base_unit_asf64(
            item.request.refund_amount,
            item.request.currency,
        )?);
        Ok(Self {
            transaction_identifier: item.request.refund_id,
            total_amount,
            refund: Some(true),
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, PowertranzBaseResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, PowertranzBaseResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_identifier.to_string(),
                refund_status: match item.response.approved {
                    true => RefundStatus::Success,
                    false => RefundStatus::Failure,
                },
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct PowertranzErrorResponse {
    pub errors: Vec<Error>,
}

#[derive(Debug, Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
}
