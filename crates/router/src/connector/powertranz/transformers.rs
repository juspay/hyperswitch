use api_models::payments::Card;
use common_utils::pii::Email;
use diesel_models::enums::RefundStatus;
use error_stack::IntoReport;
use masking::Secret;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    connector::utils::{self, CardData, PaymentsAuthorizeRequestData},
    consts,
    core::errors,
    services,
    types::{self, api, storage::enums, transformers::ForeignFrom},
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
    i_p: Option<std::net::IpAddr>,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RedirectResponsePayload {
    pub spi_token: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PowertranzPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let source = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(card) => Ok(Source::from(&card)),
            api::PaymentMethodData::Wallet(_)
            | api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_) => Err(errors::ConnectorError::NotSupported {
                message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "powertranz",
            })
            .into_report(),
        }?;
        // let billing_address = get_address_details(&item.address.billing, &item.request.email);
        // let shipping_address = get_address_details(&item.address.shipping, &item.request.email);
        let (three_d_secure, extended_data) = match item.auth_type {
            diesel_models::enums::AuthenticationType::ThreeDs => {
                (true, Some(ExtendedData::try_from(item)?))
            }
            diesel_models::enums::AuthenticationType::NoThreeDs => (false, None),
        };
        Ok(Self {
            transaction_identifier: Uuid::new_v4().to_string(),
            total_amount: utils::to_currency_base_unit_asf64(
                item.request.amount,
                item.request.currency,
            )?,
            currency_code: diesel_models::enums::Currency::iso_4217(&item.request.currency)
                .to_string(),
            three_d_secure,
            source,
            order_identifier: item.connector_request_reference_id.clone(),
            // billing_address,
            // shipping_address,
            extended_data,
        })
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for ExtendedData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            three_d_secure: ThreeDSecure {
                /// Merchants preferred sized of challenge window presented to cardholder.
                /// 5 maps to 100% of challenge window size
                challenge_window_size: 5,
            },
            merchant_response_url: item.request.get_complete_authorize_url()?,
            browser_info: BrowserInfo::try_from(&item.request.get_browser_info()?)?,
        })
    }
}

impl TryFrom<&types::BrowserInformation> for BrowserInfo {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::BrowserInformation) -> Result<Self, Self::Error> {
        Ok(Self {
            java_enabled: item.java_enabled,
            javascript_enabled: item.java_script_enabled,
            accept_header: item.accept_header.clone(),
            language: item.language.clone(),
            screen_height: item.screen_height.map(|height| height.to_string()),
            screen_width: item.screen_width.map(|width| width.to_string()),
            time_zone: item.time_zone.map(|zone| zone.to_string()),
            user_agent: item.user_agent.clone(),
            i_p: item.ip_address,
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
                power_tranz_id: key1.to_owned(),
                power_tranz_password: api_key.to_owned(),
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
    iso_response_code: String,
    redirect_data: Option<String>,
    response_message: String,
    order_identifier: String,
}

impl ForeignFrom<(u8, bool, bool)> for enums::AttemptStatus {
    fn foreign_from((transaction_type, approved, is_3ds): (u8, bool, bool)) -> Self {
        match transaction_type {
            // Auth
            1 => match approved {
                true => Self::Authorized,
                false => match is_3ds {
                    true => Self::AuthenticationPending,
                    false => Self::Failure,
                },
            },
            // Sale
            2 => match approved {
                true => Self::Charged,
                false => match is_3ds {
                    true => Self::AuthenticationPending,
                    false => Self::Failure,
                },
            },
            // Capture
            3 => match approved {
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
            .unwrap_or(item.response.transaction_identifier.clone());
        let redirection_data =
            item.response
                .redirect_data
                .map(|redirect_data| services::RedirectForm::Html {
                    html_data: redirect_data,
                });
        let response = error_response.map_or(
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(connector_transaction_id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_identifier),
            }),
            Err,
        );
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
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
    // errors object has highest precedence to get error message and code
    let error_response = if item.errors.is_some() {
        item.errors.as_ref().map(|errors| {
            let first_error = errors.first();
            let code = first_error.map(|error| error.code.clone());
            let message = first_error.map(|error| error.message.clone());

            types::ErrorResponse {
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
            }
        })
    } else if !ISO_SUCCESS_CODES.contains(&item.iso_response_code.as_str()) {
        // Incase error object is not present the error message and code should be propagated based on iso_response_code
        Some(types::ErrorResponse {
            status_code,
            code: item.iso_response_code.clone(),
            message: item.response_message.clone(),
            reason: Some(item.response_message.clone()),
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Error {
    pub code: String,
    pub message: String,
}
