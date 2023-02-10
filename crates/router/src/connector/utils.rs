use error_stack::{report, IntoReport, ResultExt};
use masking::Secret;

use crate::{
    core::errors::{self, CustomResult},
    pii::PeekInterface,
    types::{self, api},
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

pub trait PaymentsRequestData {
    fn get_attempt_id(&self) -> Result<String, Error>;
    fn get_billing(&self) -> Result<&api::Address, Error>;
    fn get_billing_country(&self) -> Result<String, Error>;
    fn get_billing_phone(&self) -> Result<&api::PhoneDetails, Error>;
    fn get_card(&self) -> Result<api::Card, Error>;
    fn get_return_url(&self) -> Result<String, Error>;
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

impl PaymentsRequestData for types::PaymentsAuthorizeRouterData {
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

    fn get_card(&self) -> Result<api::Card, Error> {
        match self.request.payment_method_data.clone() {
            api::PaymentMethod::Card(card) => Ok(card),
            _ => Err(missing_field_err("card")()),
        }
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

    fn get_return_url(&self) -> Result<String, Error> {
        self.router_return_url
            .clone()
            .ok_or_else(missing_field_err("router_return_url"))
    }
}

pub trait CardData {
    fn get_card_number(&self) -> String;
    fn get_card_expiry_month(&self) -> String;
    fn get_card_expiry_year(&self) -> String;
    fn get_card_expiry_year_2_digit(&self) -> String;
    fn get_card_cvc(&self) -> String;
}

impl CardData for api::Card {
    fn get_card_number(&self) -> String {
        self.card_number.peek().clone()
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
