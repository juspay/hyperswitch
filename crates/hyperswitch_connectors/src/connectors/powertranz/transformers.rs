use common_enums::enums::{self, AuthenticationType};
use common_utils::pii::IpAddress;
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::Execute,
    router_request_types::{
        BrowserInformation, PaymentsCancelData, PaymentsCaptureData, ResponseId,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{consts, errors};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, CardData, PaymentsAuthorizeRequestData, RouterData as _},
};

const ISO_SUCCESS_CODES: [&str; 7] = ["00", "3D0", "3D1", "HP0", "TK0", "SP4", "FC0"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PowertranzPaymentsRequest {
    transaction_identifier: String,
    total_amount: f64,
    currency_code: String,
    three_d_secure: bool,
    source: Source,
    order_identifier: String,
    // billing and shipping are optional fields and requires state in iso codes, hence commenting it
    // can be added later if we have iso code for state
    // billing_address: Option<PowertranzAddressDetails>,
    // shipping_address: Option<PowertranzAddressDetails>,
    extended_data: Option<ExtendedData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExtendedData {
    three_d_secure: ThreeDSecure,
    merchant_response_url: String,
    browser_info: BrowserInfo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BrowserInfo {
    java_enabled: Option<bool>,
    javascript_enabled: Option<bool>,
    accept_header: Option<String>,
    language: Option<String>,
    screen_height: Option<String>,
    screen_width: Option<String>,
    time_zone: Option<String>,
    user_agent: Option<String>,
    i_p: Option<Secret<String, IpAddress>>,
    color_depth: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ThreeDSecure {
    challenge_window_size: u8,
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

// #[derive(Debug, Serialize)]
// #[serde(rename_all = "PascalCase")]
// pub struct PowertranzAddressDetails {
//     first_name: Option<Secret<String>>,
//     last_name: Option<Secret<String>>,
//     line1: Option<Secret<String>>,
//     line2: Option<Secret<String>>,
//     city: Option<String>,
//     country: Option<enums::CountryAlpha2>,
//     state: Option<Secret<String>>,
//     postal_code: Option<Secret<String>>,
//     email_address: Option<Email>,
//     phone_number: Option<Secret<String>>,
// }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RedirectResponsePayload {
    pub spi_token: Secret<String>,
}

impl TryFrom<&PaymentsAuthorizeRouterData> for PowertranzPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let source = match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(card) => {
                let card_holder_name = item.get_optional_billing_full_name();
                Source::try_from((&card, card_holder_name))
            }
            PaymentMethodData::Wallet(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "powertranz",
                }
                .into())
            }
        }?;
        // let billing_address = get_address_details(&item.address.billing, &item.request.email);
        // let shipping_address = get_address_details(&item.address.shipping, &item.request.email);
        let (three_d_secure, extended_data) = match item.auth_type {
            AuthenticationType::ThreeDs => (true, Some(ExtendedData::try_from(item)?)),
            AuthenticationType::NoThreeDs => (false, None),
        };
        Ok(Self {
            transaction_identifier: Uuid::new_v4().to_string(),
            total_amount: utils::to_currency_base_unit_asf64(
                item.request.amount,
                item.request.currency,
            )?,
            currency_code: item.request.currency.iso_4217().to_string(),
            three_d_secure,
            source,
            order_identifier: item.connector_request_reference_id.clone(),
            // billing_address,
            // shipping_address,
            extended_data,
        })
    }
}

impl TryFrom<&PaymentsAuthorizeRouterData> for ExtendedData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            three_d_secure: ThreeDSecure {
                // Merchants preferred sized of challenge window presented to cardholder.
                // 5 maps to 100% of challenge window size
                challenge_window_size: 5,
            },
            merchant_response_url: item.request.get_complete_authorize_url()?,
            browser_info: BrowserInfo::try_from(&item.request.get_browser_info()?)?,
        })
    }
}

impl TryFrom<&BrowserInformation> for BrowserInfo {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &BrowserInformation) -> Result<Self, Self::Error> {
        Ok(Self {
            java_enabled: item.java_enabled,
            javascript_enabled: item.java_script_enabled,
            accept_header: item.accept_header.clone(),
            language: item.language.clone(),
            screen_height: item.screen_height.map(|height| height.to_string()),
            screen_width: item.screen_width.map(|width| width.to_string()),
            time_zone: item.time_zone.map(|zone| zone.to_string()),
            user_agent: item.user_agent.clone(),
            i_p: item
                .ip_address
                .map(|ip_address| Secret::new(ip_address.to_string())),
            color_depth: item.color_depth.map(|depth| depth.to_string()),
        })
    }
}

/*fn get_address_details(
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
}*/

impl TryFrom<(&Card, Option<Secret<String>>)> for Source {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (card, card_holder_name): (&Card, Option<Secret<String>>),
    ) -> Result<Self, Self::Error> {
        let card = PowertranzCard {
            cardholder_name: card_holder_name.unwrap_or(Secret::new("".to_string())),
            card_pan: card.card_number.clone(),
            card_expiration: card.get_expiry_date_as_yymm()?,
            card_cvv: card.card_cvc.clone(),
        };
        Ok(Self::Card(card))
    }
}

// Auth Struct
pub struct PowertranzAuthType {
    pub(super) power_tranz_id: Secret<String>,
    pub(super) power_tranz_password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PowertranzAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                power_tranz_id: key1.to_owned(),
                power_tranz_password: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// Common struct used in Payment, Capture, Void, Refund
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PowertranzBaseResponse {
    transaction_type: u8,
    approved: bool,
    transaction_identifier: String,
    original_trxn_identifier: Option<String>,
    errors: Option<Vec<Error>>,
    iso_response_code: String,
    redirect_data: Option<Secret<String>>,
    response_message: String,
    order_identifier: String,
}

fn get_status((transaction_type, approved, is_3ds): (u8, bool, bool)) -> enums::AttemptStatus {
    match transaction_type {
        // Auth
        1 => match approved {
            true => enums::AttemptStatus::Authorized,
            false => match is_3ds {
                true => enums::AttemptStatus::AuthenticationPending,
                false => enums::AttemptStatus::Failure,
            },
        },
        // Sale
        2 => match approved {
            true => enums::AttemptStatus::Charged,
            false => match is_3ds {
                true => enums::AttemptStatus::AuthenticationPending,
                false => enums::AttemptStatus::Failure,
            },
        },
        // Capture
        3 => match approved {
            true => enums::AttemptStatus::Charged,
            false => enums::AttemptStatus::Failure,
        },
        // Void
        4 => match approved {
            true => enums::AttemptStatus::Voided,
            false => enums::AttemptStatus::VoidFailed,
        },
        // Refund
        5 => match approved {
            true => enums::AttemptStatus::AutoRefunded,
            false => enums::AttemptStatus::Failure,
        },
        // Risk Management
        _ => match approved {
            true => enums::AttemptStatus::Pending,
            false => enums::AttemptStatus::Failure,
        },
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, PowertranzBaseResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PowertranzBaseResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let error_response = build_error_response(&item.response, item.http_code);
        // original_trxn_identifier will be present only in capture and void
        let connector_transaction_id = item
            .response
            .original_trxn_identifier
            .unwrap_or(item.response.transaction_identifier.clone());
        let redirection_data = item.response.redirect_data.map(|redirect_data| {
            hyperswitch_domain_models::router_response_types::RedirectForm::Html {
                html_data: redirect_data.expose(),
            }
        });
        let response = error_response.map_or(
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(connector_transaction_id),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_identifier),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            Err,
        );
        Ok(Self {
            status: get_status((
                item.response.transaction_type,
                item.response.approved,
                is_3ds_payment(item.response.iso_response_code),
            )),
            response,
            ..item.data
        })
    }
}

fn is_3ds_payment(response_code: String) -> bool {
    matches!(response_code.as_str(), "SP4")
}

// Type definition for Capture, Void, Refund Request
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PowertranzBaseRequest {
    transaction_identifier: String,
    total_amount: Option<f64>,
    refund: Option<bool>,
}

impl TryFrom<&PaymentsCancelData> for PowertranzBaseRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelData) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_identifier: item.connector_transaction_id.clone(),
            total_amount: None,
            refund: None,
        })
    }
}

impl TryFrom<&PaymentsCaptureData> for PowertranzBaseRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCaptureData) -> Result<Self, Self::Error> {
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

impl<F> TryFrom<&RefundsRouterData<F>> for PowertranzBaseRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundsRouterData<F>) -> Result<Self, Self::Error> {
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

impl TryFrom<RefundsResponseRouterData<Execute, PowertranzBaseResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, PowertranzBaseResponse>,
    ) -> Result<Self, Self::Error> {
        let error_response = build_error_response(&item.response, item.http_code);
        let response = error_response.map_or(
            Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_identifier.to_string(),
                refund_status: match item.response.approved {
                    true => enums::RefundStatus::Success,
                    false => enums::RefundStatus::Failure,
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

fn build_error_response(item: &PowertranzBaseResponse, status_code: u16) -> Option<ErrorResponse> {
    // errors object has highest precedence to get error message and code
    let error_response = if item.errors.is_some() {
        item.errors.as_ref().map(|errors| {
            let first_error = errors.first();
            let code = first_error.map(|error| error.code.clone());
            let message = first_error.map(|error| error.message.clone());

            ErrorResponse {
                status_code,
                code: code.unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: message.unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: Some(
                    errors
                        .iter()
                        .map(|error| format!("{} : {}", error.code, error.message))
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
                attempt_status: None,
                connector_transaction_id: None,
            }
        })
    } else if !ISO_SUCCESS_CODES.contains(&item.iso_response_code.as_str()) {
        // Incase error object is not present the error message and code should be propagated based on iso_response_code
        Some(ErrorResponse {
            status_code,
            code: item.iso_response_code.clone(),
            message: item.response_message.clone(),
            reason: Some(item.response_message.clone()),
            attempt_status: None,
            connector_transaction_id: None,
        })
    } else {
        None
    };
    error_response
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PowertranzErrorResponse {
    pub errors: Vec<Error>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Error {
    pub code: String,
    pub message: String,
}
