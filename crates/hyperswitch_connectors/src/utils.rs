use std::collections::{HashMap, HashSet};

use api_models::payments::{self, Address, AddressDetails, OrderDetailsWithAmount, PhoneDetails};
use base64::Engine;
use common_enums::{
    enums,
    enums::{AttemptStatus, CanadaStatesAbbreviation, FutureUsage, UsStatesAbbreviation},
};
use common_utils::{
    consts::BASE64_ENGINE,
    errors::{CustomResult, ReportSwitchExt},
    ext_traits::{OptionExt, StringExt, ValueExt},
    id_type,
    pii::{self, Email, IpAddress},
    types::{AmountConvertor, MinorUnit},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ApplePayPredecryptData, PaymentMethodToken, RecurringMandatePaymentData},
    router_request_types::{
        AuthenticationData, BrowserInformation, CompleteAuthorizeData,
        PaymentMethodTokenizationData, PaymentsAuthorizeData, PaymentsCancelData,
        PaymentsCaptureData, PaymentsPreProcessingData, PaymentsSyncData, RefundsData, ResponseId,
        SetupMandateRequestData,
    },
};
use hyperswitch_interfaces::{api, errors};
use image::Luma;
use masking::{ExposeInterface, PeekInterface, Secret};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serializer;

use crate::types::RefreshTokenRouterData;

type Error = error_stack::Report<errors::ConnectorError>;

pub(crate) fn construct_not_supported_error_report(
    capture_method: enums::CaptureMethod,
    connector_name: &'static str,
) -> error_stack::Report<errors::ConnectorError> {
    errors::ConnectorError::NotSupported {
        message: capture_method.to_string(),
        connector: connector_name,
    }
    .into()
}

pub(crate) fn to_currency_base_unit_with_zero_decimal_check(
    amount: i64,
    currency: enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    currency
        .to_currency_base_unit_with_zero_decimal_check(amount)
        .change_context(errors::ConnectorError::RequestEncodingFailed)
}

pub(crate) fn get_amount_as_string(
    currency_unit: &api::CurrencyUnit,
    amount: i64,
    currency: enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let amount = match currency_unit {
        api::CurrencyUnit::Minor => amount.to_string(),
        api::CurrencyUnit::Base => to_currency_base_unit(amount, currency)?,
    };
    Ok(amount)
}

pub(crate) fn to_currency_base_unit(
    amount: i64,
    currency: enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    currency
        .to_currency_base_unit(amount)
        .change_context(errors::ConnectorError::ParsingFailed)
}

pub(crate) fn get_amount_as_f64(
    currency_unit: &api::CurrencyUnit,
    amount: i64,
    currency: enums::Currency,
) -> Result<f64, error_stack::Report<errors::ConnectorError>> {
    let amount = match currency_unit {
        api::CurrencyUnit::Base => to_currency_base_unit_asf64(amount, currency)?,
        api::CurrencyUnit::Minor => u32::try_from(amount)
            .change_context(errors::ConnectorError::ParsingFailed)?
            .into(),
    };
    Ok(amount)
}

pub(crate) fn to_currency_base_unit_asf64(
    amount: i64,
    currency: enums::Currency,
) -> Result<f64, error_stack::Report<errors::ConnectorError>> {
    currency
        .to_currency_base_unit_asf64(amount)
        .change_context(errors::ConnectorError::ParsingFailed)
}

pub(crate) fn to_connector_meta_from_secret<T>(
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

pub(crate) fn missing_field_err(
    message: &'static str,
) -> Box<dyn Fn() -> error_stack::Report<errors::ConnectorError> + '_> {
    Box::new(move || {
        errors::ConnectorError::MissingRequiredField {
            field_name: message,
        }
        .into()
    })
}

pub(crate) fn construct_not_implemented_error_report(
    capture_method: enums::CaptureMethod,
    connector_name: &str,
) -> error_stack::Report<errors::ConnectorError> {
    errors::ConnectorError::NotImplemented(format!("{} for {}", capture_method, connector_name))
        .into()
}

pub(crate) fn str_to_f32<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let float_value = value.parse::<f64>().map_err(|_| {
        serde::ser::Error::custom("Invalid string, cannot be converted to float value")
    })?;
    serializer.serialize_f64(float_value)
}

pub(crate) const SELECTED_PAYMENT_METHOD: &str = "Selected payment method";

pub(crate) fn get_unimplemented_payment_method_error_message(connector: &str) -> String {
    format!("{} through {}", SELECTED_PAYMENT_METHOD, connector)
}

pub(crate) fn to_connector_meta<T>(connector_meta: Option<serde_json::Value>) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let json = connector_meta.ok_or_else(missing_field_err("connector_meta_data"))?;
    json.parse_value(std::any::type_name::<T>()).switch()
}

pub(crate) fn convert_amount<T>(
    amount_convertor: &dyn AmountConvertor<Output = T>,
    amount: MinorUnit,
    currency: enums::Currency,
) -> Result<T, error_stack::Report<errors::ConnectorError>> {
    amount_convertor
        .convert(amount, currency)
        .change_context(errors::ConnectorError::AmountConversionFailed)
}

pub(crate) fn convert_back_amount_to_minor_units<T>(
    amount_convertor: &dyn AmountConvertor<Output = T>,
    amount: T,
    currency: enums::Currency,
) -> Result<MinorUnit, error_stack::Report<errors::ConnectorError>> {
    amount_convertor
        .convert_back(amount, currency)
        .change_context(errors::ConnectorError::AmountConversionFailed)
}

pub(crate) fn is_payment_failure(status: AttemptStatus) -> bool {
    match status {
        AttemptStatus::AuthenticationFailed
        | AttemptStatus::AuthorizationFailed
        | AttemptStatus::CaptureFailed
        | AttemptStatus::VoidFailed
        | AttemptStatus::Failure => true,
        AttemptStatus::Started
        | AttemptStatus::RouterDeclined
        | AttemptStatus::AuthenticationPending
        | AttemptStatus::AuthenticationSuccessful
        | AttemptStatus::Authorized
        | AttemptStatus::Charged
        | AttemptStatus::Authorizing
        | AttemptStatus::CodInitiated
        | AttemptStatus::Voided
        | AttemptStatus::VoidInitiated
        | AttemptStatus::CaptureInitiated
        | AttemptStatus::AutoRefunded
        | AttemptStatus::PartialCharged
        | AttemptStatus::PartialChargedAndChargeable
        | AttemptStatus::Unresolved
        | AttemptStatus::Pending
        | AttemptStatus::PaymentMethodAwaited
        | AttemptStatus::ConfirmationAwaited
        | AttemptStatus::DeviceDataCollectionPending => false,
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
// TODO: Make all traits as `pub(crate) trait` once all connectors are moved.
pub trait RouterData {
    fn get_billing(&self) -> Result<&Address, Error>;
    fn get_billing_country(&self) -> Result<api_models::enums::CountryAlpha2, Error>;
    fn get_billing_phone(&self) -> Result<&PhoneDetails, Error>;
    fn get_description(&self) -> Result<String, Error>;
    fn get_return_url(&self) -> Result<String, Error>;
    fn get_billing_address(&self) -> Result<&AddressDetails, Error>;
    fn get_shipping_address(&self) -> Result<&AddressDetails, Error>;
    fn get_shipping_address_with_phone_number(&self) -> Result<&Address, Error>;
    fn get_connector_meta(&self) -> Result<pii::SecretSerdeValue, Error>;
    fn get_session_token(&self) -> Result<String, Error>;
    fn get_billing_first_name(&self) -> Result<Secret<String>, Error>;
    fn get_billing_full_name(&self) -> Result<Secret<String>, Error>;
    fn get_billing_last_name(&self) -> Result<Secret<String>, Error>;
    fn get_billing_line1(&self) -> Result<Secret<String>, Error>;
    fn get_billing_line2(&self) -> Result<Secret<String>, Error>;
    fn get_billing_zip(&self) -> Result<Secret<String>, Error>;
    fn get_billing_state(&self) -> Result<Secret<String>, Error>;
    fn get_billing_state_code(&self) -> Result<Secret<String>, Error>;
    fn get_billing_city(&self) -> Result<String, Error>;
    fn get_billing_email(&self) -> Result<Email, Error>;
    fn get_billing_phone_number(&self) -> Result<Secret<String>, Error>;
    fn to_connector_meta<T>(&self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned;
    fn is_three_ds(&self) -> bool;
    fn get_payment_method_token(&self) -> Result<PaymentMethodToken, Error>;
    fn get_customer_id(&self) -> Result<id_type::CustomerId, Error>;
    fn get_connector_customer_id(&self) -> Result<String, Error>;
    fn get_preprocessing_id(&self) -> Result<String, Error>;
    fn get_recurring_mandate_payment_data(&self) -> Result<RecurringMandatePaymentData, Error>;
    #[cfg(feature = "payouts")]
    fn get_payout_method_data(&self) -> Result<api_models::payouts::PayoutMethodData, Error>;
    #[cfg(feature = "payouts")]
    fn get_quote_id(&self) -> Result<String, Error>;

    fn get_optional_billing(&self) -> Option<&Address>;
    fn get_optional_shipping(&self) -> Option<&Address>;
    fn get_optional_shipping_line1(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_line2(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_city(&self) -> Option<String>;
    fn get_optional_shipping_country(&self) -> Option<enums::CountryAlpha2>;
    fn get_optional_shipping_zip(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_state(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_first_name(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_last_name(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_phone_number(&self) -> Option<Secret<String>>;
    fn get_optional_shipping_email(&self) -> Option<Email>;

    fn get_optional_billing_full_name(&self) -> Option<Secret<String>>;
    fn get_optional_billing_line1(&self) -> Option<Secret<String>>;
    fn get_optional_billing_line2(&self) -> Option<Secret<String>>;
    fn get_optional_billing_city(&self) -> Option<String>;
    fn get_optional_billing_country(&self) -> Option<enums::CountryAlpha2>;
    fn get_optional_billing_zip(&self) -> Option<Secret<String>>;
    fn get_optional_billing_state(&self) -> Option<Secret<String>>;
    fn get_optional_billing_state_2_digit(&self) -> Option<Secret<String>>;
    fn get_optional_billing_first_name(&self) -> Option<Secret<String>>;
    fn get_optional_billing_last_name(&self) -> Option<Secret<String>>;
    fn get_optional_billing_phone_number(&self) -> Option<Secret<String>>;
    fn get_optional_billing_email(&self) -> Option<Email>;
}

impl<Flow, Request, Response> RouterData
    for hyperswitch_domain_models::router_data::RouterData<Flow, Request, Response>
{
    fn get_billing(&self) -> Result<&Address, Error> {
        self.address
            .get_payment_method_billing()
            .ok_or_else(missing_field_err("billing"))
    }

    fn get_billing_country(&self) -> Result<api_models::enums::CountryAlpha2, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|a| a.address.as_ref())
            .and_then(|ad| ad.country)
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.country",
            ))
    }

    fn get_billing_phone(&self) -> Result<&PhoneDetails, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|a| a.phone.as_ref())
            .ok_or_else(missing_field_err("billing.phone"))
    }

    fn get_optional_billing(&self) -> Option<&Address> {
        self.address.get_payment_method_billing()
    }

    fn get_optional_shipping(&self) -> Option<&Address> {
        self.address.get_shipping()
    }

    fn get_optional_shipping_first_name(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.first_name)
        })
    }

    fn get_optional_shipping_last_name(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.last_name)
        })
    }

    fn get_optional_shipping_line1(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.line1)
        })
    }

    fn get_optional_shipping_line2(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.line2)
        })
    }

    fn get_optional_shipping_city(&self) -> Option<String> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.city)
        })
    }

    fn get_optional_shipping_state(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.state)
        })
    }

    fn get_optional_shipping_country(&self) -> Option<enums::CountryAlpha2> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.country)
        })
    }

    fn get_optional_shipping_zip(&self) -> Option<Secret<String>> {
        self.address.get_shipping().and_then(|shipping_address| {
            shipping_address
                .clone()
                .address
                .and_then(|shipping_details| shipping_details.zip)
        })
    }

    fn get_optional_shipping_email(&self) -> Option<Email> {
        self.address
            .get_shipping()
            .and_then(|shipping_address| shipping_address.clone().email)
    }

    fn get_optional_shipping_phone_number(&self) -> Option<Secret<String>> {
        self.address
            .get_shipping()
            .and_then(|shipping_address| shipping_address.clone().phone)
            .and_then(|phone_details| phone_details.get_number_with_country_code().ok())
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
    fn get_billing_address(&self) -> Result<&AddressDetails, Error> {
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
                    .and_then(|billing_details| billing_details.first_name.clone())
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.first_name",
            ))
    }

    fn get_billing_full_name(&self) -> Result<Secret<String>, Error> {
        self.get_optional_billing()
            .and_then(|billing_details| billing_details.address.as_ref())
            .and_then(|billing_address| billing_address.get_optional_full_name())
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.first_name",
            ))
    }

    fn get_billing_last_name(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.last_name.clone())
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.last_name",
            ))
    }

    fn get_billing_line1(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.line1.clone())
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.line1",
            ))
    }
    fn get_billing_line2(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.line2.clone())
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.line2",
            ))
    }
    fn get_billing_zip(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.zip.clone())
            })
            .ok_or_else(missing_field_err("payment_method_data.billing.address.zip"))
    }
    fn get_billing_state(&self) -> Result<Secret<String>, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.state.clone())
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.state",
            ))
    }
    fn get_billing_state_code(&self) -> Result<Secret<String>, Error> {
        let country = self.get_billing_country()?;
        let state = self.get_billing_state()?;
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
    fn get_billing_city(&self) -> Result<String, Error> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.city)
            })
            .ok_or_else(missing_field_err(
                "payment_method_data.billing.address.city",
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
                    .and_then(|billing_details| billing_details.line1)
            })
    }

    fn get_optional_billing_line2(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.line2)
            })
    }

    fn get_optional_billing_city(&self) -> Option<String> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.city)
            })
    }

    fn get_optional_billing_country(&self) -> Option<enums::CountryAlpha2> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.country)
            })
    }

    fn get_optional_billing_zip(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.zip)
            })
    }

    fn get_optional_billing_state(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.state)
            })
    }

    fn get_optional_billing_state_2_digit(&self) -> Option<Secret<String>> {
        self.get_optional_billing_state().and_then(|state| {
            if state.clone().expose().len() != 2 {
                None
            } else {
                Some(state)
            }
        })
    }

    fn get_optional_billing_first_name(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.first_name)
            })
    }

    fn get_optional_billing_last_name(&self) -> Option<Secret<String>> {
        self.address
            .get_payment_method_billing()
            .and_then(|billing_address| {
                billing_address
                    .clone()
                    .address
                    .and_then(|billing_details| billing_details.last_name)
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
        matches!(self.auth_type, enums::AuthenticationType::ThreeDs)
    }

    fn get_shipping_address(&self) -> Result<&AddressDetails, Error> {
        self.address
            .get_shipping()
            .and_then(|a| a.address.as_ref())
            .ok_or_else(missing_field_err("shipping.address"))
    }

    fn get_shipping_address_with_phone_number(&self) -> Result<&Address, Error> {
        self.address
            .get_shipping()
            .ok_or_else(missing_field_err("shipping"))
    }

    fn get_payment_method_token(&self) -> Result<PaymentMethodToken, Error> {
        self.payment_method_token
            .clone()
            .ok_or_else(missing_field_err("payment_method_token"))
    }
    fn get_customer_id(&self) -> Result<id_type::CustomerId, Error> {
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
    fn get_payout_method_data(&self) -> Result<api_models::payouts::PayoutMethodData, Error> {
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
    fn get_expiry_date_as_mmyy(&self) -> Result<Secret<String>, errors::ConnectorError>;
    fn get_expiry_month_as_i8(&self) -> Result<Secret<i8>, Error>;
    fn get_expiry_year_as_i32(&self) -> Result<Secret<i32>, Error>;
}

impl CardData for Card {
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
    fn get_expiry_date_as_mmyy(&self) -> Result<Secret<String>, errors::ConnectorError> {
        let year = self.get_card_expiry_year_2_digit()?.expose();
        let month = self.card_exp_month.clone().expose();
        Ok(Secret::new(format!("{month}{year}")))
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

pub trait AccessTokenRequestInfo {
    fn get_request_id(&self) -> Result<Secret<String>, Error>;
}

impl AccessTokenRequestInfo for RefreshTokenRouterData {
    fn get_request_id(&self) -> Result<Secret<String>, Error> {
        self.request
            .id
            .clone()
            .ok_or_else(missing_field_err("request.id"))
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
    fn to_state_code_as_optional(&self) -> Result<Option<Secret<String>>, Error>;
    fn get_optional_city(&self) -> Option<String>;
    fn get_optional_line1(&self) -> Option<Secret<String>>;
    fn get_optional_line2(&self) -> Option<Secret<String>>;
}

impl AddressDetailsData for AddressDetails {
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
    fn to_state_code_as_optional(&self) -> Result<Option<Secret<String>>, Error> {
        self.state
            .as_ref()
            .map(|state| {
                if state.peek().len() == 2 {
                    Ok(state.to_owned())
                } else {
                    self.to_state_code()
                }
            })
            .transpose()
    }

    fn get_optional_city(&self) -> Option<String> {
        self.city.clone()
    }

    fn get_optional_line1(&self) -> Option<Secret<String>> {
        self.line1.clone()
    }

    fn get_optional_line2(&self) -> Option<Secret<String>> {
        self.line2.clone()
    }
}

pub trait PhoneDetailsData {
    fn get_number(&self) -> Result<Secret<String>, Error>;
    fn get_country_code(&self) -> Result<String, Error>;
    fn get_number_with_country_code(&self) -> Result<Secret<String>, Error>;
    fn get_number_with_hash_country_code(&self) -> Result<Secret<String>, Error>;
    fn extract_country_code(&self) -> Result<String, Error>;
}

impl PhoneDetailsData for PhoneDetails {
    fn get_country_code(&self) -> Result<String, Error> {
        self.country_code
            .clone()
            .ok_or_else(missing_field_err("billing.phone.country_code"))
    }
    fn extract_country_code(&self) -> Result<String, Error> {
        self.get_country_code()
            .map(|cc| cc.trim_start_matches('+').to_string())
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

pub trait PaymentsAuthorizeRequestData {
    fn get_optional_language_from_browser_info(&self) -> Option<String>;
    fn is_auto_capture(&self) -> Result<bool, Error>;
    fn get_email(&self) -> Result<Email, Error>;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error>;
    fn get_card(&self) -> Result<Card, Error>;
    fn get_return_url(&self) -> Result<String, Error>;
    fn connector_mandate_id(&self) -> Option<String>;
    fn is_mandate_payment(&self) -> bool;
    fn is_customer_initiated_mandate_payment(&self) -> bool;
    fn get_webhook_url(&self) -> Result<String, Error>;
    fn get_router_return_url(&self) -> Result<String, Error>;
    fn is_wallet(&self) -> bool;
    fn is_card(&self) -> bool;
    fn get_payment_method_type(&self) -> Result<enums::PaymentMethodType, Error>;
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

impl PaymentsAuthorizeRequestData for PaymentsAuthorizeData {
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(enums::CaptureMethod::Automatic) | None => Ok(true),
            Some(enums::CaptureMethod::Manual) => Ok(false),
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
    fn get_optional_language_from_browser_info(&self) -> Option<String> {
        self.browser_info
            .clone()
            .and_then(|browser_info| browser_info.language)
    }

    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error> {
        self.order_details
            .clone()
            .ok_or_else(missing_field_err("order_details"))
    }

    fn get_card(&self) -> Result<Card, Error> {
        match self.payment_method_data.clone() {
            PaymentMethodData::Card(card) => Ok(card),
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
                Some(payments::MandateReferenceId::ConnectorMandateId(connector_mandate_ids)) => {
                    connector_mandate_ids.connector_mandate_id.clone()
                }
                Some(payments::MandateReferenceId::NetworkMandateId(_))
                | None
                | Some(payments::MandateReferenceId::NetworkTokenWithNTI(_)) => None,
            })
    }
    fn is_mandate_payment(&self) -> bool {
        ((self.customer_acceptance.is_some() || self.setup_mandate_details.is_some())
            && self.setup_future_usage.map_or(false, |setup_future_usage| {
                setup_future_usage == FutureUsage::OffSession
            }))
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
        matches!(self.payment_method_data, PaymentMethodData::Wallet(_))
    }
    fn is_card(&self) -> bool {
        matches!(self.payment_method_data, PaymentMethodData::Card(_))
    }

    fn get_payment_method_type(&self) -> Result<enums::PaymentMethodType, Error> {
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
            .map(|surcharge_details| surcharge_details.original_amount.get_amount_as_i64())
            .unwrap_or(self.amount)
    }
    fn get_surcharge_amount(&self) -> Option<i64> {
        self.surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.surcharge_amount.get_amount_as_i64())
    }
    fn get_tax_on_surcharge_amount(&self) -> Option<i64> {
        self.surcharge_details.as_ref().map(|surcharge_details| {
            surcharge_details
                .tax_on_surcharge_amount
                .get_amount_as_i64()
        })
    }
    fn get_total_surcharge_amount(&self) -> Option<i64> {
        self.surcharge_details.as_ref().map(|surcharge_details| {
            surcharge_details
                .get_total_surcharge_amount()
                .get_amount_as_i64()
        })
    }

    fn is_customer_initiated_mandate_payment(&self) -> bool {
        (self.customer_acceptance.is_some() || self.setup_mandate_details.is_some())
            && self.setup_future_usage.map_or(false, |setup_future_usage| {
                setup_future_usage == FutureUsage::OffSession
            })
    }

    fn get_metadata_as_object(&self) -> Option<pii::SecretSerdeValue> {
        self.metadata.clone().and_then(|meta_data| match meta_data {
            serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_)
            | serde_json::Value::Array(_) => None,
            serde_json::Value::Object(_) => Some(meta_data.into()),
        })
    }

    fn get_authentication_data(&self) -> Result<AuthenticationData, Error> {
        self.authentication_data
            .clone()
            .ok_or_else(missing_field_err("authentication_data"))
    }
}

pub trait PaymentsCaptureRequestData {
    fn get_optional_language_from_browser_info(&self) -> Option<String>;
    fn is_multiple_capture(&self) -> bool;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
}

impl PaymentsCaptureRequestData for PaymentsCaptureData {
    fn is_multiple_capture(&self) -> bool {
        self.multiple_capture_data.is_some()
    }
    fn get_browser_info(&self) -> Result<BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
    fn get_optional_language_from_browser_info(&self) -> Option<String> {
        self.browser_info
            .clone()
            .and_then(|browser_info| browser_info.language)
    }
}

pub trait PaymentsSyncRequestData {
    fn is_auto_capture(&self) -> Result<bool, Error>;
    fn get_connector_transaction_id(&self) -> CustomResult<String, errors::ConnectorError>;
}

impl PaymentsSyncRequestData for PaymentsSyncData {
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(enums::CaptureMethod::Automatic) | None => Ok(true),
            Some(enums::CaptureMethod::Manual) => Ok(false),
            Some(_) => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
        }
    }
    fn get_connector_transaction_id(&self) -> CustomResult<String, errors::ConnectorError> {
        match self.connector_transaction_id.clone() {
            ResponseId::ConnectorTransactionId(txn_id) => Ok(txn_id),
            _ => Err(
                common_utils::errors::ValidationError::IncorrectValueProvided {
                    field_name: "connector_transaction_id",
                },
            )
            .attach_printable("Expected connector transaction ID not found")
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
        }
    }
}

pub trait PaymentsCancelRequestData {
    fn get_optional_language_from_browser_info(&self) -> Option<String>;
    fn get_amount(&self) -> Result<i64, Error>;
    fn get_currency(&self) -> Result<enums::Currency, Error>;
    fn get_cancellation_reason(&self) -> Result<String, Error>;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
}

impl PaymentsCancelRequestData for PaymentsCancelData {
    fn get_amount(&self) -> Result<i64, Error> {
        self.amount.ok_or_else(missing_field_err("amount"))
    }
    fn get_currency(&self) -> Result<enums::Currency, Error> {
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
    fn get_optional_language_from_browser_info(&self) -> Option<String> {
        self.browser_info
            .clone()
            .and_then(|browser_info| browser_info.language)
    }
}

pub trait RefundsRequestData {
    fn get_optional_language_from_browser_info(&self) -> Option<String>;
    fn get_connector_refund_id(&self) -> Result<String, Error>;
    fn get_webhook_url(&self) -> Result<String, Error>;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
}

impl RefundsRequestData for RefundsData {
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
    fn get_optional_language_from_browser_info(&self) -> Option<String> {
        self.browser_info
            .clone()
            .and_then(|browser_info| browser_info.language)
    }
}

pub trait PaymentsSetupMandateRequestData {
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
    fn get_email(&self) -> Result<Email, Error>;
    fn is_card(&self) -> bool;
}

impl PaymentsSetupMandateRequestData for SetupMandateRequestData {
    fn get_browser_info(&self) -> Result<BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
    fn get_email(&self) -> Result<Email, Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }
    fn is_card(&self) -> bool {
        matches!(self.payment_method_data, PaymentMethodData::Card(_))
    }
}

pub trait PaymentMethodTokenizationRequestData {
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
}

impl PaymentMethodTokenizationRequestData for PaymentMethodTokenizationData {
    fn get_browser_info(&self) -> Result<BrowserInformation, Error> {
        self.browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))
    }
}

pub trait PaymentsCompleteAuthorizeRequestData {
    fn is_auto_capture(&self) -> Result<bool, Error>;
    fn get_email(&self) -> Result<Email, Error>;
    fn get_redirect_response_payload(&self) -> Result<pii::SecretSerdeValue, Error>;
    fn get_complete_authorize_url(&self) -> Result<String, Error>;
    fn is_mandate_payment(&self) -> bool;
}

impl PaymentsCompleteAuthorizeRequestData for CompleteAuthorizeData {
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(enums::CaptureMethod::Automatic) | None => Ok(true),
            Some(enums::CaptureMethod::Manual) => Ok(false),
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
    fn is_mandate_payment(&self) -> bool {
        ((self.customer_acceptance.is_some() || self.setup_mandate_details.is_some())
            && self.setup_future_usage.map_or(false, |setup_future_usage| {
                setup_future_usage == FutureUsage::OffSession
            }))
            || self
                .mandate_id
                .as_ref()
                .and_then(|mandate_ids| mandate_ids.mandate_reference_id.as_ref())
                .is_some()
    }
}

pub trait PaymentsPreProcessingRequestData {
    fn is_auto_capture(&self) -> Result<bool, Error>;
}

impl PaymentsPreProcessingRequestData for PaymentsPreProcessingData {
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(enums::CaptureMethod::Automatic) | None => Ok(true),
            Some(enums::CaptureMethod::Manual) => Ok(false),
            Some(enums::CaptureMethod::ManualMultiple) | Some(enums::CaptureMethod::Scheduled) => {
                Err(errors::ConnectorError::CaptureMethodNotSupported.into())
            }
        }
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

pub trait CryptoData {
    fn get_pay_currency(&self) -> Result<String, Error>;
}

impl CryptoData for hyperswitch_domain_models::payment_method_data::CryptoData {
    fn get_pay_currency(&self) -> Result<String, Error> {
        self.pay_currency
            .clone()
            .ok_or_else(missing_field_err("crypto_data.pay_currency"))
    }
}

#[macro_export]
macro_rules! unimplemented_payment_method {
    ($payment_method:expr, $connector:expr) => {
        errors::ConnectorError::NotImplemented(format!(
            "{} through {}",
            $payment_method, $connector
        ))
    };
    ($payment_method:expr, $flow:expr, $connector:expr) => {
        errors::ConnectorError::NotImplemented(format!(
            "{} {} through {}",
            $payment_method, $flow, $connector
        ))
    };
}

impl ForeignTryFrom<String> for UsStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "UsStatesAbbreviation");

        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
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
    }
}

impl ForeignTryFrom<String> for CanadaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "CanadaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
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
    }
}

pub trait ForeignTryFrom<F>: Sized {
    type Error;

    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

#[derive(Debug)]
pub struct QrImage {
    pub data: String,
}

// Qr Image data source starts with this string
// The base64 image data will be appended to it to image data source
pub(crate) const QR_IMAGE_DATA_SOURCE_STRING: &str = "data:image/png;base64";

impl QrImage {
    pub fn new_from_data(
        data: String,
    ) -> Result<Self, error_stack::Report<common_utils::errors::QrCodeError>> {
        let qr_code = qrcode::QrCode::new(data.as_bytes())
            .change_context(common_utils::errors::QrCodeError::FailedToCreateQrCode)?;

        let qrcode_image_buffer = qr_code.render::<Luma<u8>>().build();
        let qrcode_dynamic_image = image::DynamicImage::ImageLuma8(qrcode_image_buffer);

        let mut image_bytes = std::io::BufWriter::new(std::io::Cursor::new(Vec::new()));

        // Encodes qrcode_dynamic_image and write it to image_bytes
        let _ = qrcode_dynamic_image.write_to(&mut image_bytes, image::ImageFormat::Png);

        let image_data_source = format!(
            "{},{}",
            QR_IMAGE_DATA_SOURCE_STRING,
            BASE64_ENGINE.encode(image_bytes.buffer())
        );
        Ok(Self {
            data: image_data_source,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::utils;
    #[test]
    fn test_image_data_source_url() {
        let qr_image_data_source_url = utils::QrImage::new_from_data("Hyperswitch".to_string());
        assert!(qr_image_data_source_url.is_ok());
    }
}

pub fn is_mandate_supported(
    selected_pmd: PaymentMethodData,
    payment_method_type: Option<enums::PaymentMethodType>,
    mandate_implemented_pmds: HashSet<PaymentMethodDataType>,
    connector: &'static str,
) -> Result<(), Error> {
    if mandate_implemented_pmds.contains(&PaymentMethodDataType::from(selected_pmd.clone())) {
        Ok(())
    } else {
        match payment_method_type {
            Some(pm_type) => Err(errors::ConnectorError::NotSupported {
                message: format!("{} mandate payment", pm_type),
                connector,
            }
            .into()),
            None => Err(errors::ConnectorError::NotSupported {
                message: "mandate payment".to_string(),
                connector,
            }
            .into()),
        }
    }
}

#[derive(Debug, strum::Display, Eq, PartialEq, Hash)]
pub enum PaymentMethodDataType {
    Card,
    Knet,
    Benefit,
    MomoAtm,
    CardRedirect,
    AliPayQr,
    AliPayRedirect,
    AliPayHkRedirect,
    MomoRedirect,
    KakaoPayRedirect,
    GoPayRedirect,
    GcashRedirect,
    ApplePay,
    ApplePayRedirect,
    ApplePayThirdPartySdk,
    DanaRedirect,
    DuitNow,
    GooglePay,
    GooglePayRedirect,
    GooglePayThirdPartySdk,
    MbWayRedirect,
    MobilePayRedirect,
    PaypalRedirect,
    PaypalSdk,
    Paze,
    SamsungPay,
    TwintRedirect,
    VippsRedirect,
    TouchNGoRedirect,
    WeChatPayRedirect,
    WeChatPayQr,
    CashappQr,
    SwishQr,
    KlarnaRedirect,
    KlarnaSdk,
    AffirmRedirect,
    AfterpayClearpayRedirect,
    PayBrightRedirect,
    WalleyRedirect,
    AlmaRedirect,
    AtomeRedirect,
    BancontactCard,
    Bizum,
    Blik,
    Eps,
    Giropay,
    Ideal,
    Interac,
    LocalBankRedirect,
    OnlineBankingCzechRepublic,
    OnlineBankingFinland,
    OnlineBankingPoland,
    OnlineBankingSlovakia,
    OpenBankingUk,
    Przelewy24,
    Sofort,
    Trustly,
    OnlineBankingFpx,
    OnlineBankingThailand,
    AchBankDebit,
    SepaBankDebit,
    BecsBankDebit,
    BacsBankDebit,
    AchBankTransfer,
    SepaBankTransfer,
    BacsBankTransfer,
    MultibancoBankTransfer,
    PermataBankTransfer,
    BcaBankTransfer,
    BniVaBankTransfer,
    BriVaBankTransfer,
    CimbVaBankTransfer,
    DanamonVaBankTransfer,
    MandiriVaBankTransfer,
    Pix,
    Pse,
    Crypto,
    MandatePayment,
    Reward,
    Upi,
    Boleto,
    Efecty,
    PagoEfectivo,
    RedCompra,
    RedPagos,
    Alfamart,
    Indomaret,
    Oxxo,
    SevenEleven,
    Lawson,
    MiniStop,
    FamilyMart,
    Seicomart,
    PayEasy,
    Givex,
    PaySafeCar,
    CardToken,
    LocalBankTransfer,
    Mifinity,
    Fps,
    PromptPay,
    VietQr,
    OpenBanking,
    NetworkToken,
    NetworkTransactionIdAndCardDetails,
}

impl From<PaymentMethodData> for PaymentMethodDataType {
    fn from(pm_data: PaymentMethodData) -> Self {
        match pm_data {
            PaymentMethodData::Card(_) => Self::Card,
            PaymentMethodData::NetworkToken(_) => Self::NetworkToken,
            PaymentMethodData::CardDetailsForNetworkTransactionId(_) => Self::NetworkTransactionIdAndCardDetails,
            PaymentMethodData::CardRedirect(card_redirect_data) => {
                match card_redirect_data {
                   hyperswitch_domain_models::payment_method_data::CardRedirectData::Knet {} => Self::Knet,
                   hyperswitch_domain_models::payment_method_data::CardRedirectData::Benefit {} => Self::Benefit,
                   hyperswitch_domain_models::payment_method_data::CardRedirectData::MomoAtm {} => Self::MomoAtm,
                   hyperswitch_domain_models::payment_method_data::CardRedirectData::CardRedirect {} => Self::CardRedirect,
                }
            }
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                 hyperswitch_domain_models::payment_method_data::WalletData::AliPayQr(_) => Self::AliPayQr,
                 hyperswitch_domain_models::payment_method_data::WalletData::AliPayRedirect(_) => Self::AliPayRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::AliPayHkRedirect(_) => Self::AliPayHkRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::MomoRedirect(_) => Self::MomoRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::KakaoPayRedirect(_) => Self::KakaoPayRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::GoPayRedirect(_) => Self::GoPayRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::GcashRedirect(_) => Self::GcashRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::ApplePay(_) => Self::ApplePay,
                 hyperswitch_domain_models::payment_method_data::WalletData::ApplePayRedirect(_) => Self::ApplePayRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::ApplePayThirdPartySdk(_) => {
                    Self::ApplePayThirdPartySdk
                }
                 hyperswitch_domain_models::payment_method_data::WalletData::DanaRedirect {} => Self::DanaRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::GooglePay(_) => Self::GooglePay,
                 hyperswitch_domain_models::payment_method_data::WalletData::GooglePayRedirect(_) => Self::GooglePayRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::GooglePayThirdPartySdk(_) => {
                    Self::GooglePayThirdPartySdk
                }
                 hyperswitch_domain_models::payment_method_data::WalletData::MbWayRedirect(_) => Self::MbWayRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::MobilePayRedirect(_) => Self::MobilePayRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::PaypalRedirect(_) => Self::PaypalRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::PaypalSdk(_) => Self::PaypalSdk,
                 hyperswitch_domain_models::payment_method_data::WalletData::Paze(_) => Self::Paze,
                 hyperswitch_domain_models::payment_method_data::WalletData::SamsungPay(_) => Self::SamsungPay,
                 hyperswitch_domain_models::payment_method_data::WalletData::TwintRedirect {} => Self::TwintRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::VippsRedirect {} => Self::VippsRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::TouchNGoRedirect(_) => Self::TouchNGoRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::WeChatPayRedirect(_) => Self::WeChatPayRedirect,
                 hyperswitch_domain_models::payment_method_data::WalletData::WeChatPayQr(_) => Self::WeChatPayQr,
                 hyperswitch_domain_models::payment_method_data::WalletData::CashappQr(_) => Self::CashappQr,
                 hyperswitch_domain_models::payment_method_data::WalletData::SwishQr(_) => Self::SwishQr,
                 hyperswitch_domain_models::payment_method_data::WalletData::Mifinity(_) => Self::Mifinity,
            },
            PaymentMethodData::PayLater(pay_later_data) => match pay_later_data {
                 hyperswitch_domain_models::payment_method_data::PayLaterData::KlarnaRedirect { .. } => Self::KlarnaRedirect,
                 hyperswitch_domain_models::payment_method_data::PayLaterData::KlarnaSdk { .. } => Self::KlarnaSdk,
                 hyperswitch_domain_models::payment_method_data::PayLaterData::AffirmRedirect {} => Self::AffirmRedirect,
                 hyperswitch_domain_models::payment_method_data::PayLaterData::AfterpayClearpayRedirect { .. } => {
                    Self::AfterpayClearpayRedirect
                }
                 hyperswitch_domain_models::payment_method_data::PayLaterData::PayBrightRedirect {} => Self::PayBrightRedirect,
                 hyperswitch_domain_models::payment_method_data::PayLaterData::WalleyRedirect {} => Self::WalleyRedirect,
                 hyperswitch_domain_models::payment_method_data::PayLaterData::AlmaRedirect {} => Self::AlmaRedirect,
                 hyperswitch_domain_models::payment_method_data::PayLaterData::AtomeRedirect {} => Self::AtomeRedirect,
            },
            PaymentMethodData::BankRedirect(bank_redirect_data) => {
                match bank_redirect_data {
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::BancontactCard { .. } => {
                        Self::BancontactCard
                    }
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::Bizum {} => Self::Bizum,
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::Blik { .. } => Self::Blik,
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::Eps { .. } => Self::Eps,
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::Giropay { .. } => Self::Giropay,
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::Ideal { .. } => Self::Ideal,
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::Interac { .. } => Self::Interac,
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::OnlineBankingCzechRepublic { .. } => {
                        Self::OnlineBankingCzechRepublic
                    }
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::OnlineBankingFinland { .. } => {
                        Self::OnlineBankingFinland
                    }
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::OnlineBankingPoland { .. } => {
                        Self::OnlineBankingPoland
                    }
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::OnlineBankingSlovakia { .. } => {
                        Self::OnlineBankingSlovakia
                    }
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::OpenBankingUk { .. } => Self::OpenBankingUk,
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::Przelewy24 { .. } => Self::Przelewy24,
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::Sofort { .. } => Self::Sofort,
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::Trustly { .. } => Self::Trustly,
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::OnlineBankingFpx { .. } => {
                        Self::OnlineBankingFpx
                    }
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::OnlineBankingThailand { .. } => {
                        Self::OnlineBankingThailand
                    }
                     hyperswitch_domain_models::payment_method_data::BankRedirectData::LocalBankRedirect { } => {
                        Self::LocalBankRedirect
                    }
                }
            }
            PaymentMethodData::BankDebit(bank_debit_data) => {
                match bank_debit_data {
                     hyperswitch_domain_models::payment_method_data::BankDebitData::AchBankDebit { .. } => Self::AchBankDebit,
                     hyperswitch_domain_models::payment_method_data::BankDebitData::SepaBankDebit { .. } => Self::SepaBankDebit,
                     hyperswitch_domain_models::payment_method_data::BankDebitData::BecsBankDebit { .. } => Self::BecsBankDebit,
                     hyperswitch_domain_models::payment_method_data::BankDebitData::BacsBankDebit { .. } => Self::BacsBankDebit,
                }
            }
            PaymentMethodData::BankTransfer(bank_transfer_data) => {
                match *bank_transfer_data {
                     hyperswitch_domain_models::payment_method_data::BankTransferData::AchBankTransfer { .. } => {
                        Self::AchBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::SepaBankTransfer { .. } => {
                        Self::SepaBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::BacsBankTransfer { .. } => {
                        Self::BacsBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::MultibancoBankTransfer { .. } => {
                        Self::MultibancoBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::PermataBankTransfer { .. } => {
                        Self::PermataBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::BcaBankTransfer { .. } => {
                        Self::BcaBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::BniVaBankTransfer { .. } => {
                        Self::BniVaBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::BriVaBankTransfer { .. } => {
                        Self::BriVaBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::CimbVaBankTransfer { .. } => {
                        Self::CimbVaBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::DanamonVaBankTransfer { .. } => {
                        Self::DanamonVaBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::MandiriVaBankTransfer { .. } => {
                        Self::MandiriVaBankTransfer
                    }
                     hyperswitch_domain_models::payment_method_data::BankTransferData::Pix { .. } => Self::Pix,
                     hyperswitch_domain_models::payment_method_data::BankTransferData::Pse {} => Self::Pse,
                     hyperswitch_domain_models::payment_method_data::BankTransferData::LocalBankTransfer { .. } => {
                        Self::LocalBankTransfer
                    }
                }
            }
            PaymentMethodData::Crypto(_) => Self::Crypto,
            PaymentMethodData::MandatePayment => Self::MandatePayment,
            PaymentMethodData::Reward => Self::Reward,
            PaymentMethodData::Upi(_) => Self::Upi,
            PaymentMethodData::Voucher(voucher_data) => match voucher_data {
                 hyperswitch_domain_models::payment_method_data::VoucherData::Boleto(_) => Self::Boleto,
                 hyperswitch_domain_models::payment_method_data::VoucherData::Efecty => Self::Efecty,
                 hyperswitch_domain_models::payment_method_data::VoucherData::PagoEfectivo => Self::PagoEfectivo,
                 hyperswitch_domain_models::payment_method_data::VoucherData::RedCompra => Self::RedCompra,
                 hyperswitch_domain_models::payment_method_data::VoucherData::RedPagos => Self::RedPagos,
                 hyperswitch_domain_models::payment_method_data::VoucherData::Alfamart(_) => Self::Alfamart,
                 hyperswitch_domain_models::payment_method_data::VoucherData::Indomaret(_) => Self::Indomaret,
                 hyperswitch_domain_models::payment_method_data::VoucherData::Oxxo => Self::Oxxo,
                 hyperswitch_domain_models::payment_method_data::VoucherData::SevenEleven(_) => Self::SevenEleven,
                 hyperswitch_domain_models::payment_method_data::VoucherData::Lawson(_) => Self::Lawson,
                 hyperswitch_domain_models::payment_method_data::VoucherData::MiniStop(_) => Self::MiniStop,
                 hyperswitch_domain_models::payment_method_data::VoucherData::FamilyMart(_) => Self::FamilyMart,
                 hyperswitch_domain_models::payment_method_data::VoucherData::Seicomart(_) => Self::Seicomart,
                 hyperswitch_domain_models::payment_method_data::VoucherData::PayEasy(_) => Self::PayEasy,
            },
            PaymentMethodData::RealTimePayment(real_time_payment_data) => match *real_time_payment_data{
                hyperswitch_domain_models::payment_method_data::RealTimePaymentData::DuitNow {  } =>  Self::DuitNow,
                hyperswitch_domain_models::payment_method_data::RealTimePaymentData::Fps {  } => Self::Fps,
                hyperswitch_domain_models::payment_method_data::RealTimePaymentData::PromptPay {  } => Self::PromptPay,
                hyperswitch_domain_models::payment_method_data::RealTimePaymentData::VietQr {  } => Self::VietQr,
            },
            PaymentMethodData::GiftCard(gift_card_data) => {
                match *gift_card_data {
                     hyperswitch_domain_models::payment_method_data::GiftCardData::Givex(_) => Self::Givex,
                     hyperswitch_domain_models::payment_method_data::GiftCardData::PaySafeCard {} => Self::PaySafeCar,
                }
            }
            PaymentMethodData::CardToken(_) => Self::CardToken,
            PaymentMethodData::OpenBanking(data) => match data {
                hyperswitch_domain_models::payment_method_data::OpenBankingData::OpenBankingPIS {  } => Self::OpenBanking
            },
        }
    }
}
pub trait ApplePay {
    fn get_applepay_decoded_payment_data(&self) -> Result<Secret<String>, Error>;
}

impl ApplePay for hyperswitch_domain_models::payment_method_data::ApplePayWalletData {
    fn get_applepay_decoded_payment_data(&self) -> Result<Secret<String>, Error> {
        let token = Secret::new(
            String::from_utf8(BASE64_ENGINE.decode(&self.payment_data).change_context(
                errors::ConnectorError::InvalidWalletToken {
                    wallet_name: "Apple Pay".to_string(),
                },
            )?)
            .change_context(errors::ConnectorError::InvalidWalletToken {
                wallet_name: "Apple Pay".to_string(),
            })?,
        );
        Ok(token)
    }
}

pub trait WalletData {
    fn get_wallet_token(&self) -> Result<Secret<String>, Error>;
    fn get_wallet_token_as_json<T>(&self, wallet_name: String) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned;
    fn get_encoded_wallet_token(&self) -> Result<String, Error>;
}

impl WalletData for hyperswitch_domain_models::payment_method_data::WalletData {
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
                let encoded_token = BASE64_ENGINE.encode(token_as_vec);
                Ok(encoded_token)
            }
            _ => Err(
                errors::ConnectorError::NotImplemented("SELECTED PAYMENT METHOD".to_owned()).into(),
            ),
        }
    }
}

/// Trait for converting from one foreign type to another
pub trait ForeignFrom<F> {
    /// Convert from a foreign type to the current type
    fn foreign_from(from: F) -> Self;
}