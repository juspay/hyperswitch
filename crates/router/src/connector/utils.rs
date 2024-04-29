use std::collections::HashMap;

use api_models::{
    enums::{CanadaStatesAbbreviation, UsStatesAbbreviation},
    payments::{self, OrderDetailsWithAmount},
};
use base64::Engine;
use common_utils::{
    date_time,
    errors::ReportSwitchExt,
    pii::{self, Email, IpAddress},
};
use data_models::payments::payment_attempt::PaymentAttempt;
use diesel_models::enums;
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Secret};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serializer;
use time::PrimitiveDateTime;

#[cfg(feature = "frm")]
use crate::types::{fraud_check, storage::enums as storage_enums};
use crate::{
    consts,
    core::{
        errors::{self, ApiErrorResponse, CustomResult},
        payments::{types::AuthenticationData, PaymentData, RecurringMandatePaymentData},
    },
    pii::PeekInterface,
    types::{
        self, api, domain, transformers::ForeignTryFrom, ApplePayPredecryptData,
        BrowserInformation, PaymentsCancelData, ResponseId,
    },
    utils::{OptionExt, ValueExt},
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
    fn get_request_id(&self) -> Result<Secret<String>, Error>;
}

impl AccessTokenRequestInfo for types::RefreshTokenRouterData {
    fn get_request_id(&self) -> Result<Secret<String>, Error> {
        self.request
            .id
            .clone()
            .ok_or_else(missing_field_err("request.id"))
    }
}

pub trait RouterData {
    fn get_billing(&self) -> Result<&api::Address, Error>;
    fn get_billing_country(&self) -> Result<api_models::enums::CountryAlpha2, Error>;
    fn get_billing_phone(&self) -> Result<&api::PhoneDetails, Error>;
    fn get_description(&self) -> Result<String, Error>;
    fn get_return_url(&self) -> Result<String, Error>;
    fn get_billing_address(&self) -> Result<&api::AddressDetails, Error>;
    fn get_shipping_address(&self) -> Result<&api::AddressDetails, Error>;
    fn get_shipping_address_with_phone_number(&self) -> Result<&api::Address, Error>;
    fn get_connector_meta(&self) -> Result<pii::SecretSerdeValue, Error>;
    fn get_session_token(&self) -> Result<String, Error>;
    fn get_billing_first_name(&self) -> Result<Secret<String>, Error>;
    fn get_billing_email(&self) -> Result<Email, Error>;
    fn get_billing_phone_number(&self) -> Result<Secret<String>, Error>;
    fn to_connector_meta<T>(&self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned;
    fn is_three_ds(&self) -> bool;
    fn get_payment_method_token(&self) -> Result<types::PaymentMethodToken, Error>;
    fn get_customer_id(&self) -> Result<String, Error>;
    fn get_connector_customer_id(&self) -> Result<String, Error>;
    fn get_preprocessing_id(&self) -> Result<String, Error>;
    fn get_recurring_mandate_payment_data(&self) -> Result<RecurringMandatePaymentData, Error>;
    #[cfg(feature = "payouts")]
    fn get_payout_method_data(&self) -> Result<api::PayoutMethodData, Error>;
    #[cfg(feature = "payouts")]
    fn get_quote_id(&self) -> Result<String, Error>;

    fn get_optional_billing(&self) -> Option<&api::Address>;
    fn get_optional_shipping(&self) -> Option<&api::Address>;

    fn get_optional_billing_full_name(&self) -> Option<Secret<String>>;
    fn get_optional_billing_line1(&self) -> Option<Secret<String>>;
    fn get_optional_billing_line2(&self) -> Option<Secret<String>>;
    fn get_optional_billing_city(&self) -> Option<String>;
    fn get_optional_billing_country(&self) -> Option<enums::CountryAlpha2>;
    fn get_optional_billing_zip(&self) -> Option<Secret<String>>;
    fn get_optional_billing_state(&self) -> Option<Secret<String>>;
    fn get_optional_billing_first_name(&self) -> Option<Secret<String>>;
    fn get_optional_billing_last_name(&self) -> Option<Secret<String>>;
    fn get_optional_billing_phone_number(&self) -> Option<Secret<String>>;
    fn get_optional_billing_email(&self) -> Option<Email>;
}

pub trait PaymentResponseRouterData {
    fn get_attempt_status_for_db_update<F>(
        &self,
        payment_data: &PaymentData<F>,
    ) -> enums::AttemptStatus
    where
        F: Clone;
}

impl<Flow, Request, Response> PaymentResponseRouterData
    for types::RouterData<Flow, Request, Response>
where
    Request: types::Capturable,
{
    fn get_attempt_status_for_db_update<F>(
        &self,
        payment_data: &PaymentData<F>,
    ) -> enums::AttemptStatus
    where
        F: Clone,
    {
        match self.status {
            enums::AttemptStatus::Voided => {
                if payment_data.payment_intent.amount_captured > Some(0) {
                    enums::AttemptStatus::PartialCharged
                } else {
                    self.status
                }
            }
            enums::AttemptStatus::Charged => {
                let captured_amount =
                    types::Capturable::get_captured_amount(&self.request, payment_data);
                let total_capturable_amount = payment_data.payment_attempt.get_total_amount();
                if Some(total_capturable_amount) == captured_amount {
                    enums::AttemptStatus::Charged
                } else if captured_amount.is_some() {
                    enums::AttemptStatus::PartialCharged
                } else {
                    self.status
                }
            }
            _ => self.status,
        }
    }
}

pub const SELECTED_PAYMENT_METHOD: &str = "Selected payment method";

pub fn get_unimplemented_payment_method_error_message(connector: &str) -> String {
    format!("{} through {}", SELECTED_PAYMENT_METHOD, connector)
}

impl<Flow, Request, Response> RouterData for types::RouterData<Flow, Request, Response> {
    fn get_billing(&self) -> Result<&api::Address, Error> {
        self.address
            .get_payment_method_billing()
            .ok_or_else(missing_field_err("billing"))
    }

    fn get_billing_country(&self) -> Result<api_models::enums::CountryAlpha2, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|a| a.address.as_ref())
            .and_then(|ad| ad.country)
            .ok_or_else(missing_field_err("billing.address.country"))
    }

    fn get_billing_phone(&self) -> Result<&api::PhoneDetails, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|a| a.phone.as_ref())
            .ok_or_else(missing_field_err("billing.phone"))
    }

    fn get_optional_billing(&self) -> Option<&api::Address> {
        self.address.get_payment_method_billing()
    }

    fn get_optional_shipping(&self) -> Option<&api::Address> {
        self.address.get_shipping()
    }

    fn get_description(&self) -> Result<String, Error> {
        self.description
            .clone()
            .ok_or_else(missing_field_err("description"))
    }
    fn get_return_url(&self) -> Result<String, Error> {
        self.return_url
            .clone()
            .ok_or_else(missing_field_err("return_url"))
    }
    fn get_billing_address(&self) -> Result<&api::AddressDetails, Error> {
        self.address
            .get_payment_method_billing()
            .as_ref()
            .and_then(|a| a.address.as_ref())
            .ok_or_else(missing_field_err("billing.address"))
    }

    fn get_connector_meta(&self) -> Result<pii::SecretSerdeValue, Error> {
        self.connector_meta_data
            .clone()
            .ok_or_else(missing_field_err("connector_meta_data"))
    }

    fn get_session_token(&self) -> Result<String, Error> {
        self.session_token
            .clone()
            .ok_or_else(missing_field_err("session_token"))
    }

    fn get_billing_first_name(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_address_details| billing_address_details.first_name.clone())
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.first_name",
            ))
    }

    fn get_billing_email(&self) -> Result<Email, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| billing_address.email.clone())
            .ok_or_else(missing_field_err("payment_method_data.billing.email"))
    }

    fn get_billing_phone_number(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| billing_address.clone().phone)
            .map(|phone_details| phone_details.get_number_with_country_code())
            .transpose()?
            .ok_or_else(missing_field_err("payment_method_data.billing.phone"))
    }

    fn get_optional_billing_line1(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_address_details| billing_address_details.line1)
            })
    }

    fn get_optional_billing_line2(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_address_details| billing_address_details.line2)
            })
    }

    fn get_optional_billing_city(&self) -> Option<String> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_address_details| billing_address_details.city)
            })
    }

    fn get_optional_billing_country(&self) -> Option<enums::CountryAlpha2> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_address_details| billing_address_details.country)
            })
    }

    fn get_optional_billing_zip(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_address_details| billing_address_details.zip)
            })
    }

    fn get_optional_billing_state(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_address_details| billing_address_details.state)
            })
    }

    fn get_optional_billing_first_name(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_address_details| billing_address_details.first_name)
            })
    }

    fn get_optional_billing_last_name(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_address_details| billing_address_details.last_name)
            })
    }

    fn get_optional_billing_phone_number(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .phone
                    .and_then(|phone_data| phone_data.number)
            })
    }

    fn get_optional_billing_email(&self) -> Option<Email> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| billing_address.clone().email)
    }
    fn to_connector_meta<T>(&self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.get_connector_meta()?
            .parse_value(std::any::type_name::<T>())
            .change_context(errors::ConnectorError::NoConnectorMetaData)
    }

    fn is_three_ds(&self) -> bool {
        matches!(
            self.auth_type,
            diesel_models::enums::AuthenticationType::ThreeDs
        )
    }

    fn get_shipping_address(&self) -> Result<&api::AddressDetails, Error> {
        self.address
            .get_shipping()
            .and_then(|a| a.address.as_ref())
            .ok_or_else(missing_field_err("shipping.address"))
    }

    fn get_shipping_address_with_phone_number(&self) -> Result<&api::Address, Error> {
        self.address
            .get_shipping()
            .ok_or_else(missing_field_err("shipping"))
    }

    fn get_payment_method_token(&self) -> Result<types::PaymentMethodToken, Error> {
        self.payment_method_token
            .clone()
            .ok_or_else(missing_field_err("payment_method_token"))
    }
    fn get_customer_id(&self) -> Result<String, Error> {
        self.customer_id
            .to_owned()
            .ok_or_else(missing_field_err("customer_id"))
    }
    fn get_connector_customer_id(&self) -> Result<String, Error> {
        self.connector_customer
            .to_owned()
            .ok_or_else(missing_field_err("connector_customer_id"))
    }
    fn get_preprocessing_id(&self) -> Result<String, Error> {
        self.preprocessing_id
            .to_owned()
            .ok_or_else(missing_field_err("preprocessing_id"))
    }
    fn get_recurring_mandate_payment_data(&self) -> Result<RecurringMandatePaymentData, Error> {
        self.recurring_mandate_payment_data
            .to_owned()
            .ok_or_else(missing_field_err("recurring_mandate_payment_data"))
    }

    fn get_optional_billing_full_name(&self) -> Option<Secret<String>> {
        self.get_optional_billing()
            .and_then(|billing_details| billing_details.address.as_ref())
            .and_then(|billing_address| billing_address.get_optional_full_name())
    }

    #[cfg(feature = "payouts")]
    fn get_payout_method_data(&self) -> Result<api::PayoutMethodData, Error> {
        self.payout_method_data
            .to_owned()
            .ok_or_else(missing_field_err("payout_method_data"))
    }
    #[cfg(feature = "payouts")]
    fn get_quote_id(&self) -> Result<String, Error> {
        self.quote_id
            .to_owned()
            .ok_or_else(missing_field_err("quote_id"))
    }
}

pub trait PaymentsPreProcessingData {
    fn get_email(&self) -> Result<Email, Error>;
    fn get_payment_method_type(&self) -> Result<diesel_models::enums::PaymentMethodType, Error>;
    fn get_currency(&self) -> Result<diesel_models::enums::Currency, Error>;
    fn get_amount(&self) -> Result<i64, Error>;
    fn is_auto_capture(&self) -> Result<bool, Error>;
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error>;
    fn get_webhook_url(&self) -> Result<String, Error>;
    fn get_return_url(&self) -> Result<String, Error>;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
    fn get_complete_authorize_url(&self) -> Result<String, Error>;
}

impl PaymentsPreProcessingData for types::PaymentsPreProcessingData {
    fn get_email(&self) -> Result<Email, Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }
    fn get_payment_method_type(&self) -> Result<diesel_models::enums::PaymentMethodType, Error> {
        self.payment_method_type
            .to_owned()
            .ok_or_else(missing_field_err("payment_method_type"))
    }
    fn get_currency(&self) -> Result<diesel_models::enums::Currency, Error> {
        self.currency.ok_or_else(missing_field_err("currency"))
    }
    fn get_amount(&self) -> Result<i64, Error> {
        self.amount.ok_or_else(missing_field_err("amount"))
    }
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(diesel_models::enums::CaptureMethod::Automatic) | None => Ok(true),
            Some(diesel_models::enums::CaptureMethod::Manual) => Ok(false),
            Some(_) => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
        }
    }
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error> {
        self.order_details
            .clone()
            .ok_or_else(missing_field_err("order_details"))
    }
    fn get_webhook_url(&self) -> Result<String, Error> {
        self.webhook_url
            .clone()
            .ok_or_else(missing_field_err("webhook_url"))
    }
    fn get_return_url(&self) -> Result<String, Error> {
        self.router_return_url
            .clone()
            .ok_or_else(missing_field_err("return_url"))
    }
    fn get_browser_info(&self) -> Result<BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
    fn get_complete_authorize_url(&self) -> Result<String, Error> {
        self.complete_authorize_url
            .clone()
            .ok_or_else(missing_field_err("complete_authorize_url"))
    }
}

pub trait PaymentsCaptureRequestData {
    fn is_multiple_capture(&self) -> bool;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
}

impl PaymentsCaptureRequestData for types::PaymentsCaptureData {
    fn is_multiple_capture(&self) -> bool {
        self.multiple_capture_data.is_some()
    }
    fn get_browser_info(&self) -> Result<BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
}

pub trait RevokeMandateRequestData {
    fn get_connector_mandate_id(&self) -> Result<String, Error>;
}

impl RevokeMandateRequestData for types::MandateRevokeRequestData {
    fn get_connector_mandate_id(&self) -> Result<String, Error> {
        self.connector_mandate_id
            .clone()
            .ok_or_else(missing_field_err("connector_mandate_id"))
    }
}

pub trait PaymentsSetupMandateRequestData {
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
    fn get_email(&self) -> Result<Email, Error>;
    fn is_card(&self) -> bool;
}

impl PaymentsSetupMandateRequestData for types::SetupMandateRequestData {
    fn get_browser_info(&self) -> Result<BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
    fn get_email(&self) -> Result<Email, Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }
    fn is_card(&self) -> bool {
        matches!(self.payment_method_data, domain::PaymentMethodData::Card(_))
    }
}
pub trait PaymentsAuthorizeRequestData {
    fn is_auto_capture(&self) -> Result<bool, Error>;
    fn get_email(&self) -> Result<Email, Error>;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error>;
    fn get_card(&self) -> Result<domain::Card, Error>;
    fn get_return_url(&self) -> Result<String, Error>;
    fn connector_mandate_id(&self) -> Option<String>;
    fn is_mandate_payment(&self) -> bool;
    fn is_customer_initiated_mandate_payment(&self) -> bool;
    fn get_webhook_url(&self) -> Result<String, Error>;
    fn get_router_return_url(&self) -> Result<String, Error>;
    fn is_wallet(&self) -> bool;
    fn is_card(&self) -> bool;
    fn get_payment_method_type(&self) -> Result<diesel_models::enums::PaymentMethodType, Error>;
    fn get_connector_mandate_id(&self) -> Result<String, Error>;
    fn get_complete_authorize_url(&self) -> Result<String, Error>;
    fn get_ip_address_as_optional(&self) -> Option<Secret<String, IpAddress>>;
    fn get_original_amount(&self) -> i64;
    fn get_surcharge_amount(&self) -> Option<i64>;
    fn get_tax_on_surcharge_amount(&self) -> Option<i64>;
    fn get_total_surcharge_amount(&self) -> Option<i64>;
    fn get_metadata_as_object(&self) -> Option<pii::SecretSerdeValue>;
    fn get_authentication_data(&self) -> Result<AuthenticationData, Error>;
}

pub trait PaymentMethodTokenizationRequestData {
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
}

impl PaymentMethodTokenizationRequestData for types::PaymentMethodTokenizationData {
    fn get_browser_info(&self) -> Result<BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
}

impl PaymentsAuthorizeRequestData for types::PaymentsAuthorizeData {
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(diesel_models::enums::CaptureMethod::Automatic) | None => Ok(true),
            Some(diesel_models::enums::CaptureMethod::Manual) => Ok(false),
            Some(_) => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
        }
    }
    fn get_email(&self) -> Result<Email, Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }
    fn get_browser_info(&self) -> Result<BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error> {
        self.order_details
            .clone()
            .ok_or_else(missing_field_err("order_details"))
    }

    fn get_card(&self) -> Result<domain::Card, Error> {
        match self.payment_method_data.clone() {
            domain::PaymentMethodData::Card(card) => Ok(card),
            _ => Err(missing_field_err("card")()),
        }
    }
    fn get_return_url(&self) -> Result<String, Error> {
        self.router_return_url
            .clone()
            .ok_or_else(missing_field_err("return_url"))
    }

    fn get_complete_authorize_url(&self) -> Result<String, Error> {
        self.complete_authorize_url
            .clone()
            .ok_or_else(missing_field_err("complete_authorize_url"))
    }

    fn connector_mandate_id(&self) -> Option<String> {
        self.mandate_id
            .as_ref()
            .and_then(|mandate_ids| match &mandate_ids.mandate_reference_id {
                Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                    connector_mandate_ids,
                )) => connector_mandate_ids.connector_mandate_id.clone(),
                _ => None,
            })
    }
    fn is_mandate_payment(&self) -> bool {
        self.setup_mandate_details.is_some()
            || self
                .mandate_id
                .as_ref()
                .and_then(|mandate_ids| mandate_ids.mandate_reference_id.as_ref())
                .is_some()
    }
    fn get_webhook_url(&self) -> Result<String, Error> {
        self.webhook_url
            .clone()
            .ok_or_else(missing_field_err("webhook_url"))
    }
    fn get_router_return_url(&self) -> Result<String, Error> {
        self.router_return_url
            .clone()
            .ok_or_else(missing_field_err("return_url"))
    }
    fn is_wallet(&self) -> bool {
        matches!(
            self.payment_method_data,
            domain::PaymentMethodData::Wallet(_)
        )
    }
    fn is_card(&self) -> bool {
        matches!(self.payment_method_data, domain::PaymentMethodData::Card(_))
    }

    fn get_payment_method_type(&self) -> Result<diesel_models::enums::PaymentMethodType, Error> {
        self.payment_method_type
            .to_owned()
            .ok_or_else(missing_field_err("payment_method_type"))
    }

    fn get_connector_mandate_id(&self) -> Result<String, Error> {
        self.connector_mandate_id()
            .ok_or_else(missing_field_err("connector_mandate_id"))
    }
    fn get_ip_address_as_optional(&self) -> Option<Secret<String, IpAddress>> {
        self.browser_info.clone().and_then(|browser_info| {
            browser_info
                .ip_address
                .map(|ip| Secret::new(ip.to_string()))
        })
    }
    fn get_original_amount(&self) -> i64 {
        self.surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.original_amount)
            .unwrap_or(self.amount)
    }
    fn get_surcharge_amount(&self) -> Option<i64> {
        self.surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.surcharge_amount)
    }
    fn get_tax_on_surcharge_amount(&self) -> Option<i64> {
        self.surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.tax_on_surcharge_amount)
    }
    fn get_total_surcharge_amount(&self) -> Option<i64> {
        self.surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.get_total_surcharge_amount())
    }

    fn is_customer_initiated_mandate_payment(&self) -> bool {
        self.setup_mandate_details.is_some()
    }

    fn get_metadata_as_object(&self) -> Option<pii::SecretSerdeValue> {
        self.metadata
            .clone()
            .and_then(|meta_data| match meta_data.peek() {
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::String(_)
                | serde_json::Value::Array(_) => None,
                serde_json::Value::Object(_) => Some(meta_data),
            })
    }

    fn get_authentication_data(&self) -> Result<AuthenticationData, Error> {
        self.authentication_data
            .clone()
            .ok_or_else(missing_field_err("authentication_data"))
    }
}

pub trait ConnectorCustomerData {
    fn get_email(&self) -> Result<Email, Error>;
}

impl ConnectorCustomerData for types::ConnectorCustomerData {
    fn get_email(&self) -> Result<Email, Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }
}

pub trait BrowserInformationData {
    fn get_accept_header(&self) -> Result<String, Error>;
    fn get_language(&self) -> Result<String, Error>;
    fn get_screen_height(&self) -> Result<u32, Error>;
    fn get_screen_width(&self) -> Result<u32, Error>;
    fn get_color_depth(&self) -> Result<u8, Error>;
    fn get_user_agent(&self) -> Result<String, Error>;
    fn get_time_zone(&self) -> Result<i32, Error>;
    fn get_java_enabled(&self) -> Result<bool, Error>;
    fn get_java_script_enabled(&self) -> Result<bool, Error>;
    fn get_ip_address(&self) -> Result<Secret<String, IpAddress>, Error>;
}

impl BrowserInformationData for BrowserInformation {
    fn get_ip_address(&self) -> Result<Secret<String, IpAddress>, Error> {
        let ip_address = self
            .ip_address
            .ok_or_else(missing_field_err("browser_info.ip_address"))?;
        Ok(Secret::new(ip_address.to_string()))
    }
    fn get_accept_header(&self) -> Result<String, Error> {
        self.accept_header
            .clone()
            .ok_or_else(missing_field_err("browser_info.accept_header"))
    }
    fn get_language(&self) -> Result<String, Error> {
        self.language
            .clone()
            .ok_or_else(missing_field_err("browser_info.language"))
    }
    fn get_screen_height(&self) -> Result<u32, Error> {
        self.screen_height
            .ok_or_else(missing_field_err("browser_info.screen_height"))
    }
    fn get_screen_width(&self) -> Result<u32, Error> {
        self.screen_width
            .ok_or_else(missing_field_err("browser_info.screen_width"))
    }
    fn get_color_depth(&self) -> Result<u8, Error> {
        self.color_depth
            .ok_or_else(missing_field_err("browser_info.color_depth"))
    }
    fn get_user_agent(&self) -> Result<String, Error> {
        self.user_agent
            .clone()
            .ok_or_else(missing_field_err("browser_info.user_agent"))
    }
    fn get_time_zone(&self) -> Result<i32, Error> {
        self.time_zone
            .ok_or_else(missing_field_err("browser_info.time_zone"))
    }
    fn get_java_enabled(&self) -> Result<bool, Error> {
        self.java_enabled
            .ok_or_else(missing_field_err("browser_info.java_enabled"))
    }
    fn get_java_script_enabled(&self) -> Result<bool, Error> {
        self.java_script_enabled
            .ok_or_else(missing_field_err("browser_info.java_script_enabled"))
    }
}

pub trait PaymentsCompleteAuthorizeRequestData {
    fn is_auto_capture(&self) -> Result<bool, Error>;
    fn get_email(&self) -> Result<Email, Error>;
    fn get_redirect_response_payload(&self) -> Result<pii::SecretSerdeValue, Error>;
    fn get_complete_authorize_url(&self) -> Result<String, Error>;
}

impl PaymentsCompleteAuthorizeRequestData for types::CompleteAuthorizeData {
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(diesel_models::enums::CaptureMethod::Automatic) | None => Ok(true),
            Some(diesel_models::enums::CaptureMethod::Manual) => Ok(false),
            Some(_) => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
        }
    }
    fn get_email(&self) -> Result<Email, Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }
    fn get_redirect_response_payload(&self) -> Result<pii::SecretSerdeValue, Error> {
        self.redirect_response
            .as_ref()
            .and_then(|res| res.payload.to_owned())
            .ok_or(
                errors::ConnectorError::MissingConnectorRedirectionPayload {
                    field_name: "request.redirect_response.payload",
                }
                .into(),
            )
    }
    fn get_complete_authorize_url(&self) -> Result<String, Error> {
        self.complete_authorize_url
            .clone()
            .ok_or_else(missing_field_err("complete_authorize_url"))
    }
}

pub trait PaymentsSyncRequestData {
    fn is_auto_capture(&self) -> Result<bool, Error>;
    fn get_connector_transaction_id(&self) -> CustomResult<String, errors::ConnectorError>;
}

impl PaymentsSyncRequestData for types::PaymentsSyncData {
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(diesel_models::enums::CaptureMethod::Automatic) | None => Ok(true),
            Some(diesel_models::enums::CaptureMethod::Manual) => Ok(false),
            Some(_) => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
        }
    }
    fn get_connector_transaction_id(&self) -> CustomResult<String, errors::ConnectorError> {
        match self.connector_transaction_id.clone() {
            ResponseId::ConnectorTransactionId(txn_id) => Ok(txn_id),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "connector_transaction_id",
            })
            .attach_printable("Expected connector transaction ID not found")
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
        }
    }
}

pub trait PaymentsCancelRequestData {
    fn get_amount(&self) -> Result<i64, Error>;
    fn get_currency(&self) -> Result<diesel_models::enums::Currency, Error>;
    fn get_cancellation_reason(&self) -> Result<String, Error>;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
}

impl PaymentsCancelRequestData for PaymentsCancelData {
    fn get_amount(&self) -> Result<i64, Error> {
        self.amount.ok_or_else(missing_field_err("amount"))
    }
    fn get_currency(&self) -> Result<diesel_models::enums::Currency, Error> {
        self.currency.ok_or_else(missing_field_err("currency"))
    }
    fn get_cancellation_reason(&self) -> Result<String, Error> {
        self.cancellation_reason
            .clone()
            .ok_or_else(missing_field_err("cancellation_reason"))
    }
    fn get_browser_info(&self) -> Result<BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
}

pub trait RefundsRequestData {
    fn get_connector_refund_id(&self) -> Result<String, Error>;
    fn get_webhook_url(&self) -> Result<String, Error>;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
}

impl RefundsRequestData for types::RefundsData {
    #[track_caller]
    fn get_connector_refund_id(&self) -> Result<String, Error> {
        self.connector_refund_id
            .clone()
            .get_required_value("connector_refund_id")
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)
    }
    fn get_webhook_url(&self) -> Result<String, Error> {
        self.webhook_url
            .clone()
            .ok_or_else(missing_field_err("webhook_url"))
    }
    fn get_browser_info(&self) -> Result<BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayWalletData {
    #[serde(rename = "type")]
    pub pm_type: String,
    pub description: String,
    pub info: GooglePayPaymentMethodInfo,
    pub tokenization_data: GpayTokenizationData,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayPaymentMethodInfo {
    pub card_network: String,
    pub card_details: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct GpayTokenizationData {
    #[serde(rename = "type")]
    pub token_type: String,
    pub token: Secret<String>,
}

impl From<domain::GooglePayWalletData> for GooglePayWalletData {
    fn from(data: domain::GooglePayWalletData) -> Self {
        Self {
            pm_type: data.pm_type,
            description: data.description,
            info: GooglePayPaymentMethodInfo {
                card_network: data.info.card_network,
                card_details: data.info.card_details,
            },
            tokenization_data: GpayTokenizationData {
                token_type: data.tokenization_data.token_type,
                token: Secret::new(data.tokenization_data.token),
            },
        }
    }
}

static CARD_REGEX: Lazy<HashMap<CardIssuer, Result<Regex, regex::Error>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    // Reference: https://gist.github.com/michaelkeevildown/9096cd3aac9029c4e6e05588448a8841
    // [#379]: Determine card issuer from card BIN number
    map.insert(CardIssuer::Master, Regex::new(r"^5[1-5][0-9]{14}$"));
    map.insert(CardIssuer::AmericanExpress, Regex::new(r"^3[47][0-9]{13}$"));
    map.insert(CardIssuer::Visa, Regex::new(r"^4[0-9]{12}(?:[0-9]{3})?$"));
    map.insert(CardIssuer::Discover, Regex::new(r"^65[4-9][0-9]{13}|64[4-9][0-9]{13}|6011[0-9]{12}|(622(?:12[6-9]|1[3-9][0-9]|[2-8][0-9][0-9]|9[01][0-9]|92[0-5])[0-9]{10})$"));
    map.insert(
        CardIssuer::Maestro,
        Regex::new(r"^(5018|5020|5038|5893|6304|6759|6761|6762|6763)[0-9]{8,15}$"),
    );
    map.insert(
        CardIssuer::DinersClub,
        Regex::new(r"^3(?:0[0-5]|[68][0-9])[0-9]{11}$"),
    );
    map.insert(
        CardIssuer::JCB,
        Regex::new(r"^(3(?:088|096|112|158|337|5(?:2[89]|[3-8][0-9]))\d{12})$"),
    );
    map.insert(CardIssuer::CarteBlanche, Regex::new(r"^389[0-9]{11}$"));
    map
});

#[derive(Debug, Copy, Clone, strum::Display, Eq, Hash, PartialEq)]
pub enum CardIssuer {
    AmericanExpress,
    Master,
    Maestro,
    Visa,
    Discover,
    DinersClub,
    JCB,
    CarteBlanche,
}

pub trait CardData {
    fn get_card_expiry_year_2_digit(&self) -> Result<Secret<String>, errors::ConnectorError>;
    fn get_card_issuer(&self) -> Result<CardIssuer, Error>;
    fn get_card_expiry_month_year_2_digit_with_delimiter(
        &self,
        delimiter: String,
    ) -> Result<Secret<String>, errors::ConnectorError>;
    fn get_expiry_date_as_yyyymm(&self, delimiter: &str) -> Secret<String>;
    fn get_expiry_date_as_mmyyyy(&self, delimiter: &str) -> Secret<String>;
    fn get_expiry_year_4_digit(&self) -> Secret<String>;
    fn get_expiry_date_as_yymm(&self) -> Result<Secret<String>, errors::ConnectorError>;
    fn get_expiry_month_as_i8(&self) -> Result<Secret<i8>, Error>;
    fn get_expiry_year_as_i32(&self) -> Result<Secret<i32>, Error>;
}

impl CardData for domain::Card {
    fn get_card_expiry_year_2_digit(&self) -> Result<Secret<String>, errors::ConnectorError> {
        let binding = self.card_exp_year.clone();
        let year = binding.peek();
        Ok(Secret::new(
            year.get(year.len() - 2..)
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                .to_string(),
        ))
    }
    fn get_card_issuer(&self) -> Result<CardIssuer, Error> {
        get_card_issuer(self.card_number.peek())
    }
    fn get_card_expiry_month_year_2_digit_with_delimiter(
        &self,
        delimiter: String,
    ) -> Result<Secret<String>, errors::ConnectorError> {
        let year = self.get_card_expiry_year_2_digit()?;
        Ok(Secret::new(format!(
            "{}{}{}",
            self.card_exp_month.peek(),
            delimiter,
            year.peek()
        )))
    }
    fn get_expiry_date_as_yyyymm(&self, delimiter: &str) -> Secret<String> {
        let year = self.get_expiry_year_4_digit();
        Secret::new(format!(
            "{}{}{}",
            year.peek(),
            delimiter,
            self.card_exp_month.peek()
        ))
    }
    fn get_expiry_date_as_mmyyyy(&self, delimiter: &str) -> Secret<String> {
        let year = self.get_expiry_year_4_digit();
        Secret::new(format!(
            "{}{}{}",
            self.card_exp_month.peek(),
            delimiter,
            year.peek()
        ))
    }
    fn get_expiry_year_4_digit(&self) -> Secret<String> {
        let mut year = self.card_exp_year.peek().clone();
        if year.len() == 2 {
            year = format!("20{}", year);
        }
        Secret::new(year)
    }
    fn get_expiry_date_as_yymm(&self) -> Result<Secret<String>, errors::ConnectorError> {
        let year = self.get_card_expiry_year_2_digit()?.expose();
        let month = self.card_exp_month.clone().expose();
        Ok(Secret::new(format!("{year}{month}")))
    }
    fn get_expiry_month_as_i8(&self) -> Result<Secret<i8>, Error> {
        self.card_exp_month
            .peek()
            .clone()
            .parse::<i8>()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            .map(Secret::new)
    }
    fn get_expiry_year_as_i32(&self) -> Result<Secret<i32>, Error> {
        self.card_exp_year
            .peek()
            .clone()
            .parse::<i32>()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            .map(Secret::new)
    }
}

#[track_caller]
fn get_card_issuer(card_number: &str) -> Result<CardIssuer, Error> {
    for (k, v) in CARD_REGEX.iter() {
        let regex: Regex = v
            .clone()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        if regex.is_match(card_number) {
            return Ok(*k);
        }
    }
    Err(error_stack::Report::new(
        errors::ConnectorError::NotImplemented("Card Type".into()),
    ))
}
pub trait WalletData {
    fn get_wallet_token(&self) -> Result<Secret<String>, Error>;
    fn get_wallet_token_as_json<T>(&self, wallet_name: String) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned;
    fn get_encoded_wallet_token(&self) -> Result<String, Error>;
}

impl WalletData for domain::WalletData {
    fn get_wallet_token(&self) -> Result<Secret<String>, Error> {
        match self {
            Self::GooglePay(data) => Ok(Secret::new(data.tokenization_data.token.clone())),
            Self::ApplePay(data) => Ok(data.get_applepay_decoded_payment_data()?),
            Self::PaypalSdk(data) => Ok(Secret::new(data.token.clone())),
            _ => Err(errors::ConnectorError::InvalidWallet.into()),
        }
    }
    fn get_wallet_token_as_json<T>(&self, wallet_name: String) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_str::<T>(self.get_wallet_token()?.peek())
            .change_context(errors::ConnectorError::InvalidWalletToken { wallet_name })
    }

    fn get_encoded_wallet_token(&self) -> Result<String, Error> {
        match self {
            Self::GooglePay(_) => {
                let json_token: serde_json::Value =
                    self.get_wallet_token_as_json("Google Pay".to_owned())?;
                let token_as_vec = serde_json::to_vec(&json_token).change_context(
                    errors::ConnectorError::InvalidWalletToken {
                        wallet_name: "Google Pay".to_string(),
                    },
                )?;
                let encoded_token = consts::BASE64_ENGINE.encode(token_as_vec);
                Ok(encoded_token)
            }
            _ => Err(
                errors::ConnectorError::NotImplemented("SELECTED PAYMENT METHOD".to_owned()).into(),
            ),
        }
    }
}

pub trait ApplePay {
    fn get_applepay_decoded_payment_data(&self) -> Result<Secret<String>, Error>;
}

impl ApplePay for domain::ApplePayWalletData {
    fn get_applepay_decoded_payment_data(&self) -> Result<Secret<String>, Error> {
        let token = Secret::new(
            String::from_utf8(
                consts::BASE64_ENGINE
                    .decode(&self.payment_data)
                    .change_context(errors::ConnectorError::InvalidWalletToken {
                        wallet_name: "Apple Pay".to_string(),
                    })?,
            )
            .change_context(errors::ConnectorError::InvalidWalletToken {
                wallet_name: "Apple Pay".to_string(),
            })?,
        );
        Ok(token)
    }
}

pub trait ApplePayDecrypt {
    fn get_expiry_month(&self) -> Result<Secret<String>, Error>;
    fn get_four_digit_expiry_year(&self) -> Result<Secret<String>, Error>;
}

impl ApplePayDecrypt for Box<ApplePayPredecryptData> {
    fn get_four_digit_expiry_year(&self) -> Result<Secret<String>, Error> {
        Ok(Secret::new(format!(
            "20{}",
            self.application_expiration_date
                .get(0..2)
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?
        )))
    }

    fn get_expiry_month(&self) -> Result<Secret<String>, Error> {
        Ok(Secret::new(
            self.application_expiration_date
                .get(2..4)
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                .to_owned(),
        ))
    }
}

pub trait CryptoData {
    fn get_pay_currency(&self) -> Result<String, Error>;
}

impl CryptoData for domain::CryptoData {
    fn get_pay_currency(&self) -> Result<String, Error> {
        self.pay_currency
            .clone()
            .ok_or_else(missing_field_err("crypto_data.pay_currency"))
    }
}

pub trait PhoneDetailsData {
    fn get_number(&self) -> Result<Secret<String>, Error>;
    fn get_country_code(&self) -> Result<String, Error>;
    fn get_number_with_country_code(&self) -> Result<Secret<String>, Error>;
    fn get_number_with_hash_country_code(&self) -> Result<Secret<String>, Error>;
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
    fn get_number_with_country_code(&self) -> Result<Secret<String>, Error> {
        let number = self.get_number()?;
        let country_code = self.get_country_code()?;
        Ok(Secret::new(format!("{}{}", country_code, number.peek())))
    }
    fn get_number_with_hash_country_code(&self) -> Result<Secret<String>, Error> {
        let number = self.get_number()?;
        let country_code = self.get_country_code()?;
        let number_without_plus = country_code.trim_start_matches('+');
        Ok(Secret::new(format!(
            "{}#{}",
            number_without_plus,
            number.peek()
        )))
    }
}

pub trait AddressDetailsData {
    fn get_first_name(&self) -> Result<&Secret<String>, Error>;
    fn get_last_name(&self) -> Result<&Secret<String>, Error>;
    fn get_full_name(&self) -> Result<Secret<String>, Error>;
    fn get_line1(&self) -> Result<&Secret<String>, Error>;
    fn get_city(&self) -> Result<&String, Error>;
    fn get_line2(&self) -> Result<&Secret<String>, Error>;
    fn get_state(&self) -> Result<&Secret<String>, Error>;
    fn get_zip(&self) -> Result<&Secret<String>, Error>;
    fn get_country(&self) -> Result<&api_models::enums::CountryAlpha2, Error>;
    fn get_combined_address_line(&self) -> Result<Secret<String>, Error>;
    fn to_state_code(&self) -> Result<Secret<String>, Error>;
    fn to_state_code_option(&self) -> Result<Option<Secret<String>>, Error>;
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

    fn get_full_name(&self) -> Result<Secret<String>, Error> {
        let first_name = self.get_first_name()?.peek().to_owned();
        let last_name = self
            .get_last_name()
            .ok()
            .cloned()
            .unwrap_or(Secret::new("".to_string()));
        let last_name = last_name.peek();
        let full_name = format!("{} {}", first_name, last_name).trim().to_string();
        Ok(Secret::new(full_name))
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

    fn get_state(&self) -> Result<&Secret<String>, Error> {
        self.state
            .as_ref()
            .ok_or_else(missing_field_err("address.state"))
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

    fn get_country(&self) -> Result<&api_models::enums::CountryAlpha2, Error> {
        self.country
            .as_ref()
            .ok_or_else(missing_field_err("address.country"))
    }

    fn get_combined_address_line(&self) -> Result<Secret<String>, Error> {
        Ok(Secret::new(format!(
            "{},{}",
            self.get_line1()?.peek(),
            self.get_line2()?.peek()
        )))
    }
    fn to_state_code(&self) -> Result<Secret<String>, Error> {
        let country = self.get_country()?;
        let state = self.get_state()?;
        match country {
            api_models::enums::CountryAlpha2::US => Ok(Secret::new(
                UsStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::CA => Ok(Secret::new(
                CanadaStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            _ => Ok(state.clone()),
        }
    }
    fn to_state_code_option(&self) -> Result<Option<Secret<String>>, Error> {
        self.state
            .as_ref()
            .map(|_| self.to_state_code())
            .transpose()
    }
}

pub trait BankRedirectBillingData {
    fn get_billing_name(&self) -> Result<Secret<String>, Error>;
}

impl BankRedirectBillingData for domain::BankRedirectBilling {
    fn get_billing_name(&self) -> Result<Secret<String>, Error> {
        self.billing_name
            .clone()
            .ok_or_else(missing_field_err("billing_details.billing_name"))
    }
}

pub trait BankDirectDebitBillingData {
    fn get_billing_country(&self) -> Result<api_models::enums::CountryAlpha2, Error>;
}

impl BankDirectDebitBillingData for domain::BankDebitBilling {
    fn get_billing_country(&self) -> Result<api_models::enums::CountryAlpha2, Error> {
        self.address
            .as_ref()
            .and_then(|address| address.country)
            .ok_or_else(missing_field_err("billing_details.country"))
    }
}

pub trait MandateData {
    fn get_end_date(&self, format: date_time::DateFormat) -> Result<String, Error>;
    fn get_metadata(&self) -> Result<pii::SecretSerdeValue, Error>;
}

impl MandateData for payments::MandateAmountData {
    fn get_end_date(&self, format: date_time::DateFormat) -> Result<String, Error> {
        let date = self.end_date.ok_or_else(missing_field_err(
            "mandate_data.mandate_type.{multi_use|single_use}.end_date",
        ))?;
        date_time::format_date(date, format)
            .change_context(errors::ConnectorError::DateFormattingFailed)
    }
    fn get_metadata(&self) -> Result<pii::SecretSerdeValue, Error> {
        self.metadata.clone().ok_or_else(missing_field_err(
            "mandate_data.mandate_type.{multi_use|single_use}.metadata",
        ))
    }
}

pub trait RecurringMandateData {
    fn get_original_payment_amount(&self) -> Result<i64, Error>;
    fn get_original_payment_currency(&self) -> Result<diesel_models::enums::Currency, Error>;
}

impl RecurringMandateData for RecurringMandatePaymentData {
    fn get_original_payment_amount(&self) -> Result<i64, Error> {
        self.original_payment_authorized_amount
            .ok_or_else(missing_field_err("original_payment_authorized_amount"))
    }
    fn get_original_payment_currency(&self) -> Result<diesel_models::enums::Currency, Error> {
        self.original_payment_authorized_currency
            .ok_or_else(missing_field_err("original_payment_authorized_currency"))
    }
}

pub trait MandateReferenceData {
    fn get_connector_mandate_id(&self) -> Result<String, Error>;
}

impl MandateReferenceData for api_models::payments::ConnectorMandateReferenceId {
    fn get_connector_mandate_id(&self) -> Result<String, Error> {
        self.connector_mandate_id
            .clone()
            .ok_or_else(missing_field_err("mandate_id"))
    }
}

pub fn get_header_key_value<'a>(
    key: &str,
    headers: &'a actix_web::http::header::HeaderMap,
) -> CustomResult<&'a str, errors::ConnectorError> {
    get_header_field(headers.get(key))
}

pub fn get_http_header<'a>(
    key: &str,
    headers: &'a http::HeaderMap,
) -> CustomResult<&'a str, errors::ConnectorError> {
    get_header_field(headers.get(key))
}

fn get_header_field(
    field: Option<&http::HeaderValue>,
) -> CustomResult<&str, errors::ConnectorError> {
    field
        .map(|header_value| {
            header_value
                .to_str()
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
    json.parse_value(std::any::type_name::<T>()).switch()
}

pub fn to_connector_meta_from_secret<T>(
    connector_meta: Option<Secret<serde_json::Value>>,
) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let connector_meta_secret =
        connector_meta.ok_or_else(missing_field_err("connector_meta_data"))?;
    let json = connector_meta_secret.expose();
    json.parse_value(std::any::type_name::<T>()).switch()
}

impl common_utils::errors::ErrorSwitch<errors::ConnectorError> for errors::ParsingError {
    fn switch(&self) -> errors::ConnectorError {
        errors::ConnectorError::ParsingFailed
    }
}

pub fn base64_decode(data: String) -> Result<Vec<u8>, Error> {
    consts::BASE64_ENGINE
        .decode(data)
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
}

pub fn to_currency_base_unit_from_optional_amount(
    amount: Option<i64>,
    currency: diesel_models::enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    match amount {
        Some(a) => to_currency_base_unit(a, currency),
        _ => Err(errors::ConnectorError::MissingRequiredField {
            field_name: "amount",
        }
        .into()),
    }
}

pub fn get_amount_as_string(
    currency_unit: &types::api::CurrencyUnit,
    amount: i64,
    currency: diesel_models::enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let amount = match currency_unit {
        types::api::CurrencyUnit::Minor => amount.to_string(),
        types::api::CurrencyUnit::Base => to_currency_base_unit(amount, currency)?,
    };
    Ok(amount)
}

pub fn get_amount_as_f64(
    currency_unit: &types::api::CurrencyUnit,
    amount: i64,
    currency: diesel_models::enums::Currency,
) -> Result<f64, error_stack::Report<errors::ConnectorError>> {
    let amount = match currency_unit {
        types::api::CurrencyUnit::Base => to_currency_base_unit_asf64(amount, currency)?,
        types::api::CurrencyUnit::Minor => u32::try_from(amount)
            .change_context(errors::ConnectorError::ParsingFailed)?
            .into(),
    };
    Ok(amount)
}

pub fn to_currency_base_unit(
    amount: i64,
    currency: diesel_models::enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    currency
        .to_currency_base_unit(amount)
        .change_context(errors::ConnectorError::ParsingFailed)
}

pub fn to_currency_lower_unit(
    amount: String,
    currency: diesel_models::enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    currency
        .to_currency_lower_unit(amount)
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

pub fn construct_not_implemented_error_report(
    capture_method: enums::CaptureMethod,
    connector_name: &str,
) -> error_stack::Report<errors::ConnectorError> {
    errors::ConnectorError::NotImplemented(format!("{} for {}", capture_method, connector_name))
        .into()
}

pub fn construct_not_supported_error_report(
    capture_method: enums::CaptureMethod,
    connector_name: &'static str,
) -> error_stack::Report<errors::ConnectorError> {
    errors::ConnectorError::NotSupported {
        message: capture_method.to_string(),
        connector: connector_name,
    }
    .into()
}

pub fn to_currency_base_unit_with_zero_decimal_check(
    amount: i64,
    currency: diesel_models::enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    currency
        .to_currency_base_unit_with_zero_decimal_check(amount)
        .change_context(errors::ConnectorError::RequestEncodingFailed)
}

pub fn to_currency_base_unit_asf64(
    amount: i64,
    currency: diesel_models::enums::Currency,
) -> Result<f64, error_stack::Report<errors::ConnectorError>> {
    currency
        .to_currency_base_unit_asf64(amount)
        .change_context(errors::ConnectorError::ParsingFailed)
}

pub fn str_to_f32<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let float_value = value.parse::<f64>().map_err(|_| {
        serde::ser::Error::custom("Invalid string, cannot be converted to float value")
    })?;
    serializer.serialize_f64(float_value)
}

pub fn collect_values_by_removing_signature(
    value: &serde_json::Value,
    signature: &String,
) -> Vec<String> {
    match value {
        serde_json::Value::Null => vec!["null".to_owned()],
        serde_json::Value::Bool(b) => vec![b.to_string()],
        serde_json::Value::Number(n) => match n.as_f64() {
            Some(f) => vec![format!("{f:.2}")],
            None => vec![n.to_string()],
        },
        serde_json::Value::String(s) => {
            if signature == s {
                vec![]
            } else {
                vec![s.clone()]
            }
        }
        serde_json::Value::Array(arr) => arr
            .iter()
            .flat_map(|v| collect_values_by_removing_signature(v, signature))
            .collect(),
        serde_json::Value::Object(obj) => obj
            .values()
            .flat_map(|v| collect_values_by_removing_signature(v, signature))
            .collect(),
    }
}

pub fn collect_and_sort_values_by_removing_signature(
    value: &serde_json::Value,
    signature: &String,
) -> Vec<String> {
    let mut values = collect_values_by_removing_signature(value, signature);
    values.sort();
    values
}

#[inline]
pub fn get_webhook_merchant_secret_key(connector_label: &str, merchant_id: &str) -> String {
    format!("whsec_verification_{connector_label}_{merchant_id}")
}

impl ForeignTryFrom<String> for UsStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let binding = value.as_str().to_lowercase();
        let state = binding.as_str();
        match state {
            "alabama" => Ok(Self::AL),
            "alaska" => Ok(Self::AK),
            "american samoa" => Ok(Self::AS),
            "arizona" => Ok(Self::AZ),
            "arkansas" => Ok(Self::AR),
            "california" => Ok(Self::CA),
            "colorado" => Ok(Self::CO),
            "connecticut" => Ok(Self::CT),
            "delaware" => Ok(Self::DE),
            "district of columbia" | "columbia" => Ok(Self::DC),
            "federated states of micronesia" | "micronesia" => Ok(Self::FM),
            "florida" => Ok(Self::FL),
            "georgia" => Ok(Self::GA),
            "guam" => Ok(Self::GU),
            "hawaii" => Ok(Self::HI),
            "idaho" => Ok(Self::ID),
            "illinois" => Ok(Self::IL),
            "indiana" => Ok(Self::IN),
            "iowa" => Ok(Self::IA),
            "kansas" => Ok(Self::KS),
            "kentucky" => Ok(Self::KY),
            "louisiana" => Ok(Self::LA),
            "maine" => Ok(Self::ME),
            "marshall islands" => Ok(Self::MH),
            "maryland" => Ok(Self::MD),
            "massachusetts" => Ok(Self::MA),
            "michigan" => Ok(Self::MI),
            "minnesota" => Ok(Self::MN),
            "mississippi" => Ok(Self::MS),
            "missouri" => Ok(Self::MO),
            "montana" => Ok(Self::MT),
            "nebraska" => Ok(Self::NE),
            "nevada" => Ok(Self::NV),
            "new hampshire" => Ok(Self::NH),
            "new jersey" => Ok(Self::NJ),
            "new mexico" => Ok(Self::NM),
            "new york" => Ok(Self::NY),
            "north carolina" => Ok(Self::NC),
            "north dakota" => Ok(Self::ND),
            "northern mariana islands" => Ok(Self::MP),
            "ohio" => Ok(Self::OH),
            "oklahoma" => Ok(Self::OK),
            "oregon" => Ok(Self::OR),
            "palau" => Ok(Self::PW),
            "pennsylvania" => Ok(Self::PA),
            "puerto rico" => Ok(Self::PR),
            "rhode island" => Ok(Self::RI),
            "south carolina" => Ok(Self::SC),
            "south dakota" => Ok(Self::SD),
            "tennessee" => Ok(Self::TN),
            "texas" => Ok(Self::TX),
            "utah" => Ok(Self::UT),
            "vermont" => Ok(Self::VT),
            "virgin islands" => Ok(Self::VI),
            "virginia" => Ok(Self::VA),
            "washington" => Ok(Self::WA),
            "west virginia" => Ok(Self::WV),
            "wisconsin" => Ok(Self::WI),
            "wyoming" => Ok(Self::WY),
            _ => Err(errors::ConnectorError::InvalidDataFormat {
                field_name: "address.state",
            }
            .into()),
        }
    }
}

impl ForeignTryFrom<String> for CanadaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let binding = value.as_str().to_lowercase();
        let state = binding.as_str();
        match state {
            "alberta" => Ok(Self::AB),
            "british columbia" => Ok(Self::BC),
            "manitoba" => Ok(Self::MB),
            "new brunswick" => Ok(Self::NB),
            "newfoundland and labrador" | "newfoundland & labrador" => Ok(Self::NL),
            "northwest territories" => Ok(Self::NT),
            "nova scotia" => Ok(Self::NS),
            "nunavut" => Ok(Self::NU),
            "ontario" => Ok(Self::ON),
            "prince edward island" => Ok(Self::PE),
            "quebec" => Ok(Self::QC),
            "saskatchewan" => Ok(Self::SK),
            "yukon" => Ok(Self::YT),
            _ => Err(errors::ConnectorError::InvalidDataFormat {
                field_name: "address.state",
            }
            .into()),
        }
    }
}

pub trait ConnectorErrorTypeMapping {
    fn get_connector_error_type(
        &self,
        _error_code: String,
        _error_message: String,
    ) -> ConnectorErrorType {
        ConnectorErrorType::UnknownError
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorCodeAndMessage {
    pub error_code: String,
    pub error_message: String,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
//Priority of connector_error_type
pub enum ConnectorErrorType {
    UserError = 2,
    BusinessError = 3,
    TechnicalError = 4,
    UnknownError = 1,
}

//Gets the list of error_code_and_message, sorts based on the priority of error_type and gives most prior error
// This could be used in connectors where we get list of error_messages and have to choose one error_message
pub fn get_error_code_error_message_based_on_priority(
    connector: impl ConnectorErrorTypeMapping,
    error_list: Vec<ErrorCodeAndMessage>,
) -> Option<ErrorCodeAndMessage> {
    let error_type_list = error_list
        .iter()
        .map(|error| {
            connector
                .get_connector_error_type(error.error_code.clone(), error.error_message.clone())
        })
        .collect::<Vec<ConnectorErrorType>>();
    let mut error_zip_list = error_list
        .iter()
        .zip(error_type_list.iter())
        .collect::<Vec<(&ErrorCodeAndMessage, &ConnectorErrorType)>>();
    error_zip_list.sort_by_key(|&(_, error_type)| error_type);
    error_zip_list
        .first()
        .map(|&(error_code_message, _)| error_code_message)
        .cloned()
}

pub trait MultipleCaptureSyncResponse {
    fn get_connector_capture_id(&self) -> String;
    fn get_capture_attempt_status(&self) -> enums::AttemptStatus;
    fn is_capture_response(&self) -> bool;
    fn get_connector_reference_id(&self) -> Option<String> {
        None
    }
    fn get_amount_captured(&self) -> Option<i64>;
}

pub fn construct_captures_response_hashmap<T>(
    capture_sync_response_list: Vec<T>,
) -> HashMap<String, types::CaptureSyncResponse>
where
    T: MultipleCaptureSyncResponse,
{
    let mut hashmap = HashMap::new();
    capture_sync_response_list
        .into_iter()
        .for_each(|capture_sync_response| {
            let connector_capture_id = capture_sync_response.get_connector_capture_id();
            if capture_sync_response.is_capture_response() {
                hashmap.insert(
                    connector_capture_id.clone(),
                    types::CaptureSyncResponse::Success {
                        resource_id: ResponseId::ConnectorTransactionId(connector_capture_id),
                        status: capture_sync_response.get_capture_attempt_status(),
                        connector_response_reference_id: capture_sync_response
                            .get_connector_reference_id(),
                        amount: capture_sync_response.get_amount_captured(),
                    },
                );
            }
        });
    hashmap
}

pub fn is_manual_capture(capture_method: Option<enums::CaptureMethod>) -> bool {
    capture_method == Some(enums::CaptureMethod::Manual)
        || capture_method == Some(enums::CaptureMethod::ManualMultiple)
}

pub fn generate_random_bytes(length: usize) -> Vec<u8> {
    // returns random bytes of length n
    let mut rng = rand::thread_rng();
    (0..length).map(|_| rand::Rng::gen(&mut rng)).collect()
}

pub fn validate_currency(
    request_currency: types::storage::enums::Currency,
    merchant_config_currency: Option<types::storage::enums::Currency>,
) -> Result<(), errors::ConnectorError> {
    let merchant_config_currency =
        merchant_config_currency.ok_or(errors::ConnectorError::NoConnectorMetaData)?;
    if request_currency != merchant_config_currency {
        Err(errors::ConnectorError::NotSupported {
            message: format!(
                "currency {} is not supported for this merchant account",
                request_currency
            ),
            connector: "Braintree",
        })?
    }
    Ok(())
}

pub fn get_timestamp_in_milliseconds(datetime: &PrimitiveDateTime) -> i64 {
    let utc_datetime = datetime.assume_utc();
    utc_datetime.unix_timestamp() * 1000
}

#[cfg(feature = "frm")]
pub trait FraudCheckSaleRequest {
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error>;
}
#[cfg(feature = "frm")]
impl FraudCheckSaleRequest for fraud_check::FraudCheckSaleData {
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error> {
        self.order_details
            .clone()
            .ok_or_else(missing_field_err("order_details"))
    }
}

#[cfg(feature = "frm")]
pub trait FraudCheckCheckoutRequest {
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error>;
}
#[cfg(feature = "frm")]
impl FraudCheckCheckoutRequest for fraud_check::FraudCheckCheckoutData {
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error> {
        self.order_details
            .clone()
            .ok_or_else(missing_field_err("order_details"))
    }
}

#[cfg(feature = "frm")]
pub trait FraudCheckTransactionRequest {
    fn get_currency(&self) -> Result<storage_enums::Currency, Error>;
}
#[cfg(feature = "frm")]
impl FraudCheckTransactionRequest for fraud_check::FraudCheckTransactionData {
    fn get_currency(&self) -> Result<storage_enums::Currency, Error> {
        self.currency.ok_or_else(missing_field_err("currency"))
    }
}

#[cfg(feature = "frm")]
pub trait FraudCheckRecordReturnRequest {
    fn get_currency(&self) -> Result<storage_enums::Currency, Error>;
}
#[cfg(feature = "frm")]
impl FraudCheckRecordReturnRequest for fraud_check::FraudCheckRecordReturnData {
    fn get_currency(&self) -> Result<storage_enums::Currency, Error> {
        self.currency.ok_or_else(missing_field_err("currency"))
    }
}

pub trait AccessPaymentAttemptInfo {
    fn get_browser_info(
        &self,
    ) -> Result<Option<BrowserInformation>, error_stack::Report<ApiErrorResponse>>;
}

impl AccessPaymentAttemptInfo for PaymentAttempt {
    fn get_browser_info(
        &self,
    ) -> Result<Option<BrowserInformation>, error_stack::Report<ApiErrorResponse>> {
        self.browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })
    }
}

pub trait PaymentsAttemptData {
    fn get_browser_info(&self)
        -> Result<BrowserInformation, error_stack::Report<ApiErrorResponse>>;
}

impl PaymentsAttemptData for PaymentAttempt {
    fn get_browser_info(
        &self,
    ) -> Result<BrowserInformation, error_stack::Report<ApiErrorResponse>> {
        self.browser_info
            .clone()
            .ok_or(ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?
            .parse_value::<BrowserInformation>("BrowserInformation")
            .change_context(ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })
    }
}

#[cfg(feature = "frm")]
pub trait FrmTransactionRouterDataRequest {
    fn is_payment_successful(&self) -> Option<bool>;
}

#[cfg(feature = "frm")]
impl FrmTransactionRouterDataRequest for fraud_check::FrmTransactionRouterData {
    fn is_payment_successful(&self) -> Option<bool> {
        match self.status {
            storage_enums::AttemptStatus::AuthenticationFailed
            | storage_enums::AttemptStatus::RouterDeclined
            | storage_enums::AttemptStatus::AuthorizationFailed
            | storage_enums::AttemptStatus::Voided
            | storage_enums::AttemptStatus::CaptureFailed
            | storage_enums::AttemptStatus::Failure
            | storage_enums::AttemptStatus::AutoRefunded => Some(false),

            storage_enums::AttemptStatus::AuthenticationSuccessful
            | storage_enums::AttemptStatus::PartialChargedAndChargeable
            | storage_enums::AttemptStatus::Authorized
            | storage_enums::AttemptStatus::Charged => Some(true),

            storage_enums::AttemptStatus::Started
            | storage_enums::AttemptStatus::AuthenticationPending
            | storage_enums::AttemptStatus::Authorizing
            | storage_enums::AttemptStatus::CodInitiated
            | storage_enums::AttemptStatus::VoidInitiated
            | storage_enums::AttemptStatus::CaptureInitiated
            | storage_enums::AttemptStatus::VoidFailed
            | storage_enums::AttemptStatus::PartialCharged
            | storage_enums::AttemptStatus::Unresolved
            | storage_enums::AttemptStatus::Pending
            | storage_enums::AttemptStatus::PaymentMethodAwaited
            | storage_enums::AttemptStatus::ConfirmationAwaited
            | storage_enums::AttemptStatus::DeviceDataCollectionPending => None,
        }
    }
}

pub fn is_payment_failure(status: enums::AttemptStatus) -> bool {
    match status {
        common_enums::AttemptStatus::AuthenticationFailed
        | common_enums::AttemptStatus::AuthorizationFailed
        | common_enums::AttemptStatus::CaptureFailed
        | common_enums::AttemptStatus::VoidFailed
        | common_enums::AttemptStatus::Failure => true,
        common_enums::AttemptStatus::Started
        | common_enums::AttemptStatus::RouterDeclined
        | common_enums::AttemptStatus::AuthenticationPending
        | common_enums::AttemptStatus::AuthenticationSuccessful
        | common_enums::AttemptStatus::Authorized
        | common_enums::AttemptStatus::Charged
        | common_enums::AttemptStatus::Authorizing
        | common_enums::AttemptStatus::CodInitiated
        | common_enums::AttemptStatus::Voided
        | common_enums::AttemptStatus::VoidInitiated
        | common_enums::AttemptStatus::CaptureInitiated
        | common_enums::AttemptStatus::AutoRefunded
        | common_enums::AttemptStatus::PartialCharged
        | common_enums::AttemptStatus::PartialChargedAndChargeable
        | common_enums::AttemptStatus::Unresolved
        | common_enums::AttemptStatus::Pending
        | common_enums::AttemptStatus::PaymentMethodAwaited
        | common_enums::AttemptStatus::ConfirmationAwaited
        | common_enums::AttemptStatus::DeviceDataCollectionPending => false,
    }
}

pub fn is_refund_failure(status: enums::RefundStatus) -> bool {
    match status {
        common_enums::RefundStatus::Failure | common_enums::RefundStatus::TransactionFailure => {
            true
        }
        common_enums::RefundStatus::ManualReview
        | common_enums::RefundStatus::Pending
        | common_enums::RefundStatus::Success => false,
    }
}

impl
    From<(
        Option<String>,
        Option<String>,
        Option<String>,
        u16,
        Option<enums::AttemptStatus>,
        Option<String>,
    )> for types::ErrorResponse
{
    fn from(
        (code, message, reason, http_code, attempt_status, connector_transaction_id): (
            Option<String>,
            Option<String>,
            Option<String>,
            u16,
            Option<enums::AttemptStatus>,
            Option<String>,
        ),
    ) -> Self {
        Self {
            code: code.unwrap_or(consts::NO_ERROR_CODE.to_string()),
            message: message
                .clone()
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            reason,
            status_code: http_code,
            attempt_status,
            connector_transaction_id,
        }
    }
}

pub fn get_card_details(
    payment_method_data: domain::PaymentMethodData,
    connector_name: &'static str,
) -> Result<domain::payments::Card, errors::ConnectorError> {
    match payment_method_data {
        domain::PaymentMethodData::Card(details) => Ok(details),
        _ => Err(errors::ConnectorError::NotSupported {
            message: SELECTED_PAYMENT_METHOD.to_string(),
            connector: connector_name,
        })?,
    }
}

#[cfg(test)]
mod error_code_error_message_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    struct TestConnector;

    impl ConnectorErrorTypeMapping for TestConnector {
        fn get_connector_error_type(
            &self,
            error_code: String,
            error_message: String,
        ) -> ConnectorErrorType {
            match (error_code.as_str(), error_message.as_str()) {
                ("01", "INVALID_MERCHANT") => ConnectorErrorType::BusinessError,
                ("03", "INVALID_CVV") => ConnectorErrorType::UserError,
                ("04", "04") => ConnectorErrorType::TechnicalError,
                _ => ConnectorErrorType::UnknownError,
            }
        }
    }

    #[test]
    fn test_get_error_code_error_message_based_on_priority() {
        let error_code_message_list_unknown = vec![
            ErrorCodeAndMessage {
                error_code: "01".to_string(),
                error_message: "INVALID_MERCHANT".to_string(),
            },
            ErrorCodeAndMessage {
                error_code: "05".to_string(),
                error_message: "05".to_string(),
            },
            ErrorCodeAndMessage {
                error_code: "03".to_string(),
                error_message: "INVALID_CVV".to_string(),
            },
            ErrorCodeAndMessage {
                error_code: "04".to_string(),
                error_message: "04".to_string(),
            },
        ];
        let error_code_message_list_user = vec![
            ErrorCodeAndMessage {
                error_code: "01".to_string(),
                error_message: "INVALID_MERCHANT".to_string(),
            },
            ErrorCodeAndMessage {
                error_code: "03".to_string(),
                error_message: "INVALID_CVV".to_string(),
            },
        ];
        let error_code_error_message_unknown = get_error_code_error_message_based_on_priority(
            TestConnector,
            error_code_message_list_unknown,
        );
        let error_code_error_message_user = get_error_code_error_message_based_on_priority(
            TestConnector,
            error_code_message_list_user,
        );
        let error_code_error_message_none =
            get_error_code_error_message_based_on_priority(TestConnector, vec![]);
        assert_eq!(
            error_code_error_message_unknown,
            Some(ErrorCodeAndMessage {
                error_code: "05".to_string(),
                error_message: "05".to_string(),
            })
        );
        assert_eq!(
            error_code_error_message_user,
            Some(ErrorCodeAndMessage {
                error_code: "03".to_string(),
                error_message: "INVALID_CVV".to_string(),
            })
        );
        assert_eq!(error_code_error_message_none, None);
    }
}
