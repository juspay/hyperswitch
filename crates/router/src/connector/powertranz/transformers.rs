use api_models::payments::{Address, Card};
use common_utils::pii::Email;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use storage_models::enums::RefundStatus;
use uuid::Uuid;

use crate::{
    connector::utils::{self, CardData},
    consts,
    core::errors,
    types::{self, api, storage::enums, transformers::ForeignFrom},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PowertranzPaymentsRequest {
    transaction_identifier: String,
    total_amount: f64,
    currency_code: String,
    three_d_s_ecure: bool,
    source: Source,
    order_identifier: String,
    billing_address: Option<PowertranzAddressDetails>,
    shipping_address: Option<PowertranzAddressDetails>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Source {
    Card(PowertranzCard),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PowertranzCard {
    cardholder_name: Secret<String>,
    card_pan: cards::CardNumber,
    card_expiration: Secret<String>,
    card_cvv: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
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
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment methods".to_string(),
            )),
        }?;
        let billing_address = get_address_details(&item.address.billing, &item.request.email);
        let shipping_address = get_address_details(&item.address.shipping, &item.request.email);
        Ok(Self {
            transaction_identifier: Uuid::new_v4().to_string(),
            total_amount: utils::to_currency_base_unit_asf64(
                item.request.amount,
                item.request.currency,
            )?,
            currency_code: String::foreign_from(item.request.currency),
            three_d_s_ecure: false,
            source,
            order_identifier: item.payment_id.clone(),
            billing_address,
            shipping_address,
        })
    }
}

fn get_address_details(
    address: &Option<Address>,
    email: &Option<Email>,
) -> Option<PowertranzAddressDetails> {
    let phone_number = address
        .as_ref()
        .and_then(|address| address.phone.as_ref())
        .and_then(|phone| {
            phone.number.as_ref().and_then(|number| {
                phone.country_code.as_ref().map(|country_code| {
                    Secret::new(format!("{}{}", country_code, number.clone().expose()))
                })
            })
        });

    address
        .as_ref()
        .and_then(|address| address.address.as_ref())
        .map(|address_details| PowertranzAddressDetails {
            first_name: address_details.first_name.clone(),
            last_name: address_details.last_name.clone(),
            line1: address_details.line1.clone(),
            line2: address_details.line2.clone(),
            city: address_details.city.clone(),
            country: address_details.country,
            state: address_details.state.clone(),
            postal_code: address_details.zip.clone(),
            email_address: email.clone(),
            phone_number,
        })
}

impl ForeignFrom<enums::Currency> for String {
    fn foreign_from(currency: enums::Currency) -> Self {
        let iso_code = match currency {
            storage_models::enums::Currency::AED => "784",
            storage_models::enums::Currency::ALL => "008",
            storage_models::enums::Currency::AMD => "051",
            storage_models::enums::Currency::ANG => "532",
            storage_models::enums::Currency::ARS => "032",
            storage_models::enums::Currency::AUD => "036",
            storage_models::enums::Currency::AWG => "533",
            storage_models::enums::Currency::AZN => "944",
            storage_models::enums::Currency::BBD => "052",
            storage_models::enums::Currency::BDT => "050",
            storage_models::enums::Currency::BHD => "048",
            storage_models::enums::Currency::BMD => "060",
            storage_models::enums::Currency::BND => "096",
            storage_models::enums::Currency::BOB => "068",
            storage_models::enums::Currency::BRL => "986",
            storage_models::enums::Currency::BSD => "044",
            storage_models::enums::Currency::BWP => "072",
            storage_models::enums::Currency::BZD => "084",
            storage_models::enums::Currency::CAD => "124",
            storage_models::enums::Currency::CHF => "756",
            storage_models::enums::Currency::COP => "170",
            storage_models::enums::Currency::CRC => "188",
            storage_models::enums::Currency::CUP => "192",
            storage_models::enums::Currency::CZK => "203",
            storage_models::enums::Currency::DKK => "208",
            storage_models::enums::Currency::DOP => "214",
            storage_models::enums::Currency::DZD => "012",
            storage_models::enums::Currency::EGP => "818",
            storage_models::enums::Currency::ETB => "230",
            storage_models::enums::Currency::EUR => "978",
            storage_models::enums::Currency::FJD => "242",
            storage_models::enums::Currency::GBP => "826",
            storage_models::enums::Currency::GHS => "936",
            storage_models::enums::Currency::GIP => "292",
            storage_models::enums::Currency::GMD => "270",
            storage_models::enums::Currency::GTQ => "320",
            storage_models::enums::Currency::GYD => "328",
            storage_models::enums::Currency::HKD => "344",
            storage_models::enums::Currency::HNL => "340",
            storage_models::enums::Currency::HTG => "332",
            storage_models::enums::Currency::HUF => "348",
            storage_models::enums::Currency::HRK => "191",
            storage_models::enums::Currency::IDR => "360",
            storage_models::enums::Currency::ILS => "376",
            storage_models::enums::Currency::INR => "356",
            storage_models::enums::Currency::JMD => "388",
            storage_models::enums::Currency::JOD => "400",
            storage_models::enums::Currency::JPY => "392",
            storage_models::enums::Currency::KES => "404",
            storage_models::enums::Currency::KGS => "417",
            storage_models::enums::Currency::KHR => "116",
            storage_models::enums::Currency::KRW => "410",
            storage_models::enums::Currency::KWD => "414",
            storage_models::enums::Currency::KYD => "136",
            storage_models::enums::Currency::KZT => "398",
            storage_models::enums::Currency::LAK => "418",
            storage_models::enums::Currency::LBP => "422",
            storage_models::enums::Currency::LKR => "144",
            storage_models::enums::Currency::LRD => "430",
            storage_models::enums::Currency::LSL => "426",
            storage_models::enums::Currency::MAD => "504",
            storage_models::enums::Currency::MDL => "498",
            storage_models::enums::Currency::MKD => "807",
            storage_models::enums::Currency::MMK => "104",
            storage_models::enums::Currency::MNT => "496",
            storage_models::enums::Currency::MOP => "446",
            storage_models::enums::Currency::MUR => "480",
            storage_models::enums::Currency::MVR => "462",
            storage_models::enums::Currency::MWK => "454",
            storage_models::enums::Currency::MXN => "484",
            storage_models::enums::Currency::MYR => "458",
            storage_models::enums::Currency::NAD => "516",
            storage_models::enums::Currency::NGN => "566",
            storage_models::enums::Currency::NIO => "558",
            storage_models::enums::Currency::NOK => "578",
            storage_models::enums::Currency::NPR => "524",
            storage_models::enums::Currency::NZD => "554",
            storage_models::enums::Currency::OMR => "512",
            storage_models::enums::Currency::PEN => "604",
            storage_models::enums::Currency::PGK => "598",
            storage_models::enums::Currency::PHP => "608",
            storage_models::enums::Currency::PKR => "586",
            storage_models::enums::Currency::PLN => "985",
            storage_models::enums::Currency::QAR => "634",
            storage_models::enums::Currency::RON => "946",
            storage_models::enums::Currency::CNY => "156",
            storage_models::enums::Currency::RUB => "643",
            storage_models::enums::Currency::SAR => "682",
            storage_models::enums::Currency::SCR => "690",
            storage_models::enums::Currency::SEK => "752",
            storage_models::enums::Currency::SGD => "702",
            storage_models::enums::Currency::SLL => "694",
            storage_models::enums::Currency::SOS => "706",
            storage_models::enums::Currency::SSP => "728",
            storage_models::enums::Currency::SVC => "222",
            storage_models::enums::Currency::SZL => "748",
            storage_models::enums::Currency::THB => "764",
            storage_models::enums::Currency::TRY => "949",
            storage_models::enums::Currency::TTD => "780",
            storage_models::enums::Currency::TWD => "901",
            storage_models::enums::Currency::TZS => "834",
            storage_models::enums::Currency::USD => "840",
            storage_models::enums::Currency::UYU => "858",
            storage_models::enums::Currency::UZS => "860",
            storage_models::enums::Currency::YER => "886",
            storage_models::enums::Currency::ZAR => "710",
        };
        iso_code.to_string()
    }
}

impl From<&Card> for Source {
    fn from(card: &Card) -> Self {
        let card = PowertranzCard {
            cardholder_name: card.card_holder_name.clone(),
            card_pan: card.card_number.clone(),
            card_expiration: card.get_expiry_date_as_yymm(),
            card_cvv: card.card_cvc.clone(),
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
#[serde(rename_all = "PascalCase")]
pub struct PowertranzBaseResponse {
    transaction_type: u8,
    approved: bool,
    transaction_identifier: String,
    original_trxn_identifier: Option<String>,
    errors: Option<Vec<Error>>,
}

impl ForeignFrom<(u8, bool)> for enums::AttemptStatus {
    fn foreign_from((transaction_type, approved): (u8, bool)) -> Self {
        match transaction_type {
            // Auth
            1 => match approved {
                true => Self::Authorized,
                false => Self::Failure,
            },
            // Sale or Capture
            2 | 3 => match approved {
                true => Self::Charged,
                false => Self::Failure,
            },
            // Void
            4 => match approved {
                true => Self::Voided,
                false => Self::VoidFailed,
            },
            // Refund
            5 => match approved {
                true => Self::AutoRefunded,
                false => Self::Failure,
            },
            // Risk Management
            _ => match approved {
                true => Self::Pending,
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
        let error_response = build_error_response(&item.response, item.http_code);
        // original_trxn_identifier will be present only in capture and void
        let connector_transaction_id = item
            .response
            .original_trxn_identifier
            .unwrap_or(item.response.transaction_identifier);
        let response = error_response.map_or(
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(connector_transaction_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            Err,
        );
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.transaction_type,
                item.response.approved,
            )),
            response,
            ..item.data
        })
    }
}

// Type definition for Capture, Void, Refund Request
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PowertranzBaseRequest {
    transaction_identifier: String,
    total_amount: Option<f64>,
    refund: Option<bool>,
}

impl TryFrom<&types::PaymentsCancelData> for PowertranzBaseRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelData) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_identifier: item.connector_transaction_id.clone(),
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
            transaction_identifier: item.connector_transaction_id.clone(),
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
            transaction_identifier: item.request.connector_transaction_id.clone(),
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
        let error_response = build_error_response(&item.response, item.http_code);
        let response = error_response.map_or(
            Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_identifier.to_string(),
                refund_status: match item.response.approved {
                    true => RefundStatus::Success,
                    false => RefundStatus::Failure,
                },
            }),
            Err,
        );
        Ok(Self {
            response,
            ..item.data
        })
    }
}

fn build_error_response(
    item: &PowertranzBaseResponse,
    status_code: u16,
) -> Option<types::ErrorResponse> {
    item.errors.as_ref().map(|errors| {
        let first_error = errors.first();
        let code = first_error.map(|error| error.code.clone());
        let message = first_error.map(|error| error.message.clone());

        types::ErrorResponse {
            status_code,
            code: code.unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: message
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: message,
        }
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PowertranzErrorResponse {
    pub errors: Vec<Error>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Error {
    pub code: String,
    pub message: String,
}
