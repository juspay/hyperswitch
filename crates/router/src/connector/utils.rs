use base64::Engine;
use bytes::Buf;
use common_utils::pii::Email;
use error_stack::{report, IntoReport, ResultExt};
use masking::Secret;

use crate::{
    consts,
    core::errors::{self, CustomResult},
    pii::PeekInterface,
    types::{self, api, PaymentsCancelData},
    utils::OptionExt,
};

pub fn missing_field_err(
    message: &'static str,
) -> Box<dyn Fn() -> error_stack::Report<errors::ConnectorError> + '_> {
    Box::new(move || {
        errors::ConnectorError::MissingRequiredField {
            field_name: message,
        }
        .into()
    })
}

type Error = error_stack::Report<errors::ConnectorError>;

pub trait AccessTokenRequestInfo {
    fn get_request_id(&self) -> Result<String, Error>;
}

impl AccessTokenRequestInfo for types::RefreshTokenRouterData {
    fn get_request_id(&self) -> Result<String, Error> {
        self.request
            .id
            .clone()
            .ok_or_else(missing_field_err("request.id"))
    }
}

pub trait RouterData {
    fn get_attempt_id(&self) -> Result<String, Error>;
    fn get_billing(&self) -> Result<&api::Address, Error>;
    fn get_billing_country(&self) -> Result<String, Error>;
    fn get_billing_phone(&self) -> Result<&api::PhoneDetails, Error>;
    fn get_session_token(&self) -> Result<String, Error>;
    fn is_three_ds(&self) -> bool;
}

impl<Flow, Request, Response> RouterData for types::RouterData<Flow, Request, Response> {
    fn get_attempt_id(&self) -> Result<String, Error> {
        self.attempt_id
            .clone()
            .ok_or_else(missing_field_err("attempt_id"))
    }

    fn get_billing_country(&self) -> Result<String, Error> {
        self.address
            .billing
            .as_ref()
            .and_then(|a| a.address.as_ref())
            .and_then(|ad| ad.country.clone())
            .ok_or_else(missing_field_err("billing.address.country"))
    }

    fn get_billing_phone(&self) -> Result<&api::PhoneDetails, Error> {
        self.address
            .billing
            .as_ref()
            .and_then(|a| a.phone.as_ref())
            .ok_or_else(missing_field_err("billing.phone"))
    }

    fn get_billing(&self) -> Result<&api::Address, Error> {
        self.address
            .billing
            .as_ref()
            .ok_or_else(missing_field_err("billing"))
    }

    fn get_session_token(&self) -> Result<String, Error> {
        self.session_token
            .clone()
            .ok_or_else(missing_field_err("session_token"))
    }

    fn is_three_ds(&self) -> bool {
        matches!(
            self.auth_type,
            storage_models::enums::AuthenticationType::ThreeDs
        )
    }
}

pub trait PaymentsRequestData {
    fn get_card(&self) -> Result<api::Card, Error>;
    fn get_email(&self) -> Result<Secret<String, Email>, Error>;
    fn get_browser_info(&self) -> Result<types::BrowserInformation, Error>;
}

impl PaymentsRequestData for types::PaymentsAuthorizeData {
    fn get_card(&self) -> Result<api::Card, Error> {
        match self.payment_method_data.clone() {
            api::PaymentMethod::Card(card) => Ok(card),
            _ => Err(missing_field_err("card")()),
        }
    }

    fn get_email(&self) -> Result<Secret<String, Email>, Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }

    fn get_browser_info(&self) -> Result<types::BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
}

pub trait PaymentsCancelRequestData {
    fn get_amount(&self) -> Result<i64, Error>;
    fn get_currency(&self) -> Result<storage_models::enums::Currency, Error>;
}

impl PaymentsCancelRequestData for PaymentsCancelData {
    fn get_amount(&self) -> Result<i64, Error> {
        self.amount.ok_or_else(missing_field_err("amount"))
    }
    fn get_currency(&self) -> Result<storage_models::enums::Currency, Error> {
        self.currency.ok_or_else(missing_field_err("currency"))
    }
}

pub trait RefundsRequestData {
    fn get_connector_refund_id(&self) -> Result<String, Error>;
}

impl RefundsRequestData for types::RefundsData {
    fn get_connector_refund_id(&self) -> Result<String, Error> {
        self.connector_refund_id
            .clone()
            .get_required_value("connector_refund_id")
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)
    }
}

pub trait CardData {
    fn get_card_number(&self) -> String;
    fn get_card_holder_name(&self) -> String;
    fn get_card_expiry_month(&self) -> String;
    fn get_card_expiry_year(&self) -> String;
    fn get_card_expiry_year_2_digit(&self) -> String;
    fn get_card_cvc(&self) -> String;
}

pub trait WalletData {
    fn get_token(&self) -> Result<String, Error>;
}

impl WalletData for api::WalletData {
    fn get_token(&self) -> Result<String, Error> {
        self.token
            .clone()
            .ok_or_else(missing_field_err("wallet.token"))
    }
}

impl CardData for api::Card {
    fn get_card_number(&self) -> String {
        self.card_number.peek().clone()
    }
    fn get_card_holder_name(&self) -> String {
        self.card_holder_name.peek().clone()
    }
    fn get_card_expiry_month(&self) -> String {
        self.card_exp_month.peek().clone()
    }
    fn get_card_expiry_year(&self) -> String {
        self.card_exp_year.peek().clone()
    }
    fn get_card_expiry_year_2_digit(&self) -> String {
        let year = self.card_exp_year.peek().clone();
        year[year.len() - 2..].to_string()
    }
    fn get_card_cvc(&self) -> String {
        self.card_cvc.peek().clone()
    }
}
pub trait PhoneDetailsData {
    fn get_number(&self) -> Result<Secret<String>, Error>;
    fn get_country_code(&self) -> Result<String, Error>;
}

impl PhoneDetailsData for api::PhoneDetails {
    fn get_country_code(&self) -> Result<String, Error> {
        self.country_code
            .clone()
            .ok_or_else(missing_field_err("billing.phone.country_code"))
    }
    fn get_number(&self) -> Result<Secret<String>, Error> {
        self.number
            .clone()
            .ok_or_else(missing_field_err("billing.phone.number"))
    }
}

pub trait AddressDetailsData {
    fn get_first_name(&self) -> Result<&Secret<String>, Error>;
    fn get_last_name(&self) -> Result<&Secret<String>, Error>;
    fn get_line1(&self) -> Result<&Secret<String>, Error>;
    fn get_city(&self) -> Result<&String, Error>;
    fn get_line2(&self) -> Result<&Secret<String>, Error>;
    fn get_zip(&self) -> Result<&Secret<String>, Error>;
    fn get_country(&self) -> Result<&String, Error>;
}

impl AddressDetailsData for api::AddressDetails {
    fn get_first_name(&self) -> Result<&Secret<String>, Error> {
        self.first_name
            .as_ref()
            .ok_or_else(missing_field_err("address.first_name"))
    }

    fn get_last_name(&self) -> Result<&Secret<String>, Error> {
        self.last_name
            .as_ref()
            .ok_or_else(missing_field_err("address.last_name"))
    }

    fn get_line1(&self) -> Result<&Secret<String>, Error> {
        self.line1
            .as_ref()
            .ok_or_else(missing_field_err("address.line1"))
    }

    fn get_city(&self) -> Result<&String, Error> {
        self.city
            .as_ref()
            .ok_or_else(missing_field_err("address.city"))
    }

    fn get_line2(&self) -> Result<&Secret<String>, Error> {
        self.line2
            .as_ref()
            .ok_or_else(missing_field_err("address.line2"))
    }

    fn get_zip(&self) -> Result<&Secret<String>, Error> {
        self.zip
            .as_ref()
            .ok_or_else(missing_field_err("address.zip"))
    }

    fn get_country(&self) -> Result<&String, Error> {
        self.country
            .as_ref()
            .ok_or_else(missing_field_err("address.country"))
    }
}

pub fn get_header_key_value<'a>(
    key: &str,
    headers: &'a actix_web::http::header::HeaderMap,
) -> CustomResult<&'a str, errors::ConnectorError> {
    headers
        .get(key)
        .map(|header_value| {
            header_value
                .to_str()
                .into_report()
                .change_context(errors::ConnectorError::WebhookSignatureNotFound)
        })
        .ok_or(report!(
            errors::ConnectorError::WebhookSourceVerificationFailed
        ))?
}

pub fn to_boolean(string: String) -> bool {
    let str = string.as_str();
    match str {
        "true" => true,
        "false" => false,
        "yes" => true,
        "no" => false,
        _ => false,
    }
}

pub fn get_connector_meta(
    connector_meta: Option<serde_json::Value>,
) -> Result<serde_json::Value, Error> {
    connector_meta.ok_or_else(missing_field_err("connector_meta_data"))
}

pub fn to_connector_meta<T>(connector_meta: Option<serde_json::Value>) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let json = connector_meta.ok_or_else(missing_field_err("connector_meta_data"))?;
    parse_struct(json)
}

pub fn parse_struct<T>(json: serde_json::Value) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value::<T>(json.clone())
        .into_report()
        .change_context(errors::ConnectorError::ParsingFailed {
            from_type: "Json",
            to_type: std::any::type_name::<T>(),
            data: json.to_string(),
        })
}

pub fn parse_struct_from_bytes<T>(bytes: bytes::Bytes) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_slice::<T>(bytes.chunk())
        .into_report()
        .change_context(errors::ConnectorError::ParsingFailed {
            from_type: "Bytes",
            to_type: std::any::type_name::<T>(),
            data: String::from_utf8(bytes.to_vec())
                .into_report()
                .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
        })
}

pub fn parse_struct_from_bytes_slice<T>(bytes: &[u8]) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_slice::<T>(bytes)
        .into_report()
        .change_context(errors::ConnectorError::ParsingFailed {
            from_type: "&[u8]",
            to_type: std::any::type_name::<T>(),
            data: String::from_utf8(bytes.to_vec())
                .into_report()
                .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
        })
}

pub fn to_string<T>(data: &T) -> Result<String, Error>
where
    T: serde::Serialize,
{
    serde_json::to_string(data)
        .into_report()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

pub fn base64_decode(data: String) -> Result<Vec<u8>, Error> {
    consts::BASE64_ENGINE
        .decode(data)
        .into_report()
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
}
