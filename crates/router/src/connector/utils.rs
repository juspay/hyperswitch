use error_stack::{report, IntoReport, ResultExt};
use masking::Secret;

use crate::{
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
    fn get_billing(&self) -> Result<&api::Address, Error>;
    fn get_billing_country(&self) -> Result<String, Error>;
    fn get_billing_phone(&self) -> Result<&api::PhoneDetails, Error>;
    fn get_connector_meta(&self) -> Result<serde_json::Value, Error>;
    fn get_session_token(&self) -> Result<String, Error>;
    fn get_billing_address(&self) -> Result<&api::AddressDetails, Error>;
    fn to_connector_meta<T>(&self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned;
}

pub trait PaymentsRequestData {
    fn get_card(&self) -> Result<api::Card, Error>;
    fn get_return_url(&self) -> Result<String, Error>;
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

impl<Flow, Request, Response> RouterData for types::RouterData<Flow, Request, Response> {
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
    fn get_billing_address(&self) -> Result<&api::AddressDetails, Error> {
        self.address
            .billing
            .as_ref()
            .and_then(|a| a.address.as_ref())
            .ok_or_else(missing_field_err("billing.address"))
    }
    fn get_billing(&self) -> Result<&api::Address, Error> {
        self.address
            .billing
            .as_ref()
            .ok_or_else(missing_field_err("billing"))
    }

    fn get_connector_meta(&self) -> Result<serde_json::Value, Error> {
        self.connector_meta_data
            .clone()
            .ok_or_else(missing_field_err("connector_meta_data"))
    }

    fn get_session_token(&self) -> Result<String, Error> {
        self.session_token
            .clone()
            .ok_or_else(missing_field_err("session_token"))
    }

    fn to_connector_meta<T>(&self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_value::<T>(self.get_connector_meta()?)
            .into_report()
            .change_context(errors::ConnectorError::NoConnectorMetaData)
    }
}

impl PaymentsRequestData for types::PaymentsAuthorizeRouterData {
    fn get_return_url(&self) -> Result<String, Error> {
        self.router_return_url
            .clone()
            .ok_or_else(missing_field_err("router_return_url"))
    }
    fn get_card(&self) -> Result<api::Card, Error> {
        match self.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(card) => Ok(card),
            _ => Err(missing_field_err("card")()),
        }
    }
}

pub trait CardData {
    fn get_card_expiry_year_2_digit(&self) -> Secret<String>;
}

impl CardData for api::Card {
    fn get_card_expiry_year_2_digit(&self) -> Secret<String> {
        let binding = self.card_exp_year.clone();
        let year = binding.peek();
        Secret::new(year[year.len() - 2..].to_string())
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
