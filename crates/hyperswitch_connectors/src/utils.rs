use std::collections::{HashMap, HashSet};

use api_models::payments;
#[cfg(feature = "payouts")]
use api_models::payouts::PayoutVendorAccountDetails;
use base64::Engine;
use common_enums::{
    enums,
    enums::{
        AlbaniaStatesAbbreviation, AndorraStatesAbbreviation, AttemptStatus,
        AustriaStatesAbbreviation, BelarusStatesAbbreviation, BelgiumStatesAbbreviation,
        BosniaAndHerzegovinaStatesAbbreviation, BulgariaStatesAbbreviation,
        CanadaStatesAbbreviation, CroatiaStatesAbbreviation, CzechRepublicStatesAbbreviation,
        DenmarkStatesAbbreviation, FinlandStatesAbbreviation, FranceStatesAbbreviation,
        FutureUsage, GermanyStatesAbbreviation, GreeceStatesAbbreviation,
        HungaryStatesAbbreviation, IcelandStatesAbbreviation, IrelandStatesAbbreviation,
        ItalyStatesAbbreviation, LatviaStatesAbbreviation, LiechtensteinStatesAbbreviation,
        LithuaniaStatesAbbreviation, LuxembourgStatesAbbreviation, MaltaStatesAbbreviation,
        MoldovaStatesAbbreviation, MonacoStatesAbbreviation, MontenegroStatesAbbreviation,
        NetherlandsStatesAbbreviation, NorthMacedoniaStatesAbbreviation, NorwayStatesAbbreviation,
        PolandStatesAbbreviation, PortugalStatesAbbreviation, RomaniaStatesAbbreviation,
        RussiaStatesAbbreviation, SanMarinoStatesAbbreviation, SerbiaStatesAbbreviation,
        SlovakiaStatesAbbreviation, SloveniaStatesAbbreviation, SpainStatesAbbreviation,
        SwedenStatesAbbreviation, SwitzerlandStatesAbbreviation, UkraineStatesAbbreviation,
        UnitedKingdomStatesAbbreviation, UsStatesAbbreviation,
    },
};
use common_utils::{
    consts::BASE64_ENGINE,
    errors::{CustomResult, ParsingError, ReportSwitchExt},
    ext_traits::{OptionExt, StringExt, ValueExt},
    id_type,
    pii::{self, Email, IpAddress},
    types::{AmountConvertor, MinorUnit},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    address::{Address, AddressDetails, PhoneDetails},
    network_tokenization::NetworkTokenNumber,
    payment_method_data::{self, Card, CardDetailsForNetworkTransactionId, PaymentMethodData},
    router_data::{
        ApplePayPredecryptData, ErrorResponse, PaymentMethodToken, RecurringMandatePaymentData,
    },
    router_request_types::{
        AuthenticationData, BrowserInformation, CompleteAuthorizeData, ConnectorCustomerData,
        MandateRevokeRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsPreProcessingData, PaymentsSyncData,
        RefundsData, ResponseId, SetupMandateRequestData,
    },
    router_response_types::CaptureSyncResponse,
    types::OrderDetailsWithAmount,
};
use hyperswitch_interfaces::{api, consts, errors, types::Response};
use image::Luma;
use masking::{ExposeInterface, PeekInterface, Secret};
use once_cell::sync::Lazy;
use regex::Regex;
use router_env::logger;
use serde::Serializer;
use serde_json::Value;
use time::PrimitiveDateTime;

use crate::{constants::UNSUPPORTED_ERROR_MESSAGE, types::RefreshTokenRouterData};

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

pub(crate) fn get_timestamp_in_milliseconds(datetime: &PrimitiveDateTime) -> i64 {
    let utc_datetime = datetime.assume_utc();
    utc_datetime.unix_timestamp() * 1000
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

pub(crate) fn base64_decode(data: String) -> Result<Vec<u8>, Error> {
    BASE64_ENGINE
        .decode(data)
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)
}

pub(crate) fn to_currency_base_unit(
    amount: i64,
    currency: enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    currency
        .to_currency_base_unit(amount)
        .change_context(errors::ConnectorError::ParsingFailed)
}

pub(crate) fn to_currency_lower_unit(
    amount: String,
    currency: enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    currency
        .to_currency_lower_unit(amount)
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
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

pub(crate) fn get_error_code_error_message_based_on_priority(
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
    fn get_capture_attempt_status(&self) -> AttemptStatus;
    fn is_capture_response(&self) -> bool;
    fn get_connector_reference_id(&self) -> Option<String> {
        None
    }
    fn get_amount_captured(&self) -> Result<Option<MinorUnit>, error_stack::Report<ParsingError>>;
}

pub(crate) fn construct_captures_response_hashmap<T>(
    capture_sync_response_list: Vec<T>,
) -> CustomResult<HashMap<String, CaptureSyncResponse>, errors::ConnectorError>
where
    T: MultipleCaptureSyncResponse,
{
    let mut hashmap = HashMap::new();
    for capture_sync_response in capture_sync_response_list {
        let connector_capture_id = capture_sync_response.get_connector_capture_id();
        if capture_sync_response.is_capture_response() {
            hashmap.insert(
                connector_capture_id.clone(),
                CaptureSyncResponse::Success {
                    resource_id: ResponseId::ConnectorTransactionId(connector_capture_id),
                    status: capture_sync_response.get_capture_attempt_status(),
                    connector_response_reference_id: capture_sync_response
                        .get_connector_reference_id(),
                    amount: capture_sync_response
                        .get_amount_captured()
                        .change_context(errors::ConnectorError::AmountConversionFailed)
                        .attach_printable(
                            "failed to convert back captured response amount to minor unit",
                        )?,
                },
            );
        }
    }

    Ok(hashmap)
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

impl From<payment_method_data::GooglePayWalletData> for GooglePayWalletData {
    fn from(data: payment_method_data::GooglePayWalletData) -> Self {
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
    connector_meta: Option<Secret<Value>>,
) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let connector_meta_secret =
        connector_meta.ok_or_else(missing_field_err("connector_meta_data"))?;
    let json = connector_meta_secret.expose();
    json.parse_value(std::any::type_name::<T>()).switch()
}

pub(crate) fn generate_random_bytes(length: usize) -> Vec<u8> {
    // returns random bytes of length n
    let mut rng = rand::thread_rng();
    (0..length).map(|_| rand::Rng::gen(&mut rng)).collect()
}

pub(crate) fn missing_field_err(
    message: &'static str,
) -> Box<dyn Fn() -> error_stack::Report<errors::ConnectorError> + 'static> {
    Box::new(move || {
        errors::ConnectorError::MissingRequiredField {
            field_name: message,
        }
        .into()
    })
}

pub(crate) fn handle_json_response_deserialization_failure(
    res: Response,
    connector: &'static str,
) -> CustomResult<ErrorResponse, errors::ConnectorError> {
    crate::metrics::CONNECTOR_RESPONSE_DESERIALIZATION_FAILURE
        .add(1, router_env::metric_attributes!(("connector", connector)));

    let response_data = String::from_utf8(res.response.to_vec())
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

    // check for whether the response is in json format
    match serde_json::from_str::<Value>(&response_data) {
        // in case of unexpected response but in json format
        Ok(_) => Err(errors::ConnectorError::ResponseDeserializationFailed)?,
        // in case of unexpected response but in html or string format
        Err(error_msg) => {
            logger::error!(deserialization_error=?error_msg);
            logger::error!("UNEXPECTED RESPONSE FROM CONNECTOR: {}", response_data);
            Ok(ErrorResponse {
                status_code: res.status_code,
                code: consts::NO_ERROR_CODE.to_string(),
                message: UNSUPPORTED_ERROR_MESSAGE.to_string(),
                reason: Some(response_data),
                attempt_status: None,
                connector_transaction_id: None,
            })
        }
    }
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

pub(crate) fn to_connector_meta<T>(connector_meta: Option<Value>) -> Result<T, Error>
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

pub(crate) fn validate_currency(
    request_currency: enums::Currency,
    merchant_config_currency: Option<enums::Currency>,
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
    fn get_optional_shipping_full_name(&self) -> Option<Secret<String>>;
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

    fn get_optional_shipping_full_name(&self) -> Option<Secret<String>> {
        self.get_optional_shipping()
            .and_then(|shipping_details| shipping_details.address.as_ref())
            .and_then(|shipping_address| shipping_address.get_optional_full_name())
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
    fn get_expiry_year_as_4_digit_i32(&self) -> Result<Secret<i32>, Error>;
    fn get_cardholder_name(&self) -> Result<Secret<String>, Error>;
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
    fn get_expiry_year_as_4_digit_i32(&self) -> Result<Secret<i32>, Error> {
        self.get_expiry_year_4_digit()
            .peek()
            .clone()
            .parse::<i32>()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            .map(Secret::new)
    }
    fn get_cardholder_name(&self) -> Result<Secret<String>, Error> {
        self.card_holder_name
            .clone()
            .ok_or_else(missing_field_err("card.card_holder_name"))
    }
}

impl CardData for CardDetailsForNetworkTransactionId {
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
    fn get_expiry_year_as_4_digit_i32(&self) -> Result<Secret<i32>, Error> {
        self.get_expiry_year_4_digit()
            .peek()
            .clone()
            .parse::<i32>()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            .map(Secret::new)
    }
    fn get_cardholder_name(&self) -> Result<Secret<String>, Error> {
        self.card_holder_name
            .clone()
            .ok_or_else(missing_field_err("card.card_holder_name"))
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
    fn get_optional_first_name(&self) -> Option<Secret<String>>;
    fn get_optional_last_name(&self) -> Option<Secret<String>>;
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
            api_models::enums::CountryAlpha2::AL => Ok(Secret::new(
                AlbaniaStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::AD => Ok(Secret::new(
                AndorraStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::AT => Ok(Secret::new(
                AustriaStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::BY => Ok(Secret::new(
                BelarusStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::BA => Ok(Secret::new(
                BosniaAndHerzegovinaStatesAbbreviation::foreign_try_from(state.peek().to_string())?
                    .to_string(),
            )),
            api_models::enums::CountryAlpha2::BG => Ok(Secret::new(
                BulgariaStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::HR => Ok(Secret::new(
                CroatiaStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::CZ => Ok(Secret::new(
                CzechRepublicStatesAbbreviation::foreign_try_from(state.peek().to_string())?
                    .to_string(),
            )),
            api_models::enums::CountryAlpha2::DK => Ok(Secret::new(
                DenmarkStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::FI => Ok(Secret::new(
                FinlandStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::FR => Ok(Secret::new(
                FranceStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::DE => Ok(Secret::new(
                GermanyStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::GR => Ok(Secret::new(
                GreeceStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::HU => Ok(Secret::new(
                HungaryStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::IS => Ok(Secret::new(
                IcelandStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::IE => Ok(Secret::new(
                IrelandStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::LV => Ok(Secret::new(
                LatviaStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::IT => Ok(Secret::new(
                ItalyStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::LI => Ok(Secret::new(
                LiechtensteinStatesAbbreviation::foreign_try_from(state.peek().to_string())?
                    .to_string(),
            )),
            api_models::enums::CountryAlpha2::LT => Ok(Secret::new(
                LithuaniaStatesAbbreviation::foreign_try_from(state.peek().to_string())?
                    .to_string(),
            )),
            api_models::enums::CountryAlpha2::MT => Ok(Secret::new(
                MaltaStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::MD => Ok(Secret::new(
                MoldovaStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::MC => Ok(Secret::new(
                MonacoStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::ME => Ok(Secret::new(
                MontenegroStatesAbbreviation::foreign_try_from(state.peek().to_string())?
                    .to_string(),
            )),
            api_models::enums::CountryAlpha2::NL => Ok(Secret::new(
                NetherlandsStatesAbbreviation::foreign_try_from(state.peek().to_string())?
                    .to_string(),
            )),
            api_models::enums::CountryAlpha2::MK => Ok(Secret::new(
                NorthMacedoniaStatesAbbreviation::foreign_try_from(state.peek().to_string())?
                    .to_string(),
            )),
            api_models::enums::CountryAlpha2::NO => Ok(Secret::new(
                NorwayStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::PL => Ok(Secret::new(
                PolandStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::PT => Ok(Secret::new(
                PortugalStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::ES => Ok(Secret::new(
                SpainStatesAbbreviation::foreign_try_from(state.peek().to_string())?.to_string(),
            )),
            api_models::enums::CountryAlpha2::CH => Ok(Secret::new(
                SwitzerlandStatesAbbreviation::foreign_try_from(state.peek().to_string())?
                    .to_string(),
            )),
            api_models::enums::CountryAlpha2::GB => Ok(Secret::new(
                UnitedKingdomStatesAbbreviation::foreign_try_from(state.peek().to_string())?
                    .to_string(),
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

    fn get_optional_first_name(&self) -> Option<Secret<String>> {
        self.first_name.clone()
    }

    fn get_optional_last_name(&self) -> Option<Secret<String>> {
        self.last_name.clone()
    }
}

pub trait AdditionalCardInfo {
    fn get_card_expiry_year_2_digit(&self) -> Result<Secret<String>, errors::ConnectorError>;
}

impl AdditionalCardInfo for payments::AdditionalCardInfo {
    fn get_card_expiry_year_2_digit(&self) -> Result<Secret<String>, errors::ConnectorError> {
        let binding =
            self.card_exp_year
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "card_exp_year",
                })?;
        let year = binding.peek();
        Ok(Secret::new(
            year.get(year.len() - 2..)
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                .to_string(),
        ))
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

#[cfg(feature = "payouts")]
pub trait PayoutFulfillRequestData {
    fn get_connector_payout_id(&self) -> Result<String, Error>;
    fn get_connector_transfer_method_id(&self) -> Result<String, Error>;
}
#[cfg(feature = "payouts")]
impl PayoutFulfillRequestData for hyperswitch_domain_models::router_request_types::PayoutsData {
    fn get_connector_payout_id(&self) -> Result<String, Error> {
        self.connector_payout_id
            .clone()
            .ok_or_else(missing_field_err("connector_payout_id"))
    }

    fn get_connector_transfer_method_id(&self) -> Result<String, Error> {
        self.connector_transfer_method_id
            .clone()
            .ok_or_else(missing_field_err("connector_transfer_method_id"))
    }
}

pub trait CustomerData {
    fn get_email(&self) -> Result<Email, Error>;
}

impl CustomerData for ConnectorCustomerData {
    fn get_email(&self) -> Result<Email, Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }
}
pub trait PaymentsAuthorizeRequestData {
    fn get_optional_language_from_browser_info(&self) -> Option<String>;
    fn is_auto_capture(&self) -> Result<bool, Error>;
    fn get_email(&self) -> Result<Email, Error>;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error>;
    fn get_card(&self) -> Result<Card, Error>;
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
    fn get_customer_name(&self) -> Result<Secret<String>, Error>;
    fn get_connector_mandate_request_reference_id(&self) -> Result<String, Error>;
    fn get_card_holder_name_from_additional_payment_method_data(
        &self,
    ) -> Result<Secret<String>, Error>;
    fn is_cit_mandate_payment(&self) -> bool;
    fn get_optional_network_transaction_id(&self) -> Option<String>;
    fn get_optional_email(&self) -> Option<Email>;
}

impl PaymentsAuthorizeRequestData for PaymentsAuthorizeData {
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(enums::CaptureMethod::Automatic)
            | Some(enums::CaptureMethod::SequentialAutomatic)
            | None => Ok(true),
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
                    connector_mandate_ids.get_connector_mandate_id()
                }
                Some(payments::MandateReferenceId::NetworkMandateId(_))
                | None
                | Some(payments::MandateReferenceId::NetworkTokenWithNTI(_)) => None,
            })
    }
    fn is_mandate_payment(&self) -> bool {
        ((self.customer_acceptance.is_some() || self.setup_mandate_details.is_some())
            && (self.setup_future_usage == Some(FutureUsage::OffSession)))
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
            && self.setup_future_usage == Some(FutureUsage::OffSession)
    }

    fn get_metadata_as_object(&self) -> Option<pii::SecretSerdeValue> {
        self.metadata.clone().and_then(|meta_data| match meta_data {
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Array(_) => None,
            Value::Object(_) => Some(meta_data.into()),
        })
    }

    fn get_authentication_data(&self) -> Result<AuthenticationData, Error> {
        self.authentication_data
            .clone()
            .ok_or_else(missing_field_err("authentication_data"))
    }

    fn get_customer_name(&self) -> Result<Secret<String>, Error> {
        self.customer_name
            .clone()
            .ok_or_else(missing_field_err("customer_name"))
    }

    fn get_card_holder_name_from_additional_payment_method_data(
        &self,
    ) -> Result<Secret<String>, Error> {
        match &self.additional_payment_method_data {
            Some(payments::AdditionalPaymentData::Card(card_data)) => Ok(card_data
                .card_holder_name
                .clone()
                .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                    field_name: "card_holder_name",
                })?),
            _ => Err(errors::ConnectorError::MissingRequiredFields {
                field_names: vec!["card_holder_name"],
            }
            .into()),
        }
    }
    /// Attempts to retrieve the connector mandate reference ID as a `Result<String, Error>`.
    fn get_connector_mandate_request_reference_id(&self) -> Result<String, Error> {
        self.mandate_id
            .as_ref()
            .and_then(|mandate_ids| match &mandate_ids.mandate_reference_id {
                Some(payments::MandateReferenceId::ConnectorMandateId(connector_mandate_ids)) => {
                    connector_mandate_ids.get_connector_mandate_request_reference_id()
                }
                Some(payments::MandateReferenceId::NetworkMandateId(_))
                | None
                | Some(payments::MandateReferenceId::NetworkTokenWithNTI(_)) => None,
            })
            .ok_or_else(missing_field_err("connector_mandate_request_reference_id"))
    }
    fn is_cit_mandate_payment(&self) -> bool {
        (self.customer_acceptance.is_some() || self.setup_mandate_details.is_some())
            && self.setup_future_usage == Some(FutureUsage::OffSession)
    }
    fn get_optional_network_transaction_id(&self) -> Option<String> {
        self.mandate_id
            .as_ref()
            .and_then(|mandate_ids| match &mandate_ids.mandate_reference_id {
                Some(payments::MandateReferenceId::NetworkMandateId(network_transaction_id)) => {
                    Some(network_transaction_id.clone())
                }
                Some(payments::MandateReferenceId::ConnectorMandateId(_))
                | Some(payments::MandateReferenceId::NetworkTokenWithNTI(_))
                | None => None,
            })
    }
    fn get_optional_email(&self) -> Option<Email> {
        self.email.clone()
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
            Some(enums::CaptureMethod::Automatic)
            | Some(enums::CaptureMethod::SequentialAutomatic)
            | None => Ok(true),
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
    fn get_connector_metadata(&self) -> Result<Value, Error>;
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
    fn get_connector_metadata(&self) -> Result<Value, Error> {
        self.connector_metadata
            .clone()
            .ok_or_else(missing_field_err("connector_metadata"))
    }
}

pub trait PaymentsSetupMandateRequestData {
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
    fn get_email(&self) -> Result<Email, Error>;
    fn get_router_return_url(&self) -> Result<String, Error>;
    fn is_card(&self) -> bool;
    fn get_return_url(&self) -> Result<String, Error>;
    fn get_webhook_url(&self) -> Result<String, Error>;
    fn get_optional_language_from_browser_info(&self) -> Option<String>;
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
    fn get_router_return_url(&self) -> Result<String, Error> {
        self.router_return_url
            .clone()
            .ok_or_else(missing_field_err("router_return_url"))
    }
    fn is_card(&self) -> bool {
        matches!(self.payment_method_data, PaymentMethodData::Card(_))
    }
    fn get_return_url(&self) -> Result<String, Error> {
        self.router_return_url
            .clone()
            .ok_or_else(missing_field_err("return_url"))
    }
    fn get_webhook_url(&self) -> Result<String, Error> {
        self.webhook_url
            .clone()
            .ok_or_else(missing_field_err("webhook_url"))
    }
    fn get_optional_language_from_browser_info(&self) -> Option<String> {
        self.browser_info
            .clone()
            .and_then(|browser_info| browser_info.language)
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
    fn get_connector_mandate_request_reference_id(&self) -> Result<String, Error>;
    fn is_cit_mandate_payment(&self) -> bool;
}

impl PaymentsCompleteAuthorizeRequestData for CompleteAuthorizeData {
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(enums::CaptureMethod::Automatic)
            | Some(enums::CaptureMethod::SequentialAutomatic)
            | None => Ok(true),
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
            && self.setup_future_usage == Some(FutureUsage::OffSession))
            || self
                .mandate_id
                .as_ref()
                .and_then(|mandate_ids| mandate_ids.mandate_reference_id.as_ref())
                .is_some()
    }
    /// Attempts to retrieve the connector mandate reference ID as a `Result<String, Error>`.
    fn get_connector_mandate_request_reference_id(&self) -> Result<String, Error> {
        self.mandate_id
            .as_ref()
            .and_then(|mandate_ids| match &mandate_ids.mandate_reference_id {
                Some(payments::MandateReferenceId::ConnectorMandateId(connector_mandate_ids)) => {
                    connector_mandate_ids.get_connector_mandate_request_reference_id()
                }
                Some(payments::MandateReferenceId::NetworkMandateId(_))
                | None
                | Some(payments::MandateReferenceId::NetworkTokenWithNTI(_)) => None,
            })
            .ok_or_else(missing_field_err("connector_mandate_request_reference_id"))
    }
    fn is_cit_mandate_payment(&self) -> bool {
        (self.customer_acceptance.is_some() || self.setup_mandate_details.is_some())
            && self.setup_future_usage == Some(FutureUsage::OffSession)
    }
}
pub trait AddressData {
    fn get_optional_full_name(&self) -> Option<Secret<String>>;
    fn get_email(&self) -> Result<Email, Error>;
    fn get_phone_with_country_code(&self) -> Result<Secret<String>, Error>;
    fn get_optional_first_name(&self) -> Option<Secret<String>>;
    fn get_optional_last_name(&self) -> Option<Secret<String>>;
}

impl AddressData for Address {
    fn get_optional_full_name(&self) -> Option<Secret<String>> {
        self.address
            .as_ref()
            .and_then(|billing_address| billing_address.get_optional_full_name())
    }

    fn get_email(&self) -> Result<Email, Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }

    fn get_phone_with_country_code(&self) -> Result<Secret<String>, Error> {
        self.phone
            .clone()
            .map(|phone_details| phone_details.get_number_with_country_code())
            .transpose()?
            .ok_or_else(missing_field_err("phone"))
    }

    fn get_optional_first_name(&self) -> Option<Secret<String>> {
        self.address
            .as_ref()
            .and_then(|billing_address| billing_address.get_optional_first_name())
    }

    fn get_optional_last_name(&self) -> Option<Secret<String>> {
        self.address
            .as_ref()
            .and_then(|billing_address| billing_address.get_optional_last_name())
    }
}
pub trait PaymentsPreProcessingRequestData {
    fn get_redirect_response_payload(&self) -> Result<pii::SecretSerdeValue, Error>;
    fn get_email(&self) -> Result<Email, Error>;
    fn get_payment_method_type(&self) -> Result<enums::PaymentMethodType, Error>;
    fn get_currency(&self) -> Result<enums::Currency, Error>;
    fn get_amount(&self) -> Result<i64, Error>;
    fn get_minor_amount(&self) -> Result<MinorUnit, Error>;
    fn is_auto_capture(&self) -> Result<bool, Error>;
    fn get_order_details(&self) -> Result<Vec<OrderDetailsWithAmount>, Error>;
    fn get_webhook_url(&self) -> Result<String, Error>;
    fn get_router_return_url(&self) -> Result<String, Error>;
    fn get_browser_info(&self) -> Result<BrowserInformation, Error>;
    fn get_complete_authorize_url(&self) -> Result<String, Error>;
    fn connector_mandate_id(&self) -> Option<String>;
}

impl PaymentsPreProcessingRequestData for PaymentsPreProcessingData {
    fn get_email(&self) -> Result<Email, Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }
    fn get_payment_method_type(&self) -> Result<enums::PaymentMethodType, Error> {
        self.payment_method_type
            .to_owned()
            .ok_or_else(missing_field_err("payment_method_type"))
    }
    fn get_currency(&self) -> Result<enums::Currency, Error> {
        self.currency.ok_or_else(missing_field_err("currency"))
    }
    fn get_amount(&self) -> Result<i64, Error> {
        self.amount.ok_or_else(missing_field_err("amount"))
    }

    // New minor amount function for amount framework
    fn get_minor_amount(&self) -> Result<MinorUnit, Error> {
        self.minor_amount.ok_or_else(missing_field_err("amount"))
    }
    fn is_auto_capture(&self) -> Result<bool, Error> {
        match self.capture_method {
            Some(enums::CaptureMethod::Automatic)
            | None
            | Some(enums::CaptureMethod::SequentialAutomatic) => Ok(true),
            Some(enums::CaptureMethod::Manual) => Ok(false),
            Some(enums::CaptureMethod::ManualMultiple) | Some(enums::CaptureMethod::Scheduled) => {
                Err(errors::ConnectorError::CaptureMethodNotSupported.into())
            }
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
    fn get_router_return_url(&self) -> Result<String, Error> {
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
    fn connector_mandate_id(&self) -> Option<String> {
        self.mandate_id
            .as_ref()
            .and_then(|mandate_ids| match &mandate_ids.mandate_reference_id {
                Some(payments::MandateReferenceId::ConnectorMandateId(connector_mandate_ids)) => {
                    connector_mandate_ids.get_connector_mandate_id()
                }
                Some(payments::MandateReferenceId::NetworkMandateId(_))
                | None
                | Some(payments::MandateReferenceId::NetworkTokenWithNTI(_)) => None,
            })
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
    fn get_os_type(&self) -> Result<String, Error>;
    fn get_os_version(&self) -> Result<String, Error>;
    fn get_device_model(&self) -> Result<String, Error>;
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
    fn get_os_type(&self) -> Result<String, Error> {
        self.os_type
            .clone()
            .ok_or_else(missing_field_err("browser_info.os_type"))
    }
    fn get_os_version(&self) -> Result<String, Error> {
        self.os_version
            .clone()
            .ok_or_else(missing_field_err("browser_info.os_version"))
    }
    fn get_device_model(&self) -> Result<String, Error> {
        self.device_model
            .clone()
            .ok_or_else(missing_field_err("browser_info.device_model"))
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

impl CryptoData for payment_method_data::CryptoData {
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

impl ForeignTryFrom<String> for PolandStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "PolandStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "greater poland voivodeship" => Ok(Self::GreaterPolandVoivodeship),
                    "kielce" => Ok(Self::Kielce),
                    "kuyavian pomeranian voivodeship" => Ok(Self::KuyavianPomeranianVoivodeship),
                    "lesser poland voivodeship" => Ok(Self::LesserPolandVoivodeship),
                    "lower silesian voivodeship" => Ok(Self::LowerSilesianVoivodeship),
                    "lublin voivodeship" => Ok(Self::LublinVoivodeship),
                    "lubusz voivodeship" => Ok(Self::LubuszVoivodeship),
                    "masovian voivodeship" => Ok(Self::MasovianVoivodeship),
                    "opole voivodeship" => Ok(Self::OpoleVoivodeship),
                    "podkarpackie voivodeship" => Ok(Self::PodkarpackieVoivodeship),
                    "podlaskie voivodeship" => Ok(Self::PodlaskieVoivodeship),
                    "pomeranian voivodeship" => Ok(Self::PomeranianVoivodeship),
                    "silesian voivodeship" => Ok(Self::SilesianVoivodeship),
                    "warmian masurian voivodeship" => Ok(Self::WarmianMasurianVoivodeship),
                    "west pomeranian voivodeship" => Ok(Self::WestPomeranianVoivodeship),
                    "lodz voivodeship" => Ok(Self::LodzVoivodeship),
                    "swietokrzyskie voivodeship" => Ok(Self::SwietokrzyskieVoivodeship),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for FranceStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "FranceStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "alo" => Ok(Self::Alo),
                    "alsace" => Ok(Self::Alsace),
                    "aquitaine" => Ok(Self::Aquitaine),
                    "auvergne" => Ok(Self::Auvergne),
                    "auvergne rhone alpes" => Ok(Self::AuvergneRhoneAlpes),
                    "bourgogne franche comte" => Ok(Self::BourgogneFrancheComte),
                    "brittany" => Ok(Self::Brittany),
                    "burgundy" => Ok(Self::Burgundy),
                    "centre val de loire" => Ok(Self::CentreValDeLoire),
                    "champagne ardenne" => Ok(Self::ChampagneArdenne),
                    "corsica" => Ok(Self::Corsica),
                    "franche comte" => Ok(Self::FrancheComte),
                    "french guiana" => Ok(Self::FrenchGuiana),
                    "french polynesia" => Ok(Self::FrenchPolynesia),
                    "grand est" => Ok(Self::GrandEst),
                    "guadeloupe" => Ok(Self::Guadeloupe),
                    "hauts de france" => Ok(Self::HautsDeFrance),
                    "ile de france" => Ok(Self::IleDeFrance),
                    "normandy" => Ok(Self::Normandy),
                    "nouvelle aquitaine" => Ok(Self::NouvelleAquitaine),
                    "occitania" => Ok(Self::Occitania),
                    "paris" => Ok(Self::Paris),
                    "pays de la loire" => Ok(Self::PaysDeLaLoire),
                    "provence alpes cote d azur" => Ok(Self::ProvenceAlpesCoteDAzur),
                    "reunion" => Ok(Self::Reunion),
                    "saint barthelemy" => Ok(Self::SaintBarthelemy),
                    "saint martin" => Ok(Self::SaintMartin),
                    "saint pierre and miquelon" => Ok(Self::SaintPierreAndMiquelon),
                    "upper normandy" => Ok(Self::UpperNormandy),
                    "wallis and futuna" => Ok(Self::WallisAndFutuna),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for GermanyStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "GermanyStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "baden wurttemberg" => Ok(Self::BadenWurttemberg),
                    "bavaria" => Ok(Self::Bavaria),
                    "berlin" => Ok(Self::Berlin),
                    "brandenburg" => Ok(Self::Brandenburg),
                    "bremen" => Ok(Self::Bremen),
                    "hamburg" => Ok(Self::Hamburg),
                    "hesse" => Ok(Self::Hesse),
                    "lower saxony" => Ok(Self::LowerSaxony),
                    "mecklenburg vorpommern" => Ok(Self::MecklenburgVorpommern),
                    "north rhine westphalia" => Ok(Self::NorthRhineWestphalia),
                    "rhineland palatinate" => Ok(Self::RhinelandPalatinate),
                    "saarland" => Ok(Self::Saarland),
                    "saxony" => Ok(Self::Saxony),
                    "saxony anhalt" => Ok(Self::SaxonyAnhalt),
                    "schleswig holstein" => Ok(Self::SchleswigHolstein),
                    "thuringia" => Ok(Self::Thuringia),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for SpainStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "SpainStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "a coruna province" => Ok(Self::ACorunaProvince),
                    "albacete province" => Ok(Self::AlbaceteProvince),
                    "alicante province" => Ok(Self::AlicanteProvince),
                    "almeria province" => Ok(Self::AlmeriaProvince),
                    "andalusia" => Ok(Self::Andalusia),
                    "araba alava" => Ok(Self::ArabaAlava),
                    "aragon" => Ok(Self::Aragon),
                    "badajoz province" => Ok(Self::BadajozProvince),
                    "balearic islands" => Ok(Self::BalearicIslands),
                    "barcelona province" => Ok(Self::BarcelonaProvince),
                    "basque country" => Ok(Self::BasqueCountry),
                    "biscay" => Ok(Self::Biscay),
                    "burgos province" => Ok(Self::BurgosProvince),
                    "canary islands" => Ok(Self::CanaryIslands),
                    "cantabria" => Ok(Self::Cantabria),
                    "castellon province" => Ok(Self::CastellonProvince),
                    "castile and leon" => Ok(Self::CastileAndLeon),
                    "castile la mancha" => Ok(Self::CastileLaMancha),
                    "catalonia" => Ok(Self::Catalonia),
                    "ceuta" => Ok(Self::Ceuta),
                    "ciudad real province" => Ok(Self::CiudadRealProvince),
                    "community of madrid" => Ok(Self::CommunityOfMadrid),
                    "cuenca province" => Ok(Self::CuencaProvince),
                    "caceres province" => Ok(Self::CaceresProvince),
                    "cadiz province" => Ok(Self::CadizProvince),
                    "cordoba province" => Ok(Self::CordobaProvince),
                    "extremadura" => Ok(Self::Extremadura),
                    "galicia" => Ok(Self::Galicia),
                    "gipuzkoa" => Ok(Self::Gipuzkoa),
                    "girona province" => Ok(Self::GironaProvince),
                    "granada province" => Ok(Self::GranadaProvince),
                    "guadalajara province" => Ok(Self::GuadalajaraProvince),
                    "huelva province" => Ok(Self::HuelvaProvince),
                    "huesca province" => Ok(Self::HuescaProvince),
                    "jaen province" => Ok(Self::JaenProvince),
                    "la rioja" => Ok(Self::LaRioja),
                    "las palmas province" => Ok(Self::LasPalmasProvince),
                    "leon province" => Ok(Self::LeonProvince),
                    "lleida province" => Ok(Self::LleidaProvince),
                    "lugo province" => Ok(Self::LugoProvince),
                    "madrid province" => Ok(Self::MadridProvince),
                    "melilla" => Ok(Self::Melilla),
                    "murcia province" => Ok(Self::MurciaProvince),
                    "malaga province" => Ok(Self::MalagaProvince),
                    "navarre" => Ok(Self::Navarre),
                    "ourense province" => Ok(Self::OurenseProvince),
                    "palencia province" => Ok(Self::PalenciaProvince),
                    "pontevedra province" => Ok(Self::PontevedraProvince),
                    "province of asturias" => Ok(Self::ProvinceOfAsturias),
                    "province of avila" => Ok(Self::ProvinceOfAvila),
                    "region of murcia" => Ok(Self::RegionOfMurcia),
                    "salamanca province" => Ok(Self::SalamancaProvince),
                    "santa cruz de tenerife province" => Ok(Self::SantaCruzDeTenerifeProvince),
                    "segovia province" => Ok(Self::SegoviaProvince),
                    "seville province" => Ok(Self::SevilleProvince),
                    "soria province" => Ok(Self::SoriaProvince),
                    "tarragona province" => Ok(Self::TarragonaProvince),
                    "teruel province" => Ok(Self::TeruelProvince),
                    "toledo province" => Ok(Self::ToledoProvince),
                    "valencia province" => Ok(Self::ValenciaProvince),
                    "valencian community" => Ok(Self::ValencianCommunity),
                    "valladolid province" => Ok(Self::ValladolidProvince),
                    "zamora province" => Ok(Self::ZamoraProvince),
                    "zaragoza province" => Ok(Self::ZaragozaProvince),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for ItalyStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "ItalyStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "abruzzo" => Ok(Self::Abruzzo),
                    "aosta valley" => Ok(Self::AostaValley),
                    "apulia" => Ok(Self::Apulia),
                    "basilicata" => Ok(Self::Basilicata),
                    "benevento province" => Ok(Self::BeneventoProvince),
                    "calabria" => Ok(Self::Calabria),
                    "campania" => Ok(Self::Campania),
                    "emilia romagna" => Ok(Self::EmiliaRomagna),
                    "friuli venezia giulia" => Ok(Self::FriuliVeneziaGiulia),
                    "lazio" => Ok(Self::Lazio),
                    "liguria" => Ok(Self::Liguria),
                    "lombardy" => Ok(Self::Lombardy),
                    "marche" => Ok(Self::Marche),
                    "molise" => Ok(Self::Molise),
                    "piedmont" => Ok(Self::Piedmont),
                    "sardinia" => Ok(Self::Sardinia),
                    "sicily" => Ok(Self::Sicily),
                    "trentino south tyrol" => Ok(Self::TrentinoSouthTyrol),
                    "tuscany" => Ok(Self::Tuscany),
                    "umbria" => Ok(Self::Umbria),
                    "veneto" => Ok(Self::Veneto),
                    "agrigento" => Ok(Self::Agrigento),
                    "caltanissetta" => Ok(Self::Caltanissetta),
                    "enna" => Ok(Self::Enna),
                    "ragusa" => Ok(Self::Ragusa),
                    "siracusa" => Ok(Self::Siracusa),
                    "trapani" => Ok(Self::Trapani),
                    "bari" => Ok(Self::Bari),
                    "bologna" => Ok(Self::Bologna),
                    "cagliari" => Ok(Self::Cagliari),
                    "catania" => Ok(Self::Catania),
                    "florence" => Ok(Self::Florence),
                    "genoa" => Ok(Self::Genoa),
                    "messina" => Ok(Self::Messina),
                    "milan" => Ok(Self::Milan),
                    "naples" => Ok(Self::Naples),
                    "palermo" => Ok(Self::Palermo),
                    "reggio calabria" => Ok(Self::ReggioCalabria),
                    "rome" => Ok(Self::Rome),
                    "turin" => Ok(Self::Turin),
                    "venice" => Ok(Self::Venice),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for NorwayStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "NorwayStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "akershus" => Ok(Self::Akershus),
                    "buskerud" => Ok(Self::Buskerud),
                    "finnmark" => Ok(Self::Finnmark),
                    "hedmark" => Ok(Self::Hedmark),
                    "hordaland" => Ok(Self::Hordaland),
                    "janmayen" => Ok(Self::JanMayen),
                    "nordtrondelag" => Ok(Self::NordTrondelag),
                    "nordland" => Ok(Self::Nordland),
                    "oppland" => Ok(Self::Oppland),
                    "oslo" => Ok(Self::Oslo),
                    "rogaland" => Ok(Self::Rogaland),
                    "sognogfjordane" => Ok(Self::SognOgFjordane),
                    "svalbard" => Ok(Self::Svalbard),
                    "sortrondelag" => Ok(Self::SorTrondelag),
                    "telemark" => Ok(Self::Telemark),
                    "troms" => Ok(Self::Troms),
                    "trondelag" => Ok(Self::Trondelag),
                    "vestagder" => Ok(Self::VestAgder),
                    "vestfold" => Ok(Self::Vestfold),
                    "ostfold" => Ok(Self::Ostfold),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for AlbaniaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "AlbaniaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "berat" => Ok(Self::Berat),
                    "diber" => Ok(Self::Diber),
                    "durres" => Ok(Self::Durres),
                    "elbasan" => Ok(Self::Elbasan),
                    "fier" => Ok(Self::Fier),
                    "gjirokaster" => Ok(Self::Gjirokaster),
                    "korce" => Ok(Self::Korce),
                    "kukes" => Ok(Self::Kukes),
                    "lezhe" => Ok(Self::Lezhe),
                    "shkoder" => Ok(Self::Shkoder),
                    "tirane" => Ok(Self::Tirane),
                    "vlore" => Ok(Self::Vlore),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for AndorraStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "AndorraStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "andorra la vella" => Ok(Self::AndorraLaVella),
                    "canillo" => Ok(Self::Canillo),
                    "encamp" => Ok(Self::Encamp),
                    "escaldes engordany" => Ok(Self::EscaldesEngordany),
                    "la massana" => Ok(Self::LaMassana),
                    "ordino" => Ok(Self::Ordino),
                    "sant julia de loria" => Ok(Self::SantJuliaDeLoria),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for AustriaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "AustriaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "burgenland" => Ok(Self::Burgenland),
                    "carinthia" => Ok(Self::Carinthia),
                    "lower austria" => Ok(Self::LowerAustria),
                    "salzburg" => Ok(Self::Salzburg),
                    "styria" => Ok(Self::Styria),
                    "tyrol" => Ok(Self::Tyrol),
                    "upper austria" => Ok(Self::UpperAustria),
                    "vienna" => Ok(Self::Vienna),
                    "vorarlberg" => Ok(Self::Vorarlberg),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for RomaniaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "RomaniaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "alba" => Ok(Self::Alba),
                    "arad county" => Ok(Self::AradCounty),
                    "arges" => Ok(Self::Arges),
                    "bacau county" => Ok(Self::BacauCounty),
                    "bihor county" => Ok(Self::BihorCounty),
                    "bistrita nasaud county" => Ok(Self::BistritaNasaudCounty),
                    "botosani county" => Ok(Self::BotosaniCounty),
                    "braila" => Ok(Self::Braila),
                    "brasov county" => Ok(Self::BrasovCounty),
                    "bucharest" => Ok(Self::Bucharest),
                    "buzau county" => Ok(Self::BuzauCounty),
                    "caras severin county" => Ok(Self::CarasSeverinCounty),
                    "cluj county" => Ok(Self::ClujCounty),
                    "constanta county" => Ok(Self::ConstantaCounty),
                    "covasna county" => Ok(Self::CovasnaCounty),
                    "calarasi county" => Ok(Self::CalarasiCounty),
                    "dolj county" => Ok(Self::DoljCounty),
                    "dambovita county" => Ok(Self::DambovitaCounty),
                    "galati county" => Ok(Self::GalatiCounty),
                    "giurgiu county" => Ok(Self::GiurgiuCounty),
                    "gorj county" => Ok(Self::GorjCounty),
                    "harghita county" => Ok(Self::HarghitaCounty),
                    "hunedoara county" => Ok(Self::HunedoaraCounty),
                    "ialomita county" => Ok(Self::IalomitaCounty),
                    "iasi county" => Ok(Self::IasiCounty),
                    "ilfov county" => Ok(Self::IlfovCounty),
                    "mehedinti county" => Ok(Self::MehedintiCounty),
                    "mures county" => Ok(Self::MuresCounty),
                    "neamt county" => Ok(Self::NeamtCounty),
                    "olt county" => Ok(Self::OltCounty),
                    "prahova county" => Ok(Self::PrahovaCounty),
                    "satu mare county" => Ok(Self::SatuMareCounty),
                    "sibiu county" => Ok(Self::SibiuCounty),
                    "suceava county" => Ok(Self::SuceavaCounty),
                    "salaj county" => Ok(Self::SalajCounty),
                    "teleorman county" => Ok(Self::TeleormanCounty),
                    "timis county" => Ok(Self::TimisCounty),
                    "tulcea county" => Ok(Self::TulceaCounty),
                    "vaslui county" => Ok(Self::VasluiCounty),
                    "vrancea county" => Ok(Self::VranceaCounty),
                    "valcea county" => Ok(Self::ValceaCounty),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for PortugalStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "PortugalStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "aveiro district" => Ok(Self::AveiroDistrict),
                    "azores" => Ok(Self::Azores),
                    "beja district" => Ok(Self::BejaDistrict),
                    "braga district" => Ok(Self::BragaDistrict),
                    "braganca district" => Ok(Self::BragancaDistrict),
                    "castelo branco district" => Ok(Self::CasteloBrancoDistrict),
                    "coimbra district" => Ok(Self::CoimbraDistrict),
                    "faro district" => Ok(Self::FaroDistrict),
                    "guarda district" => Ok(Self::GuardaDistrict),
                    "leiria district" => Ok(Self::LeiriaDistrict),
                    "lisbon district" => Ok(Self::LisbonDistrict),
                    "madeira" => Ok(Self::Madeira),
                    "portalegre district" => Ok(Self::PortalegreDistrict),
                    "porto district" => Ok(Self::PortoDistrict),
                    "santarem district" => Ok(Self::SantaremDistrict),
                    "setubal district" => Ok(Self::SetubalDistrict),
                    "viana do castelo district" => Ok(Self::VianaDoCasteloDistrict),
                    "vila real district" => Ok(Self::VilaRealDistrict),
                    "viseu district" => Ok(Self::ViseuDistrict),
                    "evora district" => Ok(Self::EvoraDistrict),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for SwitzerlandStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "SwitzerlandStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "aargau" => Ok(Self::Aargau),
                    "appenzell ausserrhoden" => Ok(Self::AppenzellAusserrhoden),
                    "appenzell innerrhoden" => Ok(Self::AppenzellInnerrhoden),
                    "basel landschaft" => Ok(Self::BaselLandschaft),
                    "canton of fribourg" => Ok(Self::CantonOfFribourg),
                    "canton of geneva" => Ok(Self::CantonOfGeneva),
                    "canton of jura" => Ok(Self::CantonOfJura),
                    "canton of lucerne" => Ok(Self::CantonOfLucerne),
                    "canton of neuchatel" => Ok(Self::CantonOfNeuchatel),
                    "canton of schaffhausen" => Ok(Self::CantonOfSchaffhausen),
                    "canton of solothurn" => Ok(Self::CantonOfSolothurn),
                    "canton of st gallen" => Ok(Self::CantonOfStGallen),
                    "canton of valais" => Ok(Self::CantonOfValais),
                    "canton of vaud" => Ok(Self::CantonOfVaud),
                    "canton of zug" => Ok(Self::CantonOfZug),
                    "glarus" => Ok(Self::Glarus),
                    "graubunden" => Ok(Self::Graubunden),
                    "nidwalden" => Ok(Self::Nidwalden),
                    "obwalden" => Ok(Self::Obwalden),
                    "schwyz" => Ok(Self::Schwyz),
                    "thurgau" => Ok(Self::Thurgau),
                    "ticino" => Ok(Self::Ticino),
                    "uri" => Ok(Self::Uri),
                    "canton of bern" => Ok(Self::CantonOfBern),
                    "canton of zurich" => Ok(Self::CantonOfZurich),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for NorthMacedoniaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "NorthMacedoniaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "aerodrom municipality" => Ok(Self::AerodromMunicipality),
                    "aracinovo municipality" => Ok(Self::AracinovoMunicipality),
                    "berovo municipality" => Ok(Self::BerovoMunicipality),
                    "bitola municipality" => Ok(Self::BitolaMunicipality),
                    "bogdanci municipality" => Ok(Self::BogdanciMunicipality),
                    "bogovinje municipality" => Ok(Self::BogovinjeMunicipality),
                    "bosilovo municipality" => Ok(Self::BosilovoMunicipality),
                    "brvenica municipality" => Ok(Self::BrvenicaMunicipality),
                    "butel municipality" => Ok(Self::ButelMunicipality),
                    "centar municipality" => Ok(Self::CentarMunicipality),
                    "centar zupa municipality" => Ok(Self::CentarZupaMunicipality),
                    "debarca municipality" => Ok(Self::DebarcaMunicipality),
                    "delcevo municipality" => Ok(Self::DelcevoMunicipality),
                    "demir hisar municipality" => Ok(Self::DemirHisarMunicipality),
                    "demir kapija municipality" => Ok(Self::DemirKapijaMunicipality),
                    "dojran municipality" => Ok(Self::DojranMunicipality),
                    "dolneni municipality" => Ok(Self::DolneniMunicipality),
                    "drugovo municipality" => Ok(Self::DrugovoMunicipality),
                    "gazi baba municipality" => Ok(Self::GaziBabaMunicipality),
                    "gevgelija municipality" => Ok(Self::GevgelijaMunicipality),
                    "gjorce petrov municipality" => Ok(Self::GjorcePetrovMunicipality),
                    "gostivar municipality" => Ok(Self::GostivarMunicipality),
                    "gradsko municipality" => Ok(Self::GradskoMunicipality),
                    "greater skopje" => Ok(Self::GreaterSkopje),
                    "ilinden municipality" => Ok(Self::IlindenMunicipality),
                    "jegunovce municipality" => Ok(Self::JegunovceMunicipality),
                    "karbinci" => Ok(Self::Karbinci),
                    "karpos municipality" => Ok(Self::KarposMunicipality),
                    "kavadarci municipality" => Ok(Self::KavadarciMunicipality),
                    "kisela voda municipality" => Ok(Self::KiselaVodaMunicipality),
                    "kicevo municipality" => Ok(Self::KicevoMunicipality),
                    "konce municipality" => Ok(Self::KonceMunicipality),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for MontenegroStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "MontenegroStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "andrijevica municipality" => Ok(Self::AndrijevicaMunicipality),
                    "bar municipality" => Ok(Self::BarMunicipality),
                    "berane municipality" => Ok(Self::BeraneMunicipality),
                    "bijelo polje municipality" => Ok(Self::BijeloPoljeMunicipality),
                    "budva municipality" => Ok(Self::BudvaMunicipality),
                    "danilovgrad municipality" => Ok(Self::DanilovgradMunicipality),
                    "gusinje municipality" => Ok(Self::GusinjeMunicipality),
                    "kolasin municipality" => Ok(Self::KolasinMunicipality),
                    "kotor municipality" => Ok(Self::KotorMunicipality),
                    "mojkovac municipality" => Ok(Self::MojkovacMunicipality),
                    "niksic municipality" => Ok(Self::NiksicMunicipality),
                    "old royal capital cetinje" => Ok(Self::OldRoyalCapitalCetinje),
                    "petnjica municipality" => Ok(Self::PetnjicaMunicipality),
                    "plav municipality" => Ok(Self::PlavMunicipality),
                    "pljevlja municipality" => Ok(Self::PljevljaMunicipality),
                    "pluzine municipality" => Ok(Self::PluzineMunicipality),
                    "podgorica municipality" => Ok(Self::PodgoricaMunicipality),
                    "rozaje municipality" => Ok(Self::RozajeMunicipality),
                    "tivat municipality" => Ok(Self::TivatMunicipality),
                    "ulcinj municipality" => Ok(Self::UlcinjMunicipality),
                    "savnik municipality" => Ok(Self::SavnikMunicipality),
                    "zabljak municipality" => Ok(Self::ZabljakMunicipality),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for MonacoStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "MonacoStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "monaco" => Ok(Self::Monaco),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for NetherlandsStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "NetherlandsStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "bonaire" => Ok(Self::Bonaire),
                    "drenthe" => Ok(Self::Drenthe),
                    "flevoland" => Ok(Self::Flevoland),
                    "friesland" => Ok(Self::Friesland),
                    "gelderland" => Ok(Self::Gelderland),
                    "groningen" => Ok(Self::Groningen),
                    "limburg" => Ok(Self::Limburg),
                    "north brabant" => Ok(Self::NorthBrabant),
                    "north holland" => Ok(Self::NorthHolland),
                    "overijssel" => Ok(Self::Overijssel),
                    "saba" => Ok(Self::Saba),
                    "sint eustatius" => Ok(Self::SintEustatius),
                    "south holland" => Ok(Self::SouthHolland),
                    "utrecht" => Ok(Self::Utrecht),
                    "zeeland" => Ok(Self::Zeeland),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for MoldovaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "MoldovaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "anenii noi district" => Ok(Self::AneniiNoiDistrict),
                    "basarabeasca district" => Ok(Self::BasarabeascaDistrict),
                    "bender municipality" => Ok(Self::BenderMunicipality),
                    "briceni district" => Ok(Self::BriceniDistrict),
                    "balti municipality" => Ok(Self::BaltiMunicipality),
                    "cahul district" => Ok(Self::CahulDistrict),
                    "cantemir district" => Ok(Self::CantemirDistrict),
                    "chisinau municipality" => Ok(Self::ChisinauMunicipality),
                    "cimislia district" => Ok(Self::CimisliaDistrict),
                    "criuleni district" => Ok(Self::CriuleniDistrict),
                    "calarasi district" => Ok(Self::CalarasiDistrict),
                    "causeni district" => Ok(Self::CauseniDistrict),
                    "donduseni district" => Ok(Self::DonduseniDistrict),
                    "drochia district" => Ok(Self::DrochiaDistrict),
                    "dubasari district" => Ok(Self::DubasariDistrict),
                    "edinet district" => Ok(Self::EdinetDistrict),
                    "floresti district" => Ok(Self::FlorestiDistrict),
                    "falesti district" => Ok(Self::FalestiDistrict),
                    "gagauzia" => Ok(Self::Gagauzia),
                    "glodeni district" => Ok(Self::GlodeniDistrict),
                    "hincesti district" => Ok(Self::HincestiDistrict),
                    "ialoveni district" => Ok(Self::IaloveniDistrict),
                    "nisporeni district" => Ok(Self::NisporeniDistrict),
                    "ocnita district" => Ok(Self::OcnitaDistrict),
                    "orhei district" => Ok(Self::OrheiDistrict),
                    "rezina district" => Ok(Self::RezinaDistrict),
                    "riscani district" => Ok(Self::RiscaniDistrict),
                    "soroca district" => Ok(Self::SorocaDistrict),
                    "straseni district" => Ok(Self::StraseniDistrict),
                    "singerei district" => Ok(Self::SingereiDistrict),
                    "taraclia district" => Ok(Self::TaracliaDistrict),
                    "telenesti district" => Ok(Self::TelenestiDistrict),
                    "transnistria autonomous territorial unit" => {
                        Ok(Self::TransnistriaAutonomousTerritorialUnit)
                    }
                    "ungheni district" => Ok(Self::UngheniDistrict),
                    "soldanesti district" => Ok(Self::SoldanestiDistrict),
                    "stefan voda district" => Ok(Self::StefanVodaDistrict),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for LithuaniaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "LithuaniaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "akmene district municipality" => Ok(Self::AkmeneDistrictMunicipality),
                    "alytus city municipality" => Ok(Self::AlytusCityMunicipality),
                    "alytus county" => Ok(Self::AlytusCounty),
                    "alytus district municipality" => Ok(Self::AlytusDistrictMunicipality),
                    "birstonas municipality" => Ok(Self::BirstonasMunicipality),
                    "birzai district municipality" => Ok(Self::BirzaiDistrictMunicipality),
                    "druskininkai municipality" => Ok(Self::DruskininkaiMunicipality),
                    "elektrenai municipality" => Ok(Self::ElektrenaiMunicipality),
                    "ignalina district municipality" => Ok(Self::IgnalinaDistrictMunicipality),
                    "jonava district municipality" => Ok(Self::JonavaDistrictMunicipality),
                    "joniskis district municipality" => Ok(Self::JoniskisDistrictMunicipality),
                    "jurbarkas district municipality" => Ok(Self::JurbarkasDistrictMunicipality),
                    "kaisiadorys district municipality" => {
                        Ok(Self::KaisiadorysDistrictMunicipality)
                    }
                    "kalvarija municipality" => Ok(Self::KalvarijaMunicipality),
                    "kaunas city municipality" => Ok(Self::KaunasCityMunicipality),
                    "kaunas county" => Ok(Self::KaunasCounty),
                    "kaunas district municipality" => Ok(Self::KaunasDistrictMunicipality),
                    "kazlu ruda municipality" => Ok(Self::KazluRudaMunicipality),
                    "kelme district municipality" => Ok(Self::KelmeDistrictMunicipality),
                    "klaipeda city municipality" => Ok(Self::KlaipedaCityMunicipality),
                    "klaipeda county" => Ok(Self::KlaipedaCounty),
                    "klaipeda district municipality" => Ok(Self::KlaipedaDistrictMunicipality),
                    "kretinga district municipality" => Ok(Self::KretingaDistrictMunicipality),
                    "kupiskis district municipality" => Ok(Self::KupiskisDistrictMunicipality),
                    "kedainiai district municipality" => Ok(Self::KedainiaiDistrictMunicipality),
                    "lazdijai district municipality" => Ok(Self::LazdijaiDistrictMunicipality),
                    "marijampole county" => Ok(Self::MarijampoleCounty),
                    "marijampole municipality" => Ok(Self::MarijampoleMunicipality),
                    "mazeikiai district municipality" => Ok(Self::MazeikiaiDistrictMunicipality),
                    "moletai district municipality" => Ok(Self::MoletaiDistrictMunicipality),
                    "neringa municipality" => Ok(Self::NeringaMunicipality),
                    "pagegiai municipality" => Ok(Self::PagegiaiMunicipality),
                    "pakruojis district municipality" => Ok(Self::PakruojisDistrictMunicipality),
                    "palanga city municipality" => Ok(Self::PalangaCityMunicipality),
                    "panevezys city municipality" => Ok(Self::PanevezysCityMunicipality),
                    "panevezys county" => Ok(Self::PanevezysCounty),
                    "panevezys district municipality" => Ok(Self::PanevezysDistrictMunicipality),
                    "pasvalys district municipality" => Ok(Self::PasvalysDistrictMunicipality),
                    "plunge district municipality" => Ok(Self::PlungeDistrictMunicipality),
                    "prienai district municipality" => Ok(Self::PrienaiDistrictMunicipality),
                    "radviliskis district municipality" => {
                        Ok(Self::RadviliskisDistrictMunicipality)
                    }
                    "raseiniai district municipality" => Ok(Self::RaseiniaiDistrictMunicipality),
                    "rietavas municipality" => Ok(Self::RietavasMunicipality),
                    "rokiskis district municipality" => Ok(Self::RokiskisDistrictMunicipality),
                    "skuodas district municipality" => Ok(Self::SkuodasDistrictMunicipality),
                    "taurage county" => Ok(Self::TaurageCounty),
                    "taurage district municipality" => Ok(Self::TaurageDistrictMunicipality),
                    "telsiai county" => Ok(Self::TelsiaiCounty),
                    "telsiai district municipality" => Ok(Self::TelsiaiDistrictMunicipality),
                    "trakai district municipality" => Ok(Self::TrakaiDistrictMunicipality),
                    "ukmerge district municipality" => Ok(Self::UkmergeDistrictMunicipality),
                    "utena county" => Ok(Self::UtenaCounty),
                    "utena district municipality" => Ok(Self::UtenaDistrictMunicipality),
                    "varena district municipality" => Ok(Self::VarenaDistrictMunicipality),
                    "vilkaviskis district municipality" => {
                        Ok(Self::VilkaviskisDistrictMunicipality)
                    }
                    "vilnius city municipality" => Ok(Self::VilniusCityMunicipality),
                    "vilnius county" => Ok(Self::VilniusCounty),
                    "vilnius district municipality" => Ok(Self::VilniusDistrictMunicipality),
                    "visaginas municipality" => Ok(Self::VisaginasMunicipality),
                    "zarasai district municipality" => Ok(Self::ZarasaiDistrictMunicipality),
                    "sakiai district municipality" => Ok(Self::SakiaiDistrictMunicipality),
                    "salcininkai district municipality" => {
                        Ok(Self::SalcininkaiDistrictMunicipality)
                    }
                    "siauliai city municipality" => Ok(Self::SiauliaiCityMunicipality),
                    "siauliai county" => Ok(Self::SiauliaiCounty),
                    "siauliai district municipality" => Ok(Self::SiauliaiDistrictMunicipality),
                    "silale district municipality" => Ok(Self::SilaleDistrictMunicipality),
                    "silute district municipality" => Ok(Self::SiluteDistrictMunicipality),
                    "sirvintos district municipality" => Ok(Self::SirvintosDistrictMunicipality),
                    "svencionys district municipality" => Ok(Self::SvencionysDistrictMunicipality),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for LiechtensteinStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "LiechtensteinStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "balzers" => Ok(Self::Balzers),
                    "eschen" => Ok(Self::Eschen),
                    "gamprin" => Ok(Self::Gamprin),
                    "mauren" => Ok(Self::Mauren),
                    "planken" => Ok(Self::Planken),
                    "ruggell" => Ok(Self::Ruggell),
                    "schaan" => Ok(Self::Schaan),
                    "schellenberg" => Ok(Self::Schellenberg),
                    "triesen" => Ok(Self::Triesen),
                    "triesenberg" => Ok(Self::Triesenberg),
                    "vaduz" => Ok(Self::Vaduz),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for LatviaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "LatviaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "aglona municipality" => Ok(Self::AglonaMunicipality),
                    "aizkraukle municipality" => Ok(Self::AizkraukleMunicipality),
                    "aizpute municipality" => Ok(Self::AizputeMunicipality),
                    "akniiste municipality" => Ok(Self::AknīsteMunicipality),
                    "aloja municipality" => Ok(Self::AlojaMunicipality),
                    "alsunga municipality" => Ok(Self::AlsungaMunicipality),
                    "aluksne municipality" => Ok(Self::AlūksneMunicipality),
                    "amata municipality" => Ok(Self::AmataMunicipality),
                    "ape municipality" => Ok(Self::ApeMunicipality),
                    "auce municipality" => Ok(Self::AuceMunicipality),
                    "babite municipality" => Ok(Self::BabīteMunicipality),
                    "baldone municipality" => Ok(Self::BaldoneMunicipality),
                    "baltinava municipality" => Ok(Self::BaltinavaMunicipality),
                    "balvi municipality" => Ok(Self::BalviMunicipality),
                    "bauska municipality" => Ok(Self::BauskaMunicipality),
                    "beverina municipality" => Ok(Self::BeverīnaMunicipality),
                    "broceni municipality" => Ok(Self::BrocēniMunicipality),
                    "burtnieki municipality" => Ok(Self::BurtniekiMunicipality),
                    "carnikava municipality" => Ok(Self::CarnikavaMunicipality),
                    "cesvaine municipality" => Ok(Self::CesvaineMunicipality),
                    "cibla municipality" => Ok(Self::CiblaMunicipality),
                    "cesis municipality" => Ok(Self::CēsisMunicipality),
                    "dagda municipality" => Ok(Self::DagdaMunicipality),
                    "daugavpils" => Ok(Self::Daugavpils),
                    "daugavpils municipality" => Ok(Self::DaugavpilsMunicipality),
                    "dobele municipality" => Ok(Self::DobeleMunicipality),
                    "dundaga municipality" => Ok(Self::DundagaMunicipality),
                    "durbe municipality" => Ok(Self::DurbeMunicipality),
                    "engure municipality" => Ok(Self::EngureMunicipality),
                    "garkalne municipality" => Ok(Self::GarkalneMunicipality),
                    "grobina municipality" => Ok(Self::GrobiņaMunicipality),
                    "gulbene municipality" => Ok(Self::GulbeneMunicipality),
                    "iecava municipality" => Ok(Self::IecavaMunicipality),
                    "ikskile municipality" => Ok(Self::IkšķileMunicipality),
                    "ilukste municipality" => Ok(Self::IlūksteMunicipality),
                    "incukalns municipality" => Ok(Self::InčukalnsMunicipality),
                    "jaunjelgava municipality" => Ok(Self::JaunjelgavaMunicipality),
                    "jaunpiebalga municipality" => Ok(Self::JaunpiebalgaMunicipality),
                    "jaunpils municipality" => Ok(Self::JaunpilsMunicipality),
                    "jelgava" => Ok(Self::Jelgava),
                    "jelgava municipality" => Ok(Self::JelgavaMunicipality),
                    "jekabpils" => Ok(Self::Jēkabpils),
                    "jekabpils municipality" => Ok(Self::JēkabpilsMunicipality),
                    "jurmala" => Ok(Self::Jūrmala),
                    "kandava municipality" => Ok(Self::KandavaMunicipality),
                    "koceni municipality" => Ok(Self::KocēniMunicipality),
                    "koknese municipality" => Ok(Self::KokneseMunicipality),
                    "krimulda municipality" => Ok(Self::KrimuldaMunicipality),
                    "kustpils municipality" => Ok(Self::KrustpilsMunicipality),
                    "kraslava municipality" => Ok(Self::KrāslavaMunicipality),
                    "kuldiga municipality" => Ok(Self::KuldīgaMunicipality),
                    "karsava municipality" => Ok(Self::KārsavaMunicipality),
                    "lielvarde municipality" => Ok(Self::LielvārdeMunicipality),
                    "liepaja" => Ok(Self::Liepāja),
                    "limbazi municipality" => Ok(Self::LimbažiMunicipality),
                    "lubana municipality" => Ok(Self::LubānaMunicipality),
                    "ludza municipality" => Ok(Self::LudzaMunicipality),
                    "ligatne municipality" => Ok(Self::LīgatneMunicipality),
                    "livani municipality" => Ok(Self::LīvāniMunicipality),
                    "madona municipality" => Ok(Self::MadonaMunicipality),
                    "mazsalaca municipality" => Ok(Self::MazsalacaMunicipality),
                    "malpils municipality" => Ok(Self::MālpilsMunicipality),
                    "marupe municipality" => Ok(Self::MārupeMunicipality),
                    "mersrags municipality" => Ok(Self::MērsragsMunicipality),
                    "naukseni municipality" => Ok(Self::NaukšēniMunicipality),
                    "nereta municipality" => Ok(Self::NeretaMunicipality),
                    "nica municipality" => Ok(Self::NīcaMunicipality),
                    "ogre municipality" => Ok(Self::OgreMunicipality),
                    "olaine municipality" => Ok(Self::OlaineMunicipality),
                    "ozolnieki municipality" => Ok(Self::OzolniekiMunicipality),
                    "preili municipality" => Ok(Self::PreiļiMunicipality),
                    "priekule municipality" => Ok(Self::PriekuleMunicipality),
                    "priekuli municipality" => Ok(Self::PriekuļiMunicipality),
                    "pargauja municipality" => Ok(Self::PārgaujaMunicipality),
                    "pavilosta municipality" => Ok(Self::PāvilostaMunicipality),
                    "plavinas municipality" => Ok(Self::PļaviņasMunicipality),
                    "rauna municipality" => Ok(Self::RaunaMunicipality),
                    "riebini municipality" => Ok(Self::RiebiņiMunicipality),
                    "riga" => Ok(Self::Riga),
                    "roja municipality" => Ok(Self::RojaMunicipality),
                    "ropazi municipality" => Ok(Self::RopažiMunicipality),
                    "rucava municipality" => Ok(Self::RucavaMunicipality),
                    "rugaji municipality" => Ok(Self::RugājiMunicipality),
                    "rundale municipality" => Ok(Self::RundāleMunicipality),
                    "rezekne" => Ok(Self::Rēzekne),
                    "rezekne municipality" => Ok(Self::RēzekneMunicipality),
                    "rujiena municipality" => Ok(Self::RūjienaMunicipality),
                    "sala municipality" => Ok(Self::SalaMunicipality),
                    "salacgriva municipality" => Ok(Self::SalacgrīvaMunicipality),
                    "salaspils municipality" => Ok(Self::SalaspilsMunicipality),
                    "saldus municipality" => Ok(Self::SaldusMunicipality),
                    "saulkrasti municipality" => Ok(Self::SaulkrastiMunicipality),
                    "sigulda municipality" => Ok(Self::SiguldaMunicipality),
                    "skrunda municipality" => Ok(Self::SkrundaMunicipality),
                    "skriveri municipality" => Ok(Self::SkrīveriMunicipality),
                    "smiltene municipality" => Ok(Self::SmilteneMunicipality),
                    "stopini municipality" => Ok(Self::StopiņiMunicipality),
                    "strenci municipality" => Ok(Self::StrenčiMunicipality),
                    "seja municipality" => Ok(Self::SējaMunicipality),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for MaltaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "MaltaStatesAbbreviation");

        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "attard" => Ok(Self::Attard),
                    "balzan" => Ok(Self::Balzan),
                    "birgu" => Ok(Self::Birgu),
                    "birkirkara" => Ok(Self::Birkirkara),
                    "birzebbuga" => Ok(Self::Birzebbuga),
                    "cospicua" => Ok(Self::Cospicua),
                    "dingli" => Ok(Self::Dingli),
                    "fgura" => Ok(Self::Fgura),
                    "floriana" => Ok(Self::Floriana),
                    "fontana" => Ok(Self::Fontana),
                    "gudja" => Ok(Self::Gudja),
                    "gzira" => Ok(Self::Gzira),
                    "ghajnsielem" => Ok(Self::Ghajnsielem),
                    "gharb" => Ok(Self::Gharb),
                    "gharghur" => Ok(Self::Gharghur),
                    "ghasri" => Ok(Self::Ghasri),
                    "ghaxaq" => Ok(Self::Ghaxaq),
                    "hamrun" => Ok(Self::Hamrun),
                    "iklin" => Ok(Self::Iklin),
                    "senglea" => Ok(Self::Senglea),
                    "kalkara" => Ok(Self::Kalkara),
                    "kercem" => Ok(Self::Kercem),
                    "kirkop" => Ok(Self::Kirkop),
                    "lija" => Ok(Self::Lija),
                    "luqa" => Ok(Self::Luqa),
                    "marsa" => Ok(Self::Marsa),
                    "marsaskala" => Ok(Self::Marsaskala),
                    "marsaxlokk" => Ok(Self::Marsaxlokk),
                    "mdina" => Ok(Self::Mdina),
                    "mellieha" => Ok(Self::Mellieha),
                    "mgarr" => Ok(Self::Mgarr),
                    "mosta" => Ok(Self::Mosta),
                    "mqabba" => Ok(Self::Mqabba),
                    "msida" => Ok(Self::Msida),
                    "mtarfa" => Ok(Self::Mtarfa),
                    "munxar" => Ok(Self::Munxar),
                    "nadur" => Ok(Self::Nadur),
                    "naxxar" => Ok(Self::Naxxar),
                    "paola" => Ok(Self::Paola),
                    "pembroke" => Ok(Self::Pembroke),
                    "pieta" => Ok(Self::Pieta),
                    "qala" => Ok(Self::Qala),
                    "qormi" => Ok(Self::Qormi),
                    "qrendi" => Ok(Self::Qrendi),
                    "victoria" => Ok(Self::Victoria),
                    "rabat" => Ok(Self::Rabat),
                    "st julians" => Ok(Self::StJulians),
                    "san gwann" => Ok(Self::SanGwann),
                    "saint lawrence" => Ok(Self::SaintLawrence),
                    "st pauls bay" => Ok(Self::StPaulsBay),
                    "sannat" => Ok(Self::Sannat),
                    "santa lucija" => Ok(Self::SantaLucija),
                    "santa venera" => Ok(Self::SantaVenera),
                    "siggiewi" => Ok(Self::Siggiewi),
                    "sliema" => Ok(Self::Sliema),
                    "swieqi" => Ok(Self::Swieqi),
                    "ta xbiex" => Ok(Self::TaXbiex),
                    "tarxien" => Ok(Self::Tarxien),
                    "valletta" => Ok(Self::Valletta),
                    "xaghra" => Ok(Self::Xaghra),
                    "xewkija" => Ok(Self::Xewkija),
                    "xghajra" => Ok(Self::Xghajra),
                    "zabbar" => Ok(Self::Zabbar),
                    "zebbug gozo" => Ok(Self::ZebbugGozo),
                    "zebbug malta" => Ok(Self::ZebbugMalta),
                    "zejtun" => Ok(Self::Zejtun),
                    "zurrieq" => Ok(Self::Zurrieq),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for BelarusStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "BelarusStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "brest region" => Ok(Self::BrestRegion),
                    "gomel region" => Ok(Self::GomelRegion),
                    "grodno region" => Ok(Self::GrodnoRegion),
                    "minsk" => Ok(Self::Minsk),
                    "minsk region" => Ok(Self::MinskRegion),
                    "mogilev region" => Ok(Self::MogilevRegion),
                    "vitebsk region" => Ok(Self::VitebskRegion),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for IrelandStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "IrelandStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "connacht" => Ok(Self::Connacht),
                    "county carlow" => Ok(Self::CountyCarlow),
                    "county cavan" => Ok(Self::CountyCavan),
                    "county clare" => Ok(Self::CountyClare),
                    "county cork" => Ok(Self::CountyCork),
                    "county donegal" => Ok(Self::CountyDonegal),
                    "county dublin" => Ok(Self::CountyDublin),
                    "county galway" => Ok(Self::CountyGalway),
                    "county kerry" => Ok(Self::CountyKerry),
                    "county kildare" => Ok(Self::CountyKildare),
                    "county kilkenny" => Ok(Self::CountyKilkenny),
                    "county laois" => Ok(Self::CountyLaois),
                    "county limerick" => Ok(Self::CountyLimerick),
                    "county longford" => Ok(Self::CountyLongford),
                    "county louth" => Ok(Self::CountyLouth),
                    "county mayo" => Ok(Self::CountyMayo),
                    "county meath" => Ok(Self::CountyMeath),
                    "county monaghan" => Ok(Self::CountyMonaghan),
                    "county offaly" => Ok(Self::CountyOffaly),
                    "county roscommon" => Ok(Self::CountyRoscommon),
                    "county sligo" => Ok(Self::CountySligo),
                    "county tipperary" => Ok(Self::CountyTipperary),
                    "county waterford" => Ok(Self::CountyWaterford),
                    "county westmeath" => Ok(Self::CountyWestmeath),
                    "county wexford" => Ok(Self::CountyWexford),
                    "county wicklow" => Ok(Self::CountyWicklow),
                    "leinster" => Ok(Self::Leinster),
                    "munster" => Ok(Self::Munster),
                    "ulster" => Ok(Self::Ulster),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for IcelandStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "IcelandStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "capital region" => Ok(Self::CapitalRegion),
                    "eastern region" => Ok(Self::EasternRegion),
                    "northeastern region" => Ok(Self::NortheasternRegion),
                    "northwestern region" => Ok(Self::NorthwesternRegion),
                    "southern peninsula region" => Ok(Self::SouthernPeninsulaRegion),
                    "southern region" => Ok(Self::SouthernRegion),
                    "western region" => Ok(Self::WesternRegion),
                    "westfjords" => Ok(Self::Westfjords),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for HungaryStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "HungaryStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "baranya county" => Ok(Self::BaranyaCounty),
                    "borsod abauj zemplen county" => Ok(Self::BorsodAbaujZemplenCounty),
                    "budapest" => Ok(Self::Budapest),
                    "bacs kiskun county" => Ok(Self::BacsKiskunCounty),
                    "bekes county" => Ok(Self::BekesCounty),
                    "bekescsaba" => Ok(Self::Bekescsaba),
                    "csongrad county" => Ok(Self::CsongradCounty),
                    "debrecen" => Ok(Self::Debrecen),
                    "dunaujvaros" => Ok(Self::Dunaujvaros),
                    "eger" => Ok(Self::Eger),
                    "fejer county" => Ok(Self::FejerCounty),
                    "gyor" => Ok(Self::Gyor),
                    "gyor moson sopron county" => Ok(Self::GyorMosonSopronCounty),
                    "hajdu bihar county" => Ok(Self::HajduBiharCounty),
                    "heves county" => Ok(Self::HevesCounty),
                    "hodmezovasarhely" => Ok(Self::Hodmezovasarhely),
                    "jasz nagykun szolnok county" => Ok(Self::JaszNagykunSzolnokCounty),
                    "kaposvar" => Ok(Self::Kaposvar),
                    "kecskemet" => Ok(Self::Kecskemet),
                    "miskolc" => Ok(Self::Miskolc),
                    "nagykanizsa" => Ok(Self::Nagykanizsa),
                    "nyiregyhaza" => Ok(Self::Nyiregyhaza),
                    "nograd county" => Ok(Self::NogradCounty),
                    "pest county" => Ok(Self::PestCounty),
                    "pecs" => Ok(Self::Pecs),
                    "salgotarjan" => Ok(Self::Salgotarjan),
                    "somogy county" => Ok(Self::SomogyCounty),
                    "sopron" => Ok(Self::Sopron),
                    "szabolcs szatmar bereg county" => Ok(Self::SzabolcsSzatmarBeregCounty),
                    "szeged" => Ok(Self::Szeged),
                    "szekszard" => Ok(Self::Szekszard),
                    "szolnok" => Ok(Self::Szolnok),
                    "szombathely" => Ok(Self::Szombathely),
                    "szekesfehervar" => Ok(Self::Szekesfehervar),
                    "tatabanya" => Ok(Self::Tatabanya),
                    "tolna county" => Ok(Self::TolnaCounty),
                    "vas county" => Ok(Self::VasCounty),
                    "veszprem" => Ok(Self::Veszprem),
                    "veszprem county" => Ok(Self::VeszpremCounty),
                    "zala county" => Ok(Self::ZalaCounty),
                    "zalaegerszeg" => Ok(Self::Zalaegerszeg),
                    "erd" => Ok(Self::Erd),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for GreeceStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "GreeceStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "achaea regional unit" => Ok(Self::AchaeaRegionalUnit),
                    "aetolia acarnania regional unit" => Ok(Self::AetoliaAcarnaniaRegionalUnit),
                    "arcadia prefecture" => Ok(Self::ArcadiaPrefecture),
                    "argolis regional unit" => Ok(Self::ArgolisRegionalUnit),
                    "attica region" => Ok(Self::AtticaRegion),
                    "boeotia regional unit" => Ok(Self::BoeotiaRegionalUnit),
                    "central greece region" => Ok(Self::CentralGreeceRegion),
                    "central macedonia" => Ok(Self::CentralMacedonia),
                    "chania regional unit" => Ok(Self::ChaniaRegionalUnit),
                    "corfu prefecture" => Ok(Self::CorfuPrefecture),
                    "corinthia regional unit" => Ok(Self::CorinthiaRegionalUnit),
                    "crete region" => Ok(Self::CreteRegion),
                    "drama regional unit" => Ok(Self::DramaRegionalUnit),
                    "east attica regional unit" => Ok(Self::EastAtticaRegionalUnit),
                    "east macedonia and thrace" => Ok(Self::EastMacedoniaAndThrace),
                    "epirus region" => Ok(Self::EpirusRegion),
                    "euboea" => Ok(Self::Euboea),
                    "grevena prefecture" => Ok(Self::GrevenaPrefecture),
                    "imathia regional unit" => Ok(Self::ImathiaRegionalUnit),
                    "ioannina regional unit" => Ok(Self::IoanninaRegionalUnit),
                    "ionian islands region" => Ok(Self::IonianIslandsRegion),
                    "karditsa regional unit" => Ok(Self::KarditsaRegionalUnit),
                    "kastoria regional unit" => Ok(Self::KastoriaRegionalUnit),
                    "kefalonia prefecture" => Ok(Self::KefaloniaPrefecture),
                    "kilkis regional unit" => Ok(Self::KilkisRegionalUnit),
                    "kozani prefecture" => Ok(Self::KozaniPrefecture),
                    "laconia" => Ok(Self::Laconia),
                    "larissa prefecture" => Ok(Self::LarissaPrefecture),
                    "lefkada regional unit" => Ok(Self::LefkadaRegionalUnit),
                    "pella regional unit" => Ok(Self::PellaRegionalUnit),
                    "peloponnese region" => Ok(Self::PeloponneseRegion),
                    "phthiotis prefecture" => Ok(Self::PhthiotisPrefecture),
                    "preveza prefecture" => Ok(Self::PrevezaPrefecture),
                    "serres prefecture" => Ok(Self::SerresPrefecture),
                    "south aegean" => Ok(Self::SouthAegean),
                    "thessaloniki regional unit" => Ok(Self::ThessalonikiRegionalUnit),
                    "west greece region" => Ok(Self::WestGreeceRegion),
                    "west macedonia region" => Ok(Self::WestMacedoniaRegion),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for FinlandStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "FinlandStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "central finland" => Ok(Self::CentralFinland),
                    "central ostrobothnia" => Ok(Self::CentralOstrobothnia),
                    "eastern finland province" => Ok(Self::EasternFinlandProvince),
                    "finland proper" => Ok(Self::FinlandProper),
                    "kainuu" => Ok(Self::Kainuu),
                    "kymenlaakso" => Ok(Self::Kymenlaakso),
                    "lapland" => Ok(Self::Lapland),
                    "north karelia" => Ok(Self::NorthKarelia),
                    "northern ostrobothnia" => Ok(Self::NorthernOstrobothnia),
                    "northern savonia" => Ok(Self::NorthernSavonia),
                    "ostrobothnia" => Ok(Self::Ostrobothnia),
                    "oulu province" => Ok(Self::OuluProvince),
                    "pirkanmaa" => Ok(Self::Pirkanmaa),
                    "paijanne tavastia" => Ok(Self::PaijanneTavastia),
                    "satakunta" => Ok(Self::Satakunta),
                    "south karelia" => Ok(Self::SouthKarelia),
                    "southern ostrobothnia" => Ok(Self::SouthernOstrobothnia),
                    "southern savonia" => Ok(Self::SouthernSavonia),
                    "tavastia proper" => Ok(Self::TavastiaProper),
                    "uusimaa" => Ok(Self::Uusimaa),
                    "aland islands" => Ok(Self::AlandIslands),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for DenmarkStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "DenmarkStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "capital region of denmark" => Ok(Self::CapitalRegionOfDenmark),
                    "central denmark region" => Ok(Self::CentralDenmarkRegion),
                    "north denmark region" => Ok(Self::NorthDenmarkRegion),
                    "region zealand" => Ok(Self::RegionZealand),
                    "region of southern denmark" => Ok(Self::RegionOfSouthernDenmark),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for CzechRepublicStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "CzechRepublicStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "benesov district" => Ok(Self::BenesovDistrict),
                    "beroun district" => Ok(Self::BerounDistrict),
                    "blansko district" => Ok(Self::BlanskoDistrict),
                    "brno city district" => Ok(Self::BrnoCityDistrict),
                    "brno country district" => Ok(Self::BrnoCountryDistrict),
                    "bruntal district" => Ok(Self::BruntalDistrict),
                    "breclav district" => Ok(Self::BreclavDistrict),
                    "central bohemian region" => Ok(Self::CentralBohemianRegion),
                    "cheb district" => Ok(Self::ChebDistrict),
                    "chomutov district" => Ok(Self::ChomutovDistrict),
                    "chrudim district" => Ok(Self::ChrudimDistrict),
                    "domazlice district" => Ok(Self::DomazliceDistrict),
                    "decin district" => Ok(Self::DecinDistrict),
                    "frydek mistek district" => Ok(Self::FrydekMistekDistrict),
                    "havlickuv brod district" => Ok(Self::HavlickuvBrodDistrict),
                    "hodonin district" => Ok(Self::HodoninDistrict),
                    "horni pocernice" => Ok(Self::HorniPocernice),
                    "hradec kralove district" => Ok(Self::HradecKraloveDistrict),
                    "hradec kralove region" => Ok(Self::HradecKraloveRegion),
                    "jablonec nad nisou district" => Ok(Self::JablonecNadNisouDistrict),
                    "jesenik district" => Ok(Self::JesenikDistrict),
                    "jihlava district" => Ok(Self::JihlavaDistrict),
                    "jindrichuv hradec district" => Ok(Self::JindrichuvHradecDistrict),
                    "jicin district" => Ok(Self::JicinDistrict),
                    "karlovy vary district" => Ok(Self::KarlovyVaryDistrict),
                    "karlovy vary region" => Ok(Self::KarlovyVaryRegion),
                    "karvina district" => Ok(Self::KarvinaDistrict),
                    "kladno district" => Ok(Self::KladnoDistrict),
                    "klatovy district" => Ok(Self::KlatovyDistrict),
                    "kolin district" => Ok(Self::KolinDistrict),
                    "kromeriz district" => Ok(Self::KromerizDistrict),
                    "liberec district" => Ok(Self::LiberecDistrict),
                    "liberec region" => Ok(Self::LiberecRegion),
                    "litomerice district" => Ok(Self::LitomericeDistrict),
                    "louny district" => Ok(Self::LounyDistrict),
                    "mlada boleslav district" => Ok(Self::MladaBoleslavDistrict),
                    "moravian silesian region" => Ok(Self::MoravianSilesianRegion),
                    "most district" => Ok(Self::MostDistrict),
                    "melnik district" => Ok(Self::MelnikDistrict),
                    "novy jicin district" => Ok(Self::NovyJicinDistrict),
                    "nymburk district" => Ok(Self::NymburkDistrict),
                    "nachod district" => Ok(Self::NachodDistrict),
                    "olomouc district" => Ok(Self::OlomoucDistrict),
                    "olomouc region" => Ok(Self::OlomoucRegion),
                    "opava district" => Ok(Self::OpavaDistrict),
                    "ostrava city district" => Ok(Self::OstravaCityDistrict),
                    "pardubice district" => Ok(Self::PardubiceDistrict),
                    "pardubice region" => Ok(Self::PardubiceRegion),
                    "pelhrimov district" => Ok(Self::PelhrimovDistrict),
                    "plzen region" => Ok(Self::PlzenRegion),
                    "plzen city district" => Ok(Self::PlzenCityDistrict),
                    "plzen north district" => Ok(Self::PlzenNorthDistrict),
                    "plzen south district" => Ok(Self::PlzenSouthDistrict),
                    "prachatice district" => Ok(Self::PrachaticeDistrict),
                    "prague" => Ok(Self::Prague),
                    "prague1" => Ok(Self::Prague1),
                    "prague10" => Ok(Self::Prague10),
                    "prague11" => Ok(Self::Prague11),
                    "prague12" => Ok(Self::Prague12),
                    "prague13" => Ok(Self::Prague13),
                    "prague14" => Ok(Self::Prague14),
                    "prague15" => Ok(Self::Prague15),
                    "prague16" => Ok(Self::Prague16),
                    "prague2" => Ok(Self::Prague2),
                    "prague21" => Ok(Self::Prague21),
                    "prague3" => Ok(Self::Prague3),
                    "prague4" => Ok(Self::Prague4),
                    "prague5" => Ok(Self::Prague5),
                    "prague6" => Ok(Self::Prague6),
                    "prague7" => Ok(Self::Prague7),
                    "prague8" => Ok(Self::Prague8),
                    "prague9" => Ok(Self::Prague9),
                    "prague east district" => Ok(Self::PragueEastDistrict),
                    "prague west district" => Ok(Self::PragueWestDistrict),
                    "prostejov district" => Ok(Self::ProstejovDistrict),
                    "pisek district" => Ok(Self::PisekDistrict),
                    "prerov district" => Ok(Self::PrerovDistrict),
                    "pribram district" => Ok(Self::PribramDistrict),
                    "rakovnik district" => Ok(Self::RakovnikDistrict),
                    "rokycany district" => Ok(Self::RokycanyDistrict),
                    "rychnov nad kneznou district" => Ok(Self::RychnovNadKneznouDistrict),
                    "semily district" => Ok(Self::SemilyDistrict),
                    "sokolov district" => Ok(Self::SokolovDistrict),
                    "south bohemian region" => Ok(Self::SouthBohemianRegion),
                    "south moravian region" => Ok(Self::SouthMoravianRegion),
                    "strakonice district" => Ok(Self::StrakoniceDistrict),
                    "svitavy district" => Ok(Self::SvitavyDistrict),
                    "tachov district" => Ok(Self::TachovDistrict),
                    "teplice district" => Ok(Self::TepliceDistrict),
                    "trutnov district" => Ok(Self::TrutnovDistrict),
                    "tabor district" => Ok(Self::TaborDistrict),
                    "trebic district" => Ok(Self::TrebicDistrict),
                    "uherske hradiste district" => Ok(Self::UherskeHradisteDistrict),
                    "vsetin district" => Ok(Self::VsetinDistrict),
                    "vysocina region" => Ok(Self::VysocinaRegion),
                    "vyskov district" => Ok(Self::VyskovDistrict),
                    "zlin district" => Ok(Self::ZlinDistrict),
                    "zlin region" => Ok(Self::ZlinRegion),
                    "znojmo district" => Ok(Self::ZnojmoDistrict),
                    "usti nad labem district" => Ok(Self::UstiNadLabemDistrict),
                    "usti nad labem region" => Ok(Self::UstiNadLabemRegion),
                    "usti nad orlici district" => Ok(Self::UstiNadOrliciDistrict),
                    "ceska lipa district" => Ok(Self::CeskaLipaDistrict),
                    "ceske budejovice district" => Ok(Self::CeskeBudejoviceDistrict),
                    "cesky krumlov district" => Ok(Self::CeskyKrumlovDistrict),
                    "sumperk district" => Ok(Self::SumperkDistrict),
                    "zdar nad sazavou district" => Ok(Self::ZdarNadSazavouDistrict),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for CroatiaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "CroatiaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "bjelovar bilogora county" => Ok(Self::BjelovarBilogoraCounty),
                    "brod posavina county" => Ok(Self::BrodPosavinaCounty),
                    "dubrovnik neretva county" => Ok(Self::DubrovnikNeretvaCounty),
                    "istria county" => Ok(Self::IstriaCounty),
                    "koprivnica krizevci county" => Ok(Self::KoprivnicaKrizevciCounty),
                    "krapina zagorje county" => Ok(Self::KrapinaZagorjeCounty),
                    "lika senj county" => Ok(Self::LikaSenjCounty),
                    "medimurje county" => Ok(Self::MedimurjeCounty),
                    "osijek baranja county" => Ok(Self::OsijekBaranjaCounty),
                    "pozega slavonian county" => Ok(Self::PozegaSlavoniaCounty),
                    "primorje gorski kotar county" => Ok(Self::PrimorjeGorskiKotarCounty),
                    "sisak moslavina county" => Ok(Self::SisakMoslavinaCounty),
                    "split dalmatia county" => Ok(Self::SplitDalmatiaCounty),
                    "varazdin county" => Ok(Self::VarazdinCounty),
                    "virovitica podravina county" => Ok(Self::ViroviticaPodravinaCounty),
                    "vukovar syrmia county" => Ok(Self::VukovarSyrmiaCounty),
                    "zadar county" => Ok(Self::ZadarCounty),
                    "zagreb" => Ok(Self::Zagreb),
                    "zagreb county" => Ok(Self::ZagrebCounty),
                    "sibenik knin county" => Ok(Self::SibenikKninCounty),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for BulgariaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "BulgariaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "blagoevgrad province" => Ok(Self::BlagoevgradProvince),
                    "burgas province" => Ok(Self::BurgasProvince),
                    "dobrich province" => Ok(Self::DobrichProvince),
                    "gabrovo province" => Ok(Self::GabrovoProvince),
                    "haskovo province" => Ok(Self::HaskovoProvince),
                    "kardzhali province" => Ok(Self::KardzhaliProvince),
                    "kyustendil province" => Ok(Self::KyustendilProvince),
                    "lovech province" => Ok(Self::LovechProvince),
                    "montana province" => Ok(Self::MontanaProvince),
                    "pazardzhik province" => Ok(Self::PazardzhikProvince),
                    "pernik province" => Ok(Self::PernikProvince),
                    "pleven province" => Ok(Self::PlevenProvince),
                    "plovdiv province" => Ok(Self::PlovdivProvince),
                    "razgrad province" => Ok(Self::RazgradProvince),
                    "ruse province" => Ok(Self::RuseProvince),
                    "shumen" => Ok(Self::Shumen),
                    "silistra province" => Ok(Self::SilistraProvince),
                    "sliven province" => Ok(Self::SlivenProvince),
                    "smolyan province" => Ok(Self::SmolyanProvince),
                    "sofia city province" => Ok(Self::SofiaCityProvince),
                    "sofia province" => Ok(Self::SofiaProvince),
                    "stara zagora province" => Ok(Self::StaraZagoraProvince),
                    "targovishte province" => Ok(Self::TargovishteProvince),
                    "varna province" => Ok(Self::VarnaProvince),
                    "veliko tarnovo province" => Ok(Self::VelikoTarnovoProvince),
                    "vidin province" => Ok(Self::VidinProvince),
                    "vratsa province" => Ok(Self::VratsaProvince),
                    "yambol province" => Ok(Self::YambolProvince),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for BosniaAndHerzegovinaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "BosniaAndHerzegovinaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "bosnian podrinje canton" => Ok(Self::BosnianPodrinjeCanton),
                    "brcko district" => Ok(Self::BrckoDistrict),
                    "canton 10" => Ok(Self::Canton10),
                    "central bosnia canton" => Ok(Self::CentralBosniaCanton),
                    "federation of bosnia and herzegovina" => {
                        Ok(Self::FederationOfBosniaAndHerzegovina)
                    }
                    "herzegovina neretva canton" => Ok(Self::HerzegovinaNeretvaCanton),
                    "posavina canton" => Ok(Self::PosavinaCanton),
                    "republika srpska" => Ok(Self::RepublikaSrpska),
                    "sarajevo canton" => Ok(Self::SarajevoCanton),
                    "tuzla canton" => Ok(Self::TuzlaCanton),
                    "una sana canton" => Ok(Self::UnaSanaCanton),
                    "west herzegovina canton" => Ok(Self::WestHerzegovinaCanton),
                    "zenica doboj canton" => Ok(Self::ZenicaDobojCanton),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for UnitedKingdomStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "UnitedKingdomStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "aberdeen city" => Ok(Self::AberdeenCity),
                    "aberdeenshire" => Ok(Self::Aberdeenshire),
                    "angus" => Ok(Self::Angus),
                    "antrim and newtownabbey" => Ok(Self::AntrimAndNewtownabbey),
                    "ards and north down" => Ok(Self::ArdsAndNorthDown),
                    "argyll and bute" => Ok(Self::ArgyllAndBute),
                    "armagh city banbridge and craigavon" => {
                        Ok(Self::ArmaghCityBanbridgeAndCraigavon)
                    }
                    "barking and dagenham" => Ok(Self::BarkingAndDagenham),
                    "barnet" => Ok(Self::Barnet),
                    "barnsley" => Ok(Self::Barnsley),
                    "bath and north east somerset" => Ok(Self::BathAndNorthEastSomerset),
                    "bedford" => Ok(Self::Bedford),
                    "belfast city" => Ok(Self::BelfastCity),
                    "bexley" => Ok(Self::Bexley),
                    "birmingham" => Ok(Self::Birmingham),
                    "blackburn with darwen" => Ok(Self::BlackburnWithDarwen),
                    "blackpool" => Ok(Self::Blackpool),
                    "blaenau gwent" => Ok(Self::BlaenauGwent),
                    "bolton" => Ok(Self::Bolton),
                    "bournemouth christchurch and poole" => {
                        Ok(Self::BournemouthChristchurchAndPoole)
                    }
                    "bracknell forest" => Ok(Self::BracknellForest),
                    "bradford" => Ok(Self::Bradford),
                    "brent" => Ok(Self::Brent),
                    "bridgend" => Ok(Self::Bridgend),
                    "brighton and hove" => Ok(Self::BrightonAndHove),
                    "bristol city of" => Ok(Self::BristolCityOf),
                    "bromley" => Ok(Self::Bromley),
                    "buckinghamshire" => Ok(Self::Buckinghamshire),
                    "bury" => Ok(Self::Bury),
                    "caerphilly" => Ok(Self::Caerphilly),
                    "calderdale" => Ok(Self::Calderdale),
                    "cambridgeshire" => Ok(Self::Cambridgeshire),
                    "camden" => Ok(Self::Camden),
                    "cardiff" => Ok(Self::Cardiff),
                    "carmarthenshire" => Ok(Self::Carmarthenshire),
                    "causeway coast and glens" => Ok(Self::CausewayCoastAndGlens),
                    "central bedfordshire" => Ok(Self::CentralBedfordshire),
                    "ceredigion" => Ok(Self::Ceredigion),
                    "cheshire east" => Ok(Self::CheshireEast),
                    "cheshire west and chester" => Ok(Self::CheshireWestAndChester),
                    "clackmannanshire" => Ok(Self::Clackmannanshire),
                    "conwy" => Ok(Self::Conwy),
                    "cornwall" => Ok(Self::Cornwall),
                    "coventry" => Ok(Self::Coventry),
                    "croydon" => Ok(Self::Croydon),
                    "cumbria" => Ok(Self::Cumbria),
                    "darlington" => Ok(Self::Darlington),
                    "denbighshire" => Ok(Self::Denbighshire),
                    "derby" => Ok(Self::Derby),
                    "derbyshire" => Ok(Self::Derbyshire),
                    "derry and strabane" => Ok(Self::DerryAndStrabane),
                    "devon" => Ok(Self::Devon),
                    "doncaster" => Ok(Self::Doncaster),
                    "dorset" => Ok(Self::Dorset),
                    "dudley" => Ok(Self::Dudley),
                    "dumfries and galloway" => Ok(Self::DumfriesAndGalloway),
                    "dundee city" => Ok(Self::DundeeCity),
                    "durham county" => Ok(Self::DurhamCounty),
                    "ealing" => Ok(Self::Ealing),
                    "east ayrshire" => Ok(Self::EastAyrshire),
                    "east dunbartonshire" => Ok(Self::EastDunbartonshire),
                    "east lothian" => Ok(Self::EastLothian),
                    "east renfrewshire" => Ok(Self::EastRenfrewshire),
                    "east riding of yorkshire" => Ok(Self::EastRidingOfYorkshire),
                    "east sussex" => Ok(Self::EastSussex),
                    "edinburgh city of" => Ok(Self::EdinburghCityOf),
                    "eilean siar" => Ok(Self::EileanSiar),
                    "enfield" => Ok(Self::Enfield),
                    "essex" => Ok(Self::Essex),
                    "falkirk" => Ok(Self::Falkirk),
                    "fermanagh and omagh" => Ok(Self::FermanaghAndOmagh),
                    "fife" => Ok(Self::Fife),
                    "flintshire" => Ok(Self::Flintshire),
                    "gateshead" => Ok(Self::Gateshead),
                    "glasgow city" => Ok(Self::GlasgowCity),
                    "gloucestershire" => Ok(Self::Gloucestershire),
                    "greenwich" => Ok(Self::Greenwich),
                    "gwynedd" => Ok(Self::Gwynedd),
                    "hackney" => Ok(Self::Hackney),
                    "halton" => Ok(Self::Halton),
                    "hammersmith and fulham" => Ok(Self::HammersmithAndFulham),
                    "hampshire" => Ok(Self::Hampshire),
                    "haringey" => Ok(Self::Haringey),
                    "harrow" => Ok(Self::Harrow),
                    "hartlepool" => Ok(Self::Hartlepool),
                    "havering" => Ok(Self::Havering),
                    "herefordshire" => Ok(Self::Herefordshire),
                    "hertfordshire" => Ok(Self::Hertfordshire),
                    "highland" => Ok(Self::Highland),
                    "hillingdon" => Ok(Self::Hillingdon),
                    "hounslow" => Ok(Self::Hounslow),
                    "inverclyde" => Ok(Self::Inverclyde),
                    "isle of anglesey" => Ok(Self::IsleOfAnglesey),
                    "isle of wight" => Ok(Self::IsleOfWight),
                    "isles of scilly" => Ok(Self::IslesOfScilly),
                    "islington" => Ok(Self::Islington),
                    "kensington and chelsea" => Ok(Self::KensingtonAndChelsea),
                    "kent" => Ok(Self::Kent),
                    "kingston upon hull" => Ok(Self::KingstonUponHull),
                    "kingston upon thames" => Ok(Self::KingstonUponThames),
                    "kirklees" => Ok(Self::Kirklees),
                    "knowsley" => Ok(Self::Knowsley),
                    "lambeth" => Ok(Self::Lambeth),
                    "lancashire" => Ok(Self::Lancashire),
                    "leeds" => Ok(Self::Leeds),
                    "leicester" => Ok(Self::Leicester),
                    "leicestershire" => Ok(Self::Leicestershire),
                    "lewisham" => Ok(Self::Lewisham),
                    "lincolnshire" => Ok(Self::Lincolnshire),
                    "lisburn and castlereagh" => Ok(Self::LisburnAndCastlereagh),
                    "liverpool" => Ok(Self::Liverpool),
                    "london city of" => Ok(Self::LondonCityOf),
                    "luton" => Ok(Self::Luton),
                    "manchester" => Ok(Self::Manchester),
                    "medway" => Ok(Self::Medway),
                    "merthyr tydfil" => Ok(Self::MerthyrTydfil),
                    "merton" => Ok(Self::Merton),
                    "mid and east antrim" => Ok(Self::MidAndEastAntrim),
                    "mid ulster" => Ok(Self::MidUlster),
                    "middlesbrough" => Ok(Self::Middlesbrough),
                    "midlothian" => Ok(Self::Midlothian),
                    "milton keynes" => Ok(Self::MiltonKeynes),
                    "monmouthshire" => Ok(Self::Monmouthshire),
                    "moray" => Ok(Self::Moray),
                    "neath port talbot" => Ok(Self::NeathPortTalbot),
                    "newcastle upon tyne" => Ok(Self::NewcastleUponTyne),
                    "newham" => Ok(Self::Newham),
                    "newport" => Ok(Self::Newport),
                    "newry mourne and down" => Ok(Self::NewryMourneAndDown),
                    "norfolk" => Ok(Self::Norfolk),
                    "north ayrshire" => Ok(Self::NorthAyrshire),
                    "north east lincolnshire" => Ok(Self::NorthEastLincolnshire),
                    "north lanarkshire" => Ok(Self::NorthLanarkshire),
                    "north lincolnshire" => Ok(Self::NorthLincolnshire),
                    "north somerset" => Ok(Self::NorthSomerset),
                    "north tyneside" => Ok(Self::NorthTyneside),
                    "north yorkshire" => Ok(Self::NorthYorkshire),
                    "northamptonshire" => Ok(Self::Northamptonshire),
                    "northumberland" => Ok(Self::Northumberland),
                    "nottingham" => Ok(Self::Nottingham),
                    "nottinghamshire" => Ok(Self::Nottinghamshire),
                    "oldham" => Ok(Self::Oldham),
                    "orkney islands" => Ok(Self::OrkneyIslands),
                    "oxfordshire" => Ok(Self::Oxfordshire),
                    "pembrokeshire" => Ok(Self::Pembrokeshire),
                    "perth and kinross" => Ok(Self::PerthAndKinross),
                    "peterborough" => Ok(Self::Peterborough),
                    "plymouth" => Ok(Self::Plymouth),
                    "portsmouth" => Ok(Self::Portsmouth),
                    "powys" => Ok(Self::Powys),
                    "reading" => Ok(Self::Reading),
                    "redbridge" => Ok(Self::Redbridge),
                    "redcar and cleveland" => Ok(Self::RedcarAndCleveland),
                    "renfrewshire" => Ok(Self::Renfrewshire),
                    "rhondda cynon taff" => Ok(Self::RhonddaCynonTaff),
                    "richmond upon thames" => Ok(Self::RichmondUponThames),
                    "rochdale" => Ok(Self::Rochdale),
                    "rotherham" => Ok(Self::Rotherham),
                    "rutland" => Ok(Self::Rutland),
                    "salford" => Ok(Self::Salford),
                    "sandwell" => Ok(Self::Sandwell),
                    "scottish borders" => Ok(Self::ScottishBorders),
                    "sefton" => Ok(Self::Sefton),
                    "sheffield" => Ok(Self::Sheffield),
                    "shetland islands" => Ok(Self::ShetlandIslands),
                    "shropshire" => Ok(Self::Shropshire),
                    "slough" => Ok(Self::Slough),
                    "solihull" => Ok(Self::Solihull),
                    "somerset" => Ok(Self::Somerset),
                    "south ayrshire" => Ok(Self::SouthAyrshire),
                    "south gloucestershire" => Ok(Self::SouthGloucestershire),
                    "south lanarkshire" => Ok(Self::SouthLanarkshire),
                    "south tyneside" => Ok(Self::SouthTyneside),
                    "southampton" => Ok(Self::Southampton),
                    "southend on sea" => Ok(Self::SouthendOnSea),
                    "southwark" => Ok(Self::Southwark),
                    "st helens" => Ok(Self::StHelens),
                    "staffordshire" => Ok(Self::Staffordshire),
                    "stirling" => Ok(Self::Stirling),
                    "stockport" => Ok(Self::Stockport),
                    "stockton on tees" => Ok(Self::StocktonOnTees),
                    "stoke on trent" => Ok(Self::StokeOnTrent),
                    "suffolk" => Ok(Self::Suffolk),
                    "sunderland" => Ok(Self::Sunderland),
                    "surrey" => Ok(Self::Surrey),
                    "sutton" => Ok(Self::Sutton),
                    "swansea" => Ok(Self::Swansea),
                    "swindon" => Ok(Self::Swindon),
                    "tameside" => Ok(Self::Tameside),
                    "telford and wrekin" => Ok(Self::TelfordAndWrekin),
                    "thurrock" => Ok(Self::Thurrock),
                    "torbay" => Ok(Self::Torbay),
                    "torfaen" => Ok(Self::Torfaen),
                    "tower hamlets" => Ok(Self::TowerHamlets),
                    "trafford" => Ok(Self::Trafford),
                    "vale of glamorgan" => Ok(Self::ValeOfGlamorgan),
                    "wakefield" => Ok(Self::Wakefield),
                    "walsall" => Ok(Self::Walsall),
                    "waltham forest" => Ok(Self::WalthamForest),
                    "wandsworth" => Ok(Self::Wandsworth),
                    "warrington" => Ok(Self::Warrington),
                    "warwickshire" => Ok(Self::Warwickshire),
                    "west berkshire" => Ok(Self::WestBerkshire),
                    "west dunbartonshire" => Ok(Self::WestDunbartonshire),
                    "west lothian" => Ok(Self::WestLothian),
                    "west sussex" => Ok(Self::WestSussex),
                    "westminster" => Ok(Self::Westminster),
                    "wigan" => Ok(Self::Wigan),
                    "wiltshire" => Ok(Self::Wiltshire),
                    "windsor and maidenhead" => Ok(Self::WindsorAndMaidenhead),
                    "wirral" => Ok(Self::Wirral),
                    "wokingham" => Ok(Self::Wokingham),
                    "wolverhampton" => Ok(Self::Wolverhampton),
                    "worcestershire" => Ok(Self::Worcestershire),
                    "wrexham" => Ok(Self::Wrexham),
                    "york" => Ok(Self::York),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for BelgiumStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "BelgiumStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "antwerp" => Ok(Self::Antwerp),
                    "brussels capital region" => Ok(Self::BrusselsCapitalRegion),
                    "east flanders" => Ok(Self::EastFlanders),
                    "flanders" => Ok(Self::Flanders),
                    "flemish brabant" => Ok(Self::FlemishBrabant),
                    "hainaut" => Ok(Self::Hainaut),
                    "limburg" => Ok(Self::Limburg),
                    "liege" => Ok(Self::Liege),
                    "luxembourg" => Ok(Self::Luxembourg),
                    "namur" => Ok(Self::Namur),
                    "wallonia" => Ok(Self::Wallonia),
                    "walloon brabant" => Ok(Self::WalloonBrabant),
                    "west flanders" => Ok(Self::WestFlanders),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for LuxembourgStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "LuxembourgStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "canton of capellen" => Ok(Self::CantonOfCapellen),
                    "canton of clervaux" => Ok(Self::CantonOfClervaux),
                    "canton of diekirch" => Ok(Self::CantonOfDiekirch),
                    "canton of echternach" => Ok(Self::CantonOfEchternach),
                    "canton of esch sur alzette" => Ok(Self::CantonOfEschSurAlzette),
                    "canton of grevenmacher" => Ok(Self::CantonOfGrevenmacher),
                    "canton of luxembourg" => Ok(Self::CantonOfLuxembourg),
                    "canton of mersch" => Ok(Self::CantonOfMersch),
                    "canton of redange" => Ok(Self::CantonOfRedange),
                    "canton of remich" => Ok(Self::CantonOfRemich),
                    "canton of vianden" => Ok(Self::CantonOfVianden),
                    "canton of wiltz" => Ok(Self::CantonOfWiltz),
                    "diekirch district" => Ok(Self::DiekirchDistrict),
                    "grevenmacher district" => Ok(Self::GrevenmacherDistrict),
                    "luxembourg district" => Ok(Self::LuxembourgDistrict),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for RussiaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "RussiaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "altai krai" => Ok(Self::AltaiKrai),
                    "altai republic" => Ok(Self::AltaiRepublic),
                    "amur oblast" => Ok(Self::AmurOblast),
                    "arkhangelsk" => Ok(Self::Arkhangelsk),
                    "astrakhan oblast" => Ok(Self::AstrakhanOblast),
                    "belgorod oblast" => Ok(Self::BelgorodOblast),
                    "bryansk oblast" => Ok(Self::BryanskOblast),
                    "chechen republic" => Ok(Self::ChechenRepublic),
                    "chelyabinsk oblast" => Ok(Self::ChelyabinskOblast),
                    "chukotka autonomous okrug" => Ok(Self::ChukotkaAutonomousOkrug),
                    "chuvash republic" => Ok(Self::ChuvashRepublic),
                    "irkutsk" => Ok(Self::Irkutsk),
                    "ivanovo oblast" => Ok(Self::IvanovoOblast),
                    "jewish autonomous oblast" => Ok(Self::JewishAutonomousOblast),
                    "kabardino-balkar republic" => Ok(Self::KabardinoBalkarRepublic),
                    "kaliningrad" => Ok(Self::Kaliningrad),
                    "kaluga oblast" => Ok(Self::KalugaOblast),
                    "kamchatka krai" => Ok(Self::KamchatkaKrai),
                    "karachay-cherkess republic" => Ok(Self::KarachayCherkessRepublic),
                    "kemerovo oblast" => Ok(Self::KemerovoOblast),
                    "khabarovsk krai" => Ok(Self::KhabarovskKrai),
                    "khanty-mansi autonomous okrug" => Ok(Self::KhantyMansiAutonomousOkrug),
                    "kirov oblast" => Ok(Self::KirovOblast),
                    "komi republic" => Ok(Self::KomiRepublic),
                    "kostroma oblast" => Ok(Self::KostromaOblast),
                    "krasnodar krai" => Ok(Self::KrasnodarKrai),
                    "krasnoyarsk krai" => Ok(Self::KrasnoyarskKrai),
                    "kurgan oblast" => Ok(Self::KurganOblast),
                    "kursk oblast" => Ok(Self::KurskOblast),
                    "leningrad oblast" => Ok(Self::LeningradOblast),
                    "lipetsk oblast" => Ok(Self::LipetskOblast),
                    "magadan oblast" => Ok(Self::MagadanOblast),
                    "mari el republic" => Ok(Self::MariElRepublic),
                    "moscow" => Ok(Self::Moscow),
                    "moscow oblast" => Ok(Self::MoscowOblast),
                    "murmansk oblast" => Ok(Self::MurmanskOblast),
                    "nenets autonomous okrug" => Ok(Self::NenetsAutonomousOkrug),
                    "nizhny novgorod oblast" => Ok(Self::NizhnyNovgorodOblast),
                    "novgorod oblast" => Ok(Self::NovgorodOblast),
                    "novosibirsk" => Ok(Self::Novosibirsk),
                    "omsk oblast" => Ok(Self::OmskOblast),
                    "orenburg oblast" => Ok(Self::OrenburgOblast),
                    "oryol oblast" => Ok(Self::OryolOblast),
                    "penza oblast" => Ok(Self::PenzaOblast),
                    "perm krai" => Ok(Self::PermKrai),
                    "primorsky krai" => Ok(Self::PrimorskyKrai),
                    "pskov oblast" => Ok(Self::PskovOblast),
                    "republic of adygea" => Ok(Self::RepublicOfAdygea),
                    "republic of bashkortostan" => Ok(Self::RepublicOfBashkortostan),
                    "republic of buryatia" => Ok(Self::RepublicOfBuryatia),
                    "republic of dagestan" => Ok(Self::RepublicOfDagestan),
                    "republic of ingushetia" => Ok(Self::RepublicOfIngushetia),
                    "republic of kalmykia" => Ok(Self::RepublicOfKalmykia),
                    "republic of karelia" => Ok(Self::RepublicOfKarelia),
                    "republic of khakassia" => Ok(Self::RepublicOfKhakassia),
                    "republic of mordovia" => Ok(Self::RepublicOfMordovia),
                    "republic of north ossetia-alania" => Ok(Self::RepublicOfNorthOssetiaAlania),
                    "republic of tatarstan" => Ok(Self::RepublicOfTatarstan),
                    "rostov oblast" => Ok(Self::RostovOblast),
                    "ryazan oblast" => Ok(Self::RyazanOblast),
                    "saint petersburg" => Ok(Self::SaintPetersburg),
                    "sakha republic" => Ok(Self::SakhaRepublic),
                    "sakhalin" => Ok(Self::Sakhalin),
                    "samara oblast" => Ok(Self::SamaraOblast),
                    "saratov oblast" => Ok(Self::SaratovOblast),
                    "sevastopol" => Ok(Self::Sevastopol),
                    "smolensk oblast" => Ok(Self::SmolenskOblast),
                    "stavropol krai" => Ok(Self::StavropolKrai),
                    "sverdlovsk" => Ok(Self::Sverdlovsk),
                    "tambov oblast" => Ok(Self::TambovOblast),
                    "tomsk oblast" => Ok(Self::TomskOblast),
                    "tula oblast" => Ok(Self::TulaOblast),
                    "tuva republic" => Ok(Self::TuvaRepublic),
                    "tver oblast" => Ok(Self::TverOblast),
                    "tyumen oblast" => Ok(Self::TyumenOblast),
                    "udmurt republic" => Ok(Self::UdmurtRepublic),
                    "ulyanovsk oblast" => Ok(Self::UlyanovskOblast),
                    "vladimir oblast" => Ok(Self::VladimirOblast),
                    "vologda oblast" => Ok(Self::VologdaOblast),
                    "voronezh oblast" => Ok(Self::VoronezhOblast),
                    "yamalo-nenets autonomous okrug" => Ok(Self::YamaloNenetsAutonomousOkrug),
                    "yaroslavl oblast" => Ok(Self::YaroslavlOblast),
                    "zabaykalsky krai" => Ok(Self::ZabaykalskyKrai),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for SanMarinoStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "SanMarinoStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "acquaviva" => Ok(Self::Acquaviva),
                    "borgo maggiore" => Ok(Self::BorgoMaggiore),
                    "chiesanuova" => Ok(Self::Chiesanuova),
                    "domagnano" => Ok(Self::Domagnano),
                    "faetano" => Ok(Self::Faetano),
                    "fiorentino" => Ok(Self::Fiorentino),
                    "montegiardino" => Ok(Self::Montegiardino),
                    "san marino" => Ok(Self::SanMarino),
                    "serravalle" => Ok(Self::Serravalle),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for SerbiaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "SerbiaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "belgrade" => Ok(Self::Belgrade),
                    "bor district" => Ok(Self::BorDistrict),
                    "braničevo district" => Ok(Self::BraničevoDistrict),
                    "central banat district" => Ok(Self::CentralBanatDistrict),
                    "jablanica district" => Ok(Self::JablanicaDistrict),
                    "kolubara district" => Ok(Self::KolubaraDistrict),
                    "mačva district" => Ok(Self::MačvaDistrict),
                    "moravica district" => Ok(Self::MoravicaDistrict),
                    "nišava district" => Ok(Self::NišavaDistrict),
                    "north banat district" => Ok(Self::NorthBanatDistrict),
                    "north bačka district" => Ok(Self::NorthBačkaDistrict),
                    "pirot district" => Ok(Self::PirotDistrict),
                    "podunavlje district" => Ok(Self::PodunavljeDistrict),
                    "pomoravlje district" => Ok(Self::PomoravljeDistrict),
                    "pčinja district" => Ok(Self::PčinjaDistrict),
                    "rasina district" => Ok(Self::RasinaDistrict),
                    "raška district" => Ok(Self::RaškaDistrict),
                    "south banat district" => Ok(Self::SouthBanatDistrict),
                    "south bačka district" => Ok(Self::SouthBačkaDistrict),
                    "srem district" => Ok(Self::SremDistrict),
                    "toplica district" => Ok(Self::ToplicaDistrict),
                    "vojvodina" => Ok(Self::Vojvodina),
                    "west bačka district" => Ok(Self::WestBačkaDistrict),
                    "zaječar district" => Ok(Self::ZaječarDistrict),
                    "zlatibor district" => Ok(Self::ZlatiborDistrict),
                    "šumadija district" => Ok(Self::ŠumadijaDistrict),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for SlovakiaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "SlovakiaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "banska bystrica region" => Ok(Self::BanskaBystricaRegion),
                    "bratislava region" => Ok(Self::BratislavaRegion),
                    "kosice region" => Ok(Self::KosiceRegion),
                    "nitra region" => Ok(Self::NitraRegion),
                    "presov region" => Ok(Self::PresovRegion),
                    "trencin region" => Ok(Self::TrencinRegion),
                    "trnava region" => Ok(Self::TrnavaRegion),
                    "zilina region" => Ok(Self::ZilinaRegion),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for SwedenStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "SwedenStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "blekinge" => Ok(Self::Blekinge),
                    "dalarna county" => Ok(Self::DalarnaCounty),
                    "gotland county" => Ok(Self::GotlandCounty),
                    "gävleborg county" => Ok(Self::GävleborgCounty),
                    "halland county" => Ok(Self::HallandCounty),
                    "jönköping county" => Ok(Self::JönköpingCounty),
                    "kalmar county" => Ok(Self::KalmarCounty),
                    "kronoberg county" => Ok(Self::KronobergCounty),
                    "norrbotten county" => Ok(Self::NorrbottenCounty),
                    "skåne county" => Ok(Self::SkåneCounty),
                    "stockholm county" => Ok(Self::StockholmCounty),
                    "södermanland county" => Ok(Self::SödermanlandCounty),
                    "uppsala county" => Ok(Self::UppsalaCounty),
                    "värmland county" => Ok(Self::VärmlandCounty),
                    "västerbotten county" => Ok(Self::VästerbottenCounty),
                    "västernorrland county" => Ok(Self::VästernorrlandCounty),
                    "västmanland county" => Ok(Self::VästmanlandCounty),
                    "västra götaland county" => Ok(Self::VästraGötalandCounty),
                    "örebro county" => Ok(Self::ÖrebroCounty),
                    "östergötland county" => Ok(Self::ÖstergötlandCounty),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for SloveniaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "SloveniaStatesAbbreviation",
        );
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "ajdovščina" => Ok(Self::Ajdovščina),
                    "ankaran" => Ok(Self::Ankaran),
                    "beltinci" => Ok(Self::Beltinci),
                    "benedikt" => Ok(Self::Benedikt),
                    "bistrica ob sotli" => Ok(Self::BistricaObSotli),
                    "bled" => Ok(Self::Bled),
                    "bloke" => Ok(Self::Bloke),
                    "bohinj" => Ok(Self::Bohinj),
                    "borovnica" => Ok(Self::Borovnica),
                    "bovec" => Ok(Self::Bovec),
                    "braslovče" => Ok(Self::Braslovče),
                    "brda" => Ok(Self::Brda),
                    "brezovica" => Ok(Self::Brezovica),
                    "brežice" => Ok(Self::Brežice),
                    "cankova" => Ok(Self::Cankova),
                    "cerklje na gorenjskem" => Ok(Self::CerkljeNaGorenjskem),
                    "cerknica" => Ok(Self::Cerknica),
                    "cerkno" => Ok(Self::Cerkno),
                    "cerkvenjak" => Ok(Self::Cerkvenjak),
                    "city municipality of celje" => Ok(Self::CityMunicipalityOfCelje),
                    "city municipality of novo mesto" => Ok(Self::CityMunicipalityOfNovoMesto),
                    "destrnik" => Ok(Self::Destrnik),
                    "divača" => Ok(Self::Divača),
                    "dobje" => Ok(Self::Dobje),
                    "dobrepolje" => Ok(Self::Dobrepolje),
                    "dobrna" => Ok(Self::Dobrna),
                    "dobrova-polhov gradec" => Ok(Self::DobrovaPolhovGradec),
                    "dobrovnik" => Ok(Self::Dobrovnik),
                    "dol pri ljubljani" => Ok(Self::DolPriLjubljani),
                    "dolenjske toplice" => Ok(Self::DolenjskeToplice),
                    "domžale" => Ok(Self::Domžale),
                    "dornava" => Ok(Self::Dornava),
                    "dravograd" => Ok(Self::Dravograd),
                    "duplek" => Ok(Self::Duplek),
                    "gorenja vas-poljane" => Ok(Self::GorenjaVasPoljane),
                    "gorišnica" => Ok(Self::Gorišnica),
                    "gorje" => Ok(Self::Gorje),
                    "gornja radgona" => Ok(Self::GornjaRadgona),
                    "gornji grad" => Ok(Self::GornjiGrad),
                    "gornji petrovci" => Ok(Self::GornjiPetrovci),
                    "grad" => Ok(Self::Grad),
                    "grosuplje" => Ok(Self::Grosuplje),
                    "hajdina" => Ok(Self::Hajdina),
                    "hodoš" => Ok(Self::Hodoš),
                    "horjul" => Ok(Self::Horjul),
                    "hoče-slivnica" => Ok(Self::HočeSlivnica),
                    "hrastnik" => Ok(Self::Hrastnik),
                    "hrpelje-kozina" => Ok(Self::HrpeljeKozina),
                    "idrija" => Ok(Self::Idrija),
                    "ig" => Ok(Self::Ig),
                    "ivančna gorica" => Ok(Self::IvančnaGorica),
                    "izola" => Ok(Self::Izola),
                    "jesenice" => Ok(Self::Jesenice),
                    "jezersko" => Ok(Self::Jezersko),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for UkraineStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check = StringExt::<Self>::parse_enum(
            value.to_uppercase().clone(),
            "UkraineStatesAbbreviation",
        );

        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();

                match state {
                    "autonomous republic of crimea" => Ok(Self::AutonomousRepublicOfCrimea),
                    "cherkasy oblast" => Ok(Self::CherkasyOblast),
                    "chernihiv oblast" => Ok(Self::ChernihivOblast),
                    "chernivtsi oblast" => Ok(Self::ChernivtsiOblast),
                    "dnipropetrovsk oblast" => Ok(Self::DnipropetrovskOblast),
                    "donetsk oblast" => Ok(Self::DonetskOblast),
                    "ivano-frankivsk oblast" => Ok(Self::IvanoFrankivskOblast),
                    "kharkiv oblast" => Ok(Self::KharkivOblast),
                    "kherson oblast" => Ok(Self::KhersonOblast),
                    "khmelnytsky oblast" => Ok(Self::KhmelnytskyOblast),
                    "kiev" => Ok(Self::Kiev),
                    "kirovohrad oblast" => Ok(Self::KirovohradOblast),
                    "kyiv oblast" => Ok(Self::KyivOblast),
                    "luhansk oblast" => Ok(Self::LuhanskOblast),
                    "lviv oblast" => Ok(Self::LvivOblast),
                    "mykolaiv oblast" => Ok(Self::MykolaivOblast),
                    "odessa oblast" => Ok(Self::OdessaOblast),
                    "rivne oblast" => Ok(Self::RivneOblast),
                    "sumy oblast" => Ok(Self::SumyOblast),
                    "ternopil oblast" => Ok(Self::TernopilOblast),
                    "vinnytsia oblast" => Ok(Self::VinnytsiaOblast),
                    "volyn oblast" => Ok(Self::VolynOblast),
                    "zakarpattia oblast" => Ok(Self::ZakarpattiaOblast),
                    "zaporizhzhya oblast" => Ok(Self::ZaporizhzhyaOblast),
                    "zhytomyr oblast" => Ok(Self::ZhytomyrOblast),
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
    AmazonPayRedirect,
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
    DirectCarrierBilling,
}

impl From<PaymentMethodData> for PaymentMethodDataType {
    fn from(pm_data: PaymentMethodData) -> Self {
        match pm_data {
            PaymentMethodData::Card(_) => Self::Card,
            PaymentMethodData::NetworkToken(_) => Self::NetworkToken,
            PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Self::NetworkTransactionIdAndCardDetails
            }
            PaymentMethodData::CardRedirect(card_redirect_data) => match card_redirect_data {
                payment_method_data::CardRedirectData::Knet {} => Self::Knet,
                payment_method_data::CardRedirectData::Benefit {} => Self::Benefit,
                payment_method_data::CardRedirectData::MomoAtm {} => Self::MomoAtm,
                payment_method_data::CardRedirectData::CardRedirect {} => Self::CardRedirect,
            },
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                payment_method_data::WalletData::AliPayQr(_) => Self::AliPayQr,
                payment_method_data::WalletData::AliPayRedirect(_) => Self::AliPayRedirect,
                payment_method_data::WalletData::AliPayHkRedirect(_) => Self::AliPayHkRedirect,
                payment_method_data::WalletData::AmazonPayRedirect(_) => Self::AmazonPayRedirect,
                payment_method_data::WalletData::MomoRedirect(_) => Self::MomoRedirect,
                payment_method_data::WalletData::KakaoPayRedirect(_) => Self::KakaoPayRedirect,
                payment_method_data::WalletData::GoPayRedirect(_) => Self::GoPayRedirect,
                payment_method_data::WalletData::GcashRedirect(_) => Self::GcashRedirect,
                payment_method_data::WalletData::ApplePay(_) => Self::ApplePay,
                payment_method_data::WalletData::ApplePayRedirect(_) => Self::ApplePayRedirect,
                payment_method_data::WalletData::ApplePayThirdPartySdk(_) => {
                    Self::ApplePayThirdPartySdk
                }
                payment_method_data::WalletData::DanaRedirect {} => Self::DanaRedirect,
                payment_method_data::WalletData::GooglePay(_) => Self::GooglePay,
                payment_method_data::WalletData::GooglePayRedirect(_) => Self::GooglePayRedirect,
                payment_method_data::WalletData::GooglePayThirdPartySdk(_) => {
                    Self::GooglePayThirdPartySdk
                }
                payment_method_data::WalletData::MbWayRedirect(_) => Self::MbWayRedirect,
                payment_method_data::WalletData::MobilePayRedirect(_) => Self::MobilePayRedirect,
                payment_method_data::WalletData::PaypalRedirect(_) => Self::PaypalRedirect,
                payment_method_data::WalletData::PaypalSdk(_) => Self::PaypalSdk,
                payment_method_data::WalletData::Paze(_) => Self::Paze,
                payment_method_data::WalletData::SamsungPay(_) => Self::SamsungPay,
                payment_method_data::WalletData::TwintRedirect {} => Self::TwintRedirect,
                payment_method_data::WalletData::VippsRedirect {} => Self::VippsRedirect,
                payment_method_data::WalletData::TouchNGoRedirect(_) => Self::TouchNGoRedirect,
                payment_method_data::WalletData::WeChatPayRedirect(_) => Self::WeChatPayRedirect,
                payment_method_data::WalletData::WeChatPayQr(_) => Self::WeChatPayQr,
                payment_method_data::WalletData::CashappQr(_) => Self::CashappQr,
                payment_method_data::WalletData::SwishQr(_) => Self::SwishQr,
                payment_method_data::WalletData::Mifinity(_) => Self::Mifinity,
            },
            PaymentMethodData::PayLater(pay_later_data) => match pay_later_data {
                payment_method_data::PayLaterData::KlarnaRedirect { .. } => Self::KlarnaRedirect,
                payment_method_data::PayLaterData::KlarnaSdk { .. } => Self::KlarnaSdk,
                payment_method_data::PayLaterData::AffirmRedirect {} => Self::AffirmRedirect,
                payment_method_data::PayLaterData::AfterpayClearpayRedirect { .. } => {
                    Self::AfterpayClearpayRedirect
                }
                payment_method_data::PayLaterData::PayBrightRedirect {} => Self::PayBrightRedirect,
                payment_method_data::PayLaterData::WalleyRedirect {} => Self::WalleyRedirect,
                payment_method_data::PayLaterData::AlmaRedirect {} => Self::AlmaRedirect,
                payment_method_data::PayLaterData::AtomeRedirect {} => Self::AtomeRedirect,
            },
            PaymentMethodData::BankRedirect(bank_redirect_data) => match bank_redirect_data {
                payment_method_data::BankRedirectData::BancontactCard { .. } => {
                    Self::BancontactCard
                }
                payment_method_data::BankRedirectData::Bizum {} => Self::Bizum,
                payment_method_data::BankRedirectData::Blik { .. } => Self::Blik,
                payment_method_data::BankRedirectData::Eps { .. } => Self::Eps,
                payment_method_data::BankRedirectData::Giropay { .. } => Self::Giropay,
                payment_method_data::BankRedirectData::Ideal { .. } => Self::Ideal,
                payment_method_data::BankRedirectData::Interac { .. } => Self::Interac,
                payment_method_data::BankRedirectData::OnlineBankingCzechRepublic { .. } => {
                    Self::OnlineBankingCzechRepublic
                }
                payment_method_data::BankRedirectData::OnlineBankingFinland { .. } => {
                    Self::OnlineBankingFinland
                }
                payment_method_data::BankRedirectData::OnlineBankingPoland { .. } => {
                    Self::OnlineBankingPoland
                }
                payment_method_data::BankRedirectData::OnlineBankingSlovakia { .. } => {
                    Self::OnlineBankingSlovakia
                }
                payment_method_data::BankRedirectData::OpenBankingUk { .. } => Self::OpenBankingUk,
                payment_method_data::BankRedirectData::Przelewy24 { .. } => Self::Przelewy24,
                payment_method_data::BankRedirectData::Sofort { .. } => Self::Sofort,
                payment_method_data::BankRedirectData::Trustly { .. } => Self::Trustly,
                payment_method_data::BankRedirectData::OnlineBankingFpx { .. } => {
                    Self::OnlineBankingFpx
                }
                payment_method_data::BankRedirectData::OnlineBankingThailand { .. } => {
                    Self::OnlineBankingThailand
                }
                payment_method_data::BankRedirectData::LocalBankRedirect {} => {
                    Self::LocalBankRedirect
                }
            },
            PaymentMethodData::BankDebit(bank_debit_data) => match bank_debit_data {
                payment_method_data::BankDebitData::AchBankDebit { .. } => Self::AchBankDebit,
                payment_method_data::BankDebitData::SepaBankDebit { .. } => Self::SepaBankDebit,
                payment_method_data::BankDebitData::BecsBankDebit { .. } => Self::BecsBankDebit,
                payment_method_data::BankDebitData::BacsBankDebit { .. } => Self::BacsBankDebit,
            },
            PaymentMethodData::BankTransfer(bank_transfer_data) => match *bank_transfer_data {
                payment_method_data::BankTransferData::AchBankTransfer { .. } => {
                    Self::AchBankTransfer
                }
                payment_method_data::BankTransferData::SepaBankTransfer { .. } => {
                    Self::SepaBankTransfer
                }
                payment_method_data::BankTransferData::BacsBankTransfer { .. } => {
                    Self::BacsBankTransfer
                }
                payment_method_data::BankTransferData::MultibancoBankTransfer { .. } => {
                    Self::MultibancoBankTransfer
                }
                payment_method_data::BankTransferData::PermataBankTransfer { .. } => {
                    Self::PermataBankTransfer
                }
                payment_method_data::BankTransferData::BcaBankTransfer { .. } => {
                    Self::BcaBankTransfer
                }
                payment_method_data::BankTransferData::BniVaBankTransfer { .. } => {
                    Self::BniVaBankTransfer
                }
                payment_method_data::BankTransferData::BriVaBankTransfer { .. } => {
                    Self::BriVaBankTransfer
                }
                payment_method_data::BankTransferData::CimbVaBankTransfer { .. } => {
                    Self::CimbVaBankTransfer
                }
                payment_method_data::BankTransferData::DanamonVaBankTransfer { .. } => {
                    Self::DanamonVaBankTransfer
                }
                payment_method_data::BankTransferData::MandiriVaBankTransfer { .. } => {
                    Self::MandiriVaBankTransfer
                }
                payment_method_data::BankTransferData::Pix { .. } => Self::Pix,
                payment_method_data::BankTransferData::Pse {} => Self::Pse,
                payment_method_data::BankTransferData::LocalBankTransfer { .. } => {
                    Self::LocalBankTransfer
                }
            },
            PaymentMethodData::Crypto(_) => Self::Crypto,
            PaymentMethodData::MandatePayment => Self::MandatePayment,
            PaymentMethodData::Reward => Self::Reward,
            PaymentMethodData::Upi(_) => Self::Upi,
            PaymentMethodData::Voucher(voucher_data) => match voucher_data {
                payment_method_data::VoucherData::Boleto(_) => Self::Boleto,
                payment_method_data::VoucherData::Efecty => Self::Efecty,
                payment_method_data::VoucherData::PagoEfectivo => Self::PagoEfectivo,
                payment_method_data::VoucherData::RedCompra => Self::RedCompra,
                payment_method_data::VoucherData::RedPagos => Self::RedPagos,
                payment_method_data::VoucherData::Alfamart(_) => Self::Alfamart,
                payment_method_data::VoucherData::Indomaret(_) => Self::Indomaret,
                payment_method_data::VoucherData::Oxxo => Self::Oxxo,
                payment_method_data::VoucherData::SevenEleven(_) => Self::SevenEleven,
                payment_method_data::VoucherData::Lawson(_) => Self::Lawson,
                payment_method_data::VoucherData::MiniStop(_) => Self::MiniStop,
                payment_method_data::VoucherData::FamilyMart(_) => Self::FamilyMart,
                payment_method_data::VoucherData::Seicomart(_) => Self::Seicomart,
                payment_method_data::VoucherData::PayEasy(_) => Self::PayEasy,
            },
            PaymentMethodData::RealTimePayment(real_time_payment_data) => {
                match *real_time_payment_data {
                    payment_method_data::RealTimePaymentData::DuitNow {} => Self::DuitNow,
                    payment_method_data::RealTimePaymentData::Fps {} => Self::Fps,
                    payment_method_data::RealTimePaymentData::PromptPay {} => Self::PromptPay,
                    payment_method_data::RealTimePaymentData::VietQr {} => Self::VietQr,
                }
            }
            PaymentMethodData::GiftCard(gift_card_data) => match *gift_card_data {
                payment_method_data::GiftCardData::Givex(_) => Self::Givex,
                payment_method_data::GiftCardData::PaySafeCard {} => Self::PaySafeCar,
            },
            PaymentMethodData::CardToken(_) => Self::CardToken,
            PaymentMethodData::OpenBanking(data) => match data {
                payment_method_data::OpenBankingData::OpenBankingPIS {} => Self::OpenBanking,
            },
            PaymentMethodData::MobilePayment(mobile_payment_data) => match mobile_payment_data {
                payment_method_data::MobilePaymentData::DirectCarrierBilling { .. } => {
                    Self::DirectCarrierBilling
                }
            },
        }
    }
}
pub trait ApplePay {
    fn get_applepay_decoded_payment_data(&self) -> Result<Secret<String>, Error>;
}

impl ApplePay for payment_method_data::ApplePayWalletData {
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

impl WalletData for payment_method_data::WalletData {
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
                let json_token: Value = self.get_wallet_token_as_json("Google Pay".to_owned())?;
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

pub fn deserialize_xml_to_struct<T: serde::de::DeserializeOwned>(
    xml_data: &[u8],
) -> Result<T, errors::ConnectorError> {
    let response_str = std::str::from_utf8(xml_data)
        .map_err(|e| {
            router_env::logger::error!("Error converting response data to UTF-8: {:?}", e);
            errors::ConnectorError::ResponseDeserializationFailed
        })?
        .trim();
    let result: T = quick_xml::de::from_str(response_str).map_err(|e| {
        router_env::logger::error!("Error deserializing XML response: {:?}", e);
        errors::ConnectorError::ResponseDeserializationFailed
    })?;

    Ok(result)
}

pub fn is_html_response(response: &str) -> bool {
    response.starts_with("<html>") || response.starts_with("<!DOCTYPE html>")
}

#[cfg(feature = "payouts")]
pub trait PayoutsData {
    fn get_transfer_id(&self) -> Result<String, Error>;
    fn get_customer_details(
        &self,
    ) -> Result<hyperswitch_domain_models::router_request_types::CustomerDetails, Error>;
    fn get_vendor_details(&self) -> Result<PayoutVendorAccountDetails, Error>;
    #[cfg(feature = "payouts")]
    fn get_payout_type(&self) -> Result<enums::PayoutType, Error>;
}

#[cfg(feature = "payouts")]
impl PayoutsData for hyperswitch_domain_models::router_request_types::PayoutsData {
    fn get_transfer_id(&self) -> Result<String, Error> {
        self.connector_payout_id
            .clone()
            .ok_or_else(missing_field_err("transfer_id"))
    }
    fn get_customer_details(
        &self,
    ) -> Result<hyperswitch_domain_models::router_request_types::CustomerDetails, Error> {
        self.customer_details
            .clone()
            .ok_or_else(missing_field_err("customer_details"))
    }
    fn get_vendor_details(&self) -> Result<PayoutVendorAccountDetails, Error> {
        self.vendor_details
            .clone()
            .ok_or_else(missing_field_err("vendor_details"))
    }
    #[cfg(feature = "payouts")]
    fn get_payout_type(&self) -> Result<enums::PayoutType, Error> {
        self.payout_type
            .to_owned()
            .ok_or_else(missing_field_err("payout_type"))
    }
}
pub trait RevokeMandateRequestData {
    fn get_connector_mandate_id(&self) -> Result<String, Error>;
}

impl RevokeMandateRequestData for MandateRevokeRequestData {
    fn get_connector_mandate_id(&self) -> Result<String, Error> {
        self.connector_mandate_id
            .clone()
            .ok_or_else(missing_field_err("connector_mandate_id"))
    }
}
pub trait RecurringMandateData {
    fn get_original_payment_amount(&self) -> Result<i64, Error>;
    fn get_original_payment_currency(&self) -> Result<enums::Currency, Error>;
}

impl RecurringMandateData for RecurringMandatePaymentData {
    fn get_original_payment_amount(&self) -> Result<i64, Error> {
        self.original_payment_authorized_amount
            .ok_or_else(missing_field_err("original_payment_authorized_amount"))
    }
    fn get_original_payment_currency(&self) -> Result<enums::Currency, Error> {
        self.original_payment_authorized_currency
            .ok_or_else(missing_field_err("original_payment_authorized_currency"))
    }
}

#[cfg(feature = "payouts")]
impl CardData for api_models::payouts::CardPayout {
    fn get_card_expiry_year_2_digit(&self) -> Result<Secret<String>, errors::ConnectorError> {
        let binding = self.expiry_year.clone();
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
            self.expiry_month.peek(),
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
            self.expiry_month.peek()
        ))
    }
    fn get_expiry_date_as_mmyyyy(&self, delimiter: &str) -> Secret<String> {
        let year = self.get_expiry_year_4_digit();
        Secret::new(format!(
            "{}{}{}",
            self.expiry_month.peek(),
            delimiter,
            year.peek()
        ))
    }
    fn get_expiry_year_4_digit(&self) -> Secret<String> {
        let mut year = self.expiry_year.peek().clone();
        if year.len() == 2 {
            year = format!("20{}", year);
        }
        Secret::new(year)
    }
    fn get_expiry_date_as_yymm(&self) -> Result<Secret<String>, errors::ConnectorError> {
        let year = self.get_card_expiry_year_2_digit()?.expose();
        let month = self.expiry_month.clone().expose();
        Ok(Secret::new(format!("{year}{month}")))
    }
    fn get_expiry_month_as_i8(&self) -> Result<Secret<i8>, Error> {
        self.expiry_month
            .peek()
            .clone()
            .parse::<i8>()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            .map(Secret::new)
    }
    fn get_expiry_year_as_i32(&self) -> Result<Secret<i32>, Error> {
        self.expiry_year
            .peek()
            .clone()
            .parse::<i32>()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            .map(Secret::new)
    }

    fn get_expiry_date_as_mmyy(&self) -> Result<Secret<String>, errors::ConnectorError> {
        let year = self.get_card_expiry_year_2_digit()?.expose();
        let month = self.expiry_month.clone().expose();
        Ok(Secret::new(format!("{month}{year}")))
    }

    fn get_expiry_year_as_4_digit_i32(&self) -> Result<Secret<i32>, Error> {
        self.get_expiry_year_4_digit()
            .peek()
            .clone()
            .parse::<i32>()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            .map(Secret::new)
    }
    fn get_cardholder_name(&self) -> Result<Secret<String>, Error> {
        self.card_holder_name
            .clone()
            .ok_or_else(missing_field_err("card.card_holder_name"))
    }
}

pub trait NetworkTokenData {
    fn get_card_issuer(&self) -> Result<CardIssuer, Error>;
    fn get_expiry_year_4_digit(&self) -> Secret<String>;
    fn get_network_token(&self) -> NetworkTokenNumber;
    fn get_network_token_expiry_month(&self) -> Secret<String>;
    fn get_network_token_expiry_year(&self) -> Secret<String>;
    fn get_cryptogram(&self) -> Option<Secret<String>>;
}

impl NetworkTokenData for payment_method_data::NetworkTokenData {
    #[cfg(feature = "v1")]
    fn get_card_issuer(&self) -> Result<CardIssuer, Error> {
        get_card_issuer(self.token_number.peek())
    }

    #[cfg(feature = "v2")]
    fn get_card_issuer(&self) -> Result<CardIssuer, Error> {
        get_card_issuer(self.network_token.peek())
    }

    #[cfg(feature = "v1")]
    fn get_expiry_year_4_digit(&self) -> Secret<String> {
        let mut year = self.token_exp_year.peek().clone();
        if year.len() == 2 {
            year = format!("20{}", year);
        }
        Secret::new(year)
    }

    #[cfg(feature = "v2")]
    fn get_expiry_year_4_digit(&self) -> Secret<String> {
        let mut year = self.network_token_exp_year.peek().clone();
        if year.len() == 2 {
            year = format!("20{}", year);
        }
        Secret::new(year)
    }

    #[cfg(feature = "v1")]
    fn get_network_token(&self) -> NetworkTokenNumber {
        self.token_number.clone()
    }

    #[cfg(feature = "v2")]
    fn get_network_token(&self) -> NetworkTokenNumber {
        self.network_token.clone()
    }

    #[cfg(feature = "v1")]
    fn get_network_token_expiry_month(&self) -> Secret<String> {
        self.token_exp_month.clone()
    }

    #[cfg(feature = "v2")]
    fn get_network_token_expiry_month(&self) -> Secret<String> {
        self.network_token_exp_month.clone()
    }

    #[cfg(feature = "v1")]
    fn get_network_token_expiry_year(&self) -> Secret<String> {
        self.token_exp_year.clone()
    }

    #[cfg(feature = "v2")]
    fn get_network_token_expiry_year(&self) -> Secret<String> {
        self.network_token_exp_year.clone()
    }

    #[cfg(feature = "v1")]
    fn get_cryptogram(&self) -> Option<Secret<String>> {
        self.token_cryptogram.clone()
    }

    #[cfg(feature = "v2")]
    fn get_cryptogram(&self) -> Option<Secret<String>> {
        self.cryptogram.clone()
    }
}
