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
            StringExt::<Self>::parse_enum(value.clone(), "PolandStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Greater Poland" => Ok(Self::GreaterPoland),
                "Holy Cross" => Ok(Self::HolyCross),
                "Kuyavia-Pomerania" => Ok(Self::KuyaviaPomerania),
                "Lesser Poland" => Ok(Self::LesserPoland),
                "Lower Silesia" => Ok(Self::LowerSilesia),
                "Lublin" => Ok(Self::Lublin),
                "Lubusz" => Ok(Self::Lubusz),
                "Łódź" => Ok(Self::Łódź),
                "Mazovia" => Ok(Self::Mazovia),
                "Podlaskie" => Ok(Self::Podlaskie),
                "Pomerania" => Ok(Self::Pomerania),
                "Silesia" => Ok(Self::Silesia),
                "Subcarpathia" => Ok(Self::Subcarpathia),
                "Upper Silesia" => Ok(Self::UpperSilesia),
                "Warmia-Masuria" => Ok(Self::WarmiaMasuria),
                "West Pomerania" => Ok(Self::WestPomerania),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for FranceStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "FranceStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Ain" => Ok(Self::Ain),
                "Aisne" => Ok(Self::Aisne),
                "Allier" => Ok(Self::Allier),
                "Alpes-de-Haute-Provence" => Ok(Self::AlpesDeHauteProvence),
                "Alpes-Maritimes" => Ok(Self::AlpesMaritimes),
                "Alsace" => Ok(Self::Alsace),
                "Ardèche" => Ok(Self::Ardeche),
                "Ardennes" => Ok(Self::Ardennes),
                "Ariège" => Ok(Self::Ariege),
                "Aube" => Ok(Self::Aube),
                "Aude" => Ok(Self::Aude),
                "Auvergne-Rhône-Alpes" => Ok(Self::AuvergneRhoneAlpes),
                "Aveyron" => Ok(Self::Aveyron),
                "Bas-Rhin" => Ok(Self::BasRhin),
                "Bouches-du-Rhône" => Ok(Self::BouchesDuRhone),
                "Bourgogne-Franche-Comté" => Ok(Self::BourgogneFrancheComte),
                "Bretagne" => Ok(Self::Bretagne),
                "Calvados" => Ok(Self::Calvados),
                "Cantal" => Ok(Self::Cantal),
                "Centre-Val de Loire" => Ok(Self::CentreValDeLoire),
                "Charente" => Ok(Self::Charente),
                "Charente-Maritime" => Ok(Self::CharenteMaritime),
                "Cher" => Ok(Self::Cher),
                "Clipperton" => Ok(Self::Clipperton),
                "Corrèze" => Ok(Self::Correze),
                "Corse" => Ok(Self::Corse),
                "Corse-du-Sud" => Ok(Self::CorseDuSud),
                "Côte-d'Or" => Ok(Self::CoteDor),
                "Côtes-d'Armor" => Ok(Self::CotesDarmor),
                "Creuse" => Ok(Self::Creuse),
                "Deux-Sèvres" => Ok(Self::DeuxSevres),
                "Dordogne" => Ok(Self::Dordogne),
                "Doubs" => Ok(Self::Doubs),
                "Drôme" => Ok(Self::Drome),
                "Essonne" => Ok(Self::Essonne),
                "Eure" => Ok(Self::Eure),
                "Eure-et-Loir" => Ok(Self::EureEtLoir),
                "Finistère" => Ok(Self::Finistere),
                "French Guiana" => Ok(Self::FrenchGuiana),
                "French Polynesia" => Ok(Self::FrenchPolynesia),
                "French Southern and Antarctic Lands" => Ok(Self::FrenchSouthernAndAntarcticLands),
                "Gard" => Ok(Self::Gard),
                "Gers" => Ok(Self::Gers),
                "Gironde" => Ok(Self::Gironde),
                "Grand-Est" => Ok(Self::GrandEst),
                "Guadeloupe" => Ok(Self::Guadeloupe),
                "Haut-Rhin" => Ok(Self::HautRhin),
                "Haute-Corse" => Ok(Self::HauteCorse),
                "Haute-Garonne" => Ok(Self::HauteGaronne),
                "Haute-Loire" => Ok(Self::HauteLoire),
                "Haute-Marne" => Ok(Self::HauteMarne),
                "Haute-Saône" => Ok(Self::HauteSaone),
                "Haute-Savoie" => Ok(Self::HauteSavoie),
                "Haute-Vienne" => Ok(Self::HauteVienne),
                "Hautes-Alpes" => Ok(Self::HautesAlpes),
                "Hautes-Pyrénées" => Ok(Self::HautesPyrenees),
                "Hauts-de-France" => Ok(Self::HautsDeFrance),
                "Hauts-de-Seine" => Ok(Self::HautsDeSeine),
                "Hérault" => Ok(Self::Herault),
                "Île-de-France" => Ok(Self::IleDeFrance),
                "Ille-et-Vilaine" => Ok(Self::IlleEtVilaine),
                "Indre" => Ok(Self::Indre),
                "Indre-et-Loire" => Ok(Self::IndreEtLoire),
                "Isère" => Ok(Self::Isere),
                "Jura" => Ok(Self::Jura),
                "La Réunion" => Ok(Self::LaReunion),
                "Landes" => Ok(Self::Landes),
                "Loir-et-Cher" => Ok(Self::LoirEtCher),
                "Loire" => Ok(Self::Loire),
                "Loire-Atlantique" => Ok(Self::LoireAtlantique),
                "Loiret" => Ok(Self::Loiret),
                "Lot" => Ok(Self::Lot),
                "Lot-et-Garonne" => Ok(Self::LotEtGaronne),
                "Lozère" => Ok(Self::Lozere),
                "Maine-et-Loire" => Ok(Self::MaineEtLoire),
                "Manche" => Ok(Self::Manche),
                "Marne" => Ok(Self::Marne),
                "Martinique" => Ok(Self::Martinique),
                "Mayenne" => Ok(Self::Mayenne),
                "Mayotte" => Ok(Self::Mayotte),
                "Métropole de Lyon" => Ok(Self::MetropoleDeLyon),
                "Meurthe-et-Moselle" => Ok(Self::MeurtheEtMoselle),
                "Meuse" => Ok(Self::Meuse),
                "Morbihan" => Ok(Self::Morbihan),
                "Moselle" => Ok(Self::Moselle),
                "Nièvre" => Ok(Self::Nievre),
                "Nord" => Ok(Self::Nord),
                "Normandie" => Ok(Self::Normandie),
                "Nouvelle-Aquitaine" => Ok(Self::NouvelleAquitaine),
                "Occitanie" => Ok(Self::Occitanie),
                "Oise" => Ok(Self::Oise),
                "Orne" => Ok(Self::Orne),
                "Paris" => Ok(Self::Paris),
                "Pas-de-Calais" => Ok(Self::PasDeCalais),
                "Pays-de-la-Loire" => Ok(Self::PaysDeLaLoire),
                "Provence-Alpes-Côte-d'Azur" => Ok(Self::ProvenceAlpesCoteDazur),
                "Puy-de-Dôme" => Ok(Self::PuyDeDome),
                "Pyrénées-Atlantiques" => Ok(Self::PyreneesAtlantiques),
                "Pyrénées-Orientales" => Ok(Self::PyreneesOrientales),
                "Rhône" => Ok(Self::Rhone),
                "Saint Pierre and Miquelon" => Ok(Self::SaintPierreAndMiquelon),
                "Saint-Barthélemy" => Ok(Self::SaintBarthelemy),
                "Saint-Martin" => Ok(Self::SaintMartin),
                "Saône-et-Loire" => Ok(Self::SaoneEtLoire),
                "Sarthe" => Ok(Self::Sarthe),
                "Savoie" => Ok(Self::Savoie),
                "Seine-et-Marne" => Ok(Self::SeineEtMarne),
                "Seine-Maritime" => Ok(Self::SeineMaritime),
                "Seine-Saint-Denis" => Ok(Self::SeineSaintDenis),
                "Somme" => Ok(Self::Somme),
                "Tarn" => Ok(Self::Tarn),
                "Tarn-et-Garonne" => Ok(Self::TarnEtGaronne),
                "Territoire de Belfort" => Ok(Self::TerritoireDeBelfort),
                "Val-d'Oise" => Ok(Self::ValDoise),
                "Val-de-Marne" => Ok(Self::ValDeMarne),
                "Var" => Ok(Self::Var),
                "Vaucluse" => Ok(Self::Vaucluse),
                "Vendée" => Ok(Self::Vendee),
                "Vienne" => Ok(Self::Vienne),
                "Vosges" => Ok(Self::Vosges),
                "Wallis and Futuna" => Ok(Self::WallisAndFutuna),
                "Yonne" => Ok(Self::Yonne),
                "Yvelines" => Ok(Self::Yvelines),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for GermanyStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "GermanyStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Baden-Württemberg" => Ok(Self::BW),
                "Bavaria" => Ok(Self::BY),
                "Berlin" => Ok(Self::BE),
                "Brandenburg" => Ok(Self::BB),
                "Bremen" => Ok(Self::HB),
                "Hamburg" => Ok(Self::HH),
                "Hessen" => Ok(Self::HE),
                "Lower Saxony" => Ok(Self::NI),
                "Mecklenburg-Vorpommern" => Ok(Self::MV),
                "North Rhine-Westphalia" => Ok(Self::NW),
                "Rhineland-Palatinate" => Ok(Self::RP),
                "Saarland" => Ok(Self::SL),
                "Saxony" => Ok(Self::SN),
                "Saxony-Anhalt" => Ok(Self::ST),
                "Schleswig-Holstein" => Ok(Self::SH),
                "Thuringia" => Ok(Self::TH),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for SpainStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "SpainStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "A Coruña Province" => Ok(Self::ACorunaProvince),
                "Albacete Province" => Ok(Self::AlbaceteProvince),
                "Alicante Province" => Ok(Self::AlicanteProvince),
                "Almería Province" => Ok(Self::AlmeriaProvince),
                "Andalusia" => Ok(Self::Andalusia),
                "Araba / Álava" => Ok(Self::ArabaAlava),
                "Aragon" => Ok(Self::Aragon),
                "Badajoz Province" => Ok(Self::BadajozProvince),
                "Balearic Islands" => Ok(Self::BalearicIslands),
                "Barcelona Province" => Ok(Self::BarcelonaProvince),
                "Basque Country" => Ok(Self::BasqueCountry),
                "Biscay" => Ok(Self::Biscay),
                "Burgos Province" => Ok(Self::BurgosProvince),
                "Canary Islands" => Ok(Self::CanaryIslands),
                "Cantabria" => Ok(Self::Cantabria),
                "Castellón Province" => Ok(Self::CastellonProvince),
                "Castile and León" => Ok(Self::CastileAndLeon),
                "Castilla-La Mancha" => Ok(Self::CastileLaMancha),
                "Catalonia" => Ok(Self::Catalonia),
                "Ceuta" => Ok(Self::Ceuta),
                "Ciudad Real Province" => Ok(Self::CiudadRealProvince),
                "Community of Madrid" => Ok(Self::CommunityOfMadrid),
                "Cuenca Province" => Ok(Self::CuencaProvince),
                "Cáceres Province" => Ok(Self::CaceresProvince),
                "Cádiz Province" => Ok(Self::CadizProvince),
                "Córdoba Province" => Ok(Self::CordobaProvince),
                "Extremadura" => Ok(Self::Extremadura),
                "Galicia" => Ok(Self::Galicia),
                "Gipuzkoa" => Ok(Self::Gipuzkoa),
                "Girona Province" => Ok(Self::GironaProvince),
                "Granada Province" => Ok(Self::GranadaProvince),
                "Guadalajara Province" => Ok(Self::GuadalajaraProvince),
                "Huelva Province" => Ok(Self::HuelvaProvince),
                "Huesca Province" => Ok(Self::HuescaProvince),
                "Jaén Province" => Ok(Self::JaenProvince),
                "La Rioja" => Ok(Self::LaRioja),
                "Las Palmas Province" => Ok(Self::LasPalmasProvince),
                "León Province" => Ok(Self::LeonProvince),
                "Lleida Province" => Ok(Self::LleidaProvince),
                "Lugo Province" => Ok(Self::LugoProvince),
                "Madrid Province" => Ok(Self::MadridProvince),
                "Melilla" => Ok(Self::Melilla),
                "Murcia Province" => Ok(Self::MurciaProvince),
                "Málaga Province" => Ok(Self::MalagaProvince),
                "Navarre" => Ok(Self::Navarre),
                "Ourense Province" => Ok(Self::OurenseProvince),
                "Palencia Province" => Ok(Self::PalenciaProvince),
                "Pontevedra Province" => Ok(Self::PontevedraProvince),
                "Province of Asturias" => Ok(Self::ProvinceOfAsturias),
                "Province of Ávila" => Ok(Self::ProvinceOfAvila),
                "Region of Murcia" => Ok(Self::RegionOfMurcia),
                "Salamanca Province" => Ok(Self::SalamancaProvince),
                "Santa Cruz de Tenerife Province" => Ok(Self::SantaCruzDeTenerifeProvince),
                "Segovia Province" => Ok(Self::SegoviaProvince),
                "Seville Province" => Ok(Self::SevilleProvince),
                "Soria Province" => Ok(Self::SoriaProvince),
                "Tarragona Province" => Ok(Self::TarragonaProvince),
                "Teruel Province" => Ok(Self::TeruelProvince),
                "Toledo Province" => Ok(Self::ToledoProvince),
                "Valencia Province" => Ok(Self::ValenciaProvince),
                "Valencian Community" => Ok(Self::ValencianCommunity),
                "Valladolid Province" => Ok(Self::ValladolidProvince),
                "Zamora Province" => Ok(Self::ZamoraProvince),
                "Zaragoza Province" => Ok(Self::ZaragozaProvince),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for ItalyStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "ItalyStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Abruzzo" => Ok(Self::Abruzzo),
                "Aosta Valley" => Ok(Self::AostaValley),
                "Apulia" => Ok(Self::Apulia),
                "Basilicata" => Ok(Self::Basilicata),
                "Benevento Province" => Ok(Self::BeneventoProvince),
                "Calabria" => Ok(Self::Calabria),
                "Campania" => Ok(Self::Campania),
                "Emilia-Romagna" => Ok(Self::EmiliaRomagna),
                "Friuli–Venezia Giulia" => Ok(Self::FriuliVeneziaGiulia),
                "Lazio" => Ok(Self::Lazio),
                "Liguria" => Ok(Self::Liguria),
                "Lombardy" => Ok(Self::Lombardy),
                "Marche" => Ok(Self::Marche),
                "Molise" => Ok(Self::Molise),
                "Piedmont" => Ok(Self::Piedmont),
                "Sardinia" => Ok(Self::Sardinia),
                "Sicily" => Ok(Self::Sicily),
                "Trentino-South Tyrol" => Ok(Self::TrentinoSouthTyrol),
                "Tuscany" => Ok(Self::Tuscany),
                "Umbria" => Ok(Self::Umbria),
                "Veneto" => Ok(Self::Veneto),
                "Libero consorzio comunale di Agrigento" => Ok(Self::Agrigento),
                "Libero consorzio comunale di Caltanissetta" => Ok(Self::Caltanissetta),
                "Libero consorzio comunale di Enna" => Ok(Self::Enna),
                "Libero consorzio comunale di Ragusa" => Ok(Self::Ragusa),
                "Libero consorzio comunale di Siracusa" => Ok(Self::Siracusa),
                "Libero consorzio comunale di Trapani" => Ok(Self::Trapani),
                "Metropolitan City of Bari" => Ok(Self::Bari),
                "Metropolitan City of Bologna" => Ok(Self::Bologna),
                "Metropolitan City of Cagliari" => Ok(Self::Cagliari),
                "Metropolitan City of Catania" => Ok(Self::Catania),
                "Metropolitan City of Florence" => Ok(Self::Florence),
                "Metropolitan City of Genoa" => Ok(Self::Genoa),
                "Metropolitan City of Messina" => Ok(Self::Messina),
                "Metropolitan City of Milan" => Ok(Self::Milan),
                "Metropolitan City of Naples" => Ok(Self::Naples),
                "Metropolitan City of Palermo" => Ok(Self::Palermo),
                "Metropolitan City of Reggio Calabria" => Ok(Self::ReggioCalabria),
                "Metropolitan City of Rome" => Ok(Self::Rome),
                "Metropolitan City of Turin" => Ok(Self::Turin),
                "Metropolitan City of Venice" => Ok(Self::Venice),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for NorwayStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "NorwayStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Akershus" => Ok(Self::Akershus),
                "Buskerud" => Ok(Self::Buskerud),
                "Finnmark" => Ok(Self::Finnmark),
                "Hedmark" => Ok(Self::Hedmark),
                "Hordaland" => Ok(Self::Hordaland),
                "Jan Mayen" => Ok(Self::JanMayen),
                "Møre og Romsdal" => Ok(Self::MoreOgRomsdal),
                "Nord-Trøndelag" => Ok(Self::NordTrondelag),
                "Nordland" => Ok(Self::Nordland),
                "Oppland" => Ok(Self::Oppland),
                "Oslo" => Ok(Self::Oslo),
                "Rogaland" => Ok(Self::Rogaland),
                "Sogn og Fjordane" => Ok(Self::SognOgFjordane),
                "Svalbard" => Ok(Self::Svalbard),
                "Sør-Trøndelag" => Ok(Self::SorTrondelag),
                "Telemark" => Ok(Self::Telemark),
                "Troms" => Ok(Self::Troms),
                "Trøndelag" => Ok(Self::Trondelag),
                "Vest-Agder" => Ok(Self::VestAgder),
                "Vestfold" => Ok(Self::Vestfold),
                "Østfold" => Ok(Self::Ostfold),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for AlbaniaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "AlbaniaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Berat" => Ok(Self::Berat),
                "Dibër" => Ok(Self::Diber),
                "Durrës" => Ok(Self::Durres),
                "Elbasan" => Ok(Self::Elbasan),
                "Fier" => Ok(Self::Fier),
                "Gjirokastër" => Ok(Self::Gjirokaster),
                "Korçë" => Ok(Self::Korce),
                "Kukës" => Ok(Self::Kukes),
                "Lezhë" => Ok(Self::Lezhe),
                "Shkodër" => Ok(Self::Shkoder),
                "Tiranë" => Ok(Self::Tirane),
                "Vlorë" => Ok(Self::Vlore),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for AndorraStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "AndorraStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Andorra la Vella" => Ok(Self::AndorraLaVella),
                "Canillo" => Ok(Self::Canillo),
                "Encamp" => Ok(Self::Encamp),
                "Escaldes-Engordany" => Ok(Self::EscaldesEngordany),
                "La Massana" => Ok(Self::LaMassana),
                "Ordino" => Ok(Self::Ordino),
                "Sant Julià de Lòria" => Ok(Self::SantJuliaDeLoria),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for AustriaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "AustriaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Burgenland" => Ok(Self::Burgenland),
                "Carinthia" => Ok(Self::Carinthia),
                "Lower Austria" => Ok(Self::LowerAustria),
                "Salzburg" => Ok(Self::Salzburg),
                "Styria" => Ok(Self::Styria),
                "Tyrol" => Ok(Self::Tyrol),
                "Upper Austria" => Ok(Self::UpperAustria),
                "Vienna" => Ok(Self::Vienna),
                "Vorarlberg" => Ok(Self::Vorarlberg),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for RomaniaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "RomaniaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Alba" => Ok(Self::Alba),
                "Arad County" => Ok(Self::AradCounty),
                "Argeș" => Ok(Self::Arges),
                "Bacău County" => Ok(Self::BacauCounty),
                "Bihor County" => Ok(Self::BihorCounty),
                "Bistrița-Năsăud County" => Ok(Self::BistritaNasaudCounty),
                "Botoșani County" => Ok(Self::BotosaniCounty),
                "Brăila" => Ok(Self::Braila),
                "Brașov County" => Ok(Self::BrasovCounty),
                "Bucharest" => Ok(Self::Bucharest),
                "Buzău County" => Ok(Self::BuzauCounty),
                "Caraș-Severin County" => Ok(Self::CarasSeverinCounty),
                "Cluj County" => Ok(Self::ClujCounty),
                "Constanța County" => Ok(Self::ConstantaCounty),
                "Covasna County" => Ok(Self::CovasnaCounty),
                "Călărași County" => Ok(Self::CalarasiCounty),
                "Dolj County" => Ok(Self::DoljCounty),
                "Dâmbovița County" => Ok(Self::DambovitaCounty),
                "Galați County" => Ok(Self::GalatiCounty),
                "Giurgiu County" => Ok(Self::GiurgiuCounty),
                "Gorj County" => Ok(Self::GorjCounty),
                "Harghita County" => Ok(Self::HarghitaCounty),
                "Hunedoara County" => Ok(Self::HunedoaraCounty),
                "Ialomița County" => Ok(Self::IalomitaCounty),
                "Iași County" => Ok(Self::IasiCounty),
                "Ilfov County" => Ok(Self::IlfovCounty),
                "Mehedinți County" => Ok(Self::MehedintiCounty),
                "Mureș County" => Ok(Self::MuresCounty),
                "Neamț County" => Ok(Self::NeamtCounty),
                "Olt County" => Ok(Self::OltCounty),
                "Prahova County" => Ok(Self::PrahovaCounty),
                "Satu Mare County" => Ok(Self::SatuMareCounty),
                "Sibiu County" => Ok(Self::SibiuCounty),
                "Suceava County" => Ok(Self::SuceavaCounty),
                "Sălaj County" => Ok(Self::SalajCounty),
                "Teleorman County" => Ok(Self::TeleormanCounty),
                "Timiș County" => Ok(Self::TimisCounty),
                "Tulcea County" => Ok(Self::TulceaCounty),
                "Vaslui County" => Ok(Self::VasluiCounty),
                "Vrancea County" => Ok(Self::VranceaCounty),
                "Vâlcea County" => Ok(Self::ValceaCounty),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for PortugalStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "PortugalStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Aveiro District" => Ok(Self::AveiroDistrict),
                "Azores" => Ok(Self::Azores),
                "Beja District" => Ok(Self::BejaDistrict),
                "Braga District" => Ok(Self::BragaDistrict),
                "Bragança District" => Ok(Self::BragancaDistrict),
                "Castelo Branco District" => Ok(Self::CasteloBrancoDistrict),
                "Coimbra District" => Ok(Self::CoimbraDistrict),
                "Faro District" => Ok(Self::FaroDistrict),
                "Guarda District" => Ok(Self::GuardaDistrict),
                "Leiria District" => Ok(Self::LeiriaDistrict),
                "Lisbon District" => Ok(Self::LisbonDistrict),
                "Madeira" => Ok(Self::Madeira),
                "Portalegre District" => Ok(Self::PortalegreDistrict),
                "Porto District" => Ok(Self::PortoDistrict),
                "Santarém District" => Ok(Self::SantaremDistrict),
                "Setúbal District" => Ok(Self::SetubalDistrict),
                "Viana do Castelo District" => Ok(Self::VianaDoCasteloDistrict),
                "Vila Real District" => Ok(Self::VilaRealDistrict),
                "Viseu District" => Ok(Self::ViseuDistrict),
                "Évora District" => Ok(Self::EvoraDistrict),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for SwitzerlandStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "SwitzerlandStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Aargau" => Ok(Self::Aargau),
                "Appenzell Ausserrhoden" => Ok(Self::AppenzellAusserrhoden),
                "Appenzell Innerrhoden" => Ok(Self::AppenzellInnerrhoden),
                "Basel-Landschaft" => Ok(Self::BaselLandschaft),
                "Canton of Fribourg" => Ok(Self::CantonOfFribourg),
                "Canton of Geneva" => Ok(Self::CantonOfGeneva),
                "Canton of Jura" => Ok(Self::CantonOfJura),
                "Canton of Lucerne" => Ok(Self::CantonOfLucerne),
                "Canton of Neuchâtel" => Ok(Self::CantonOfNeuchatel),
                "Canton of Schaffhausen" => Ok(Self::CantonOfSchaffhausen),
                "Canton of Solothurn" => Ok(Self::CantonOfSolothurn),
                "Canton of St. Gallen" => Ok(Self::CantonOfStGallen),
                "Canton of Valais" => Ok(Self::CantonOfValais),
                "Canton of Vaud" => Ok(Self::CantonOfVaud),
                "Canton of Zug" => Ok(Self::CantonOfZug),
                "Glarus" => Ok(Self::Glarus),
                "Graubünden" => Ok(Self::Graubunden),
                "Nidwalden" => Ok(Self::Nidwalden),
                "Obwalden" => Ok(Self::Obwalden),
                "Schwyz" => Ok(Self::Schwyz),
                "Thurgau" => Ok(Self::Thurgau),
                "Ticino" => Ok(Self::Ticino),
                "Uri" => Ok(Self::Uri),
                "canton of Bern" => Ok(Self::CantonOfBern),
                "canton of Zürich" => Ok(Self::CantonOfZurich),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for NorthMacedoniaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "NorthMacedoniaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Aerodrom Municipality" => Ok(Self::AerodromMunicipality),
                "Aračinovo Municipality" => Ok(Self::AracinovoMunicipality),
                "Berovo Municipality" => Ok(Self::BerovoMunicipality),
                "Bitola Municipality" => Ok(Self::BitolaMunicipality),
                "Bogdanci Municipality" => Ok(Self::BogdanciMunicipality),
                "Bogovinje Municipality" => Ok(Self::BogovinjeMunicipality),
                "Bosilovo Municipality" => Ok(Self::BosilovoMunicipality),
                "Brvenica Municipality" => Ok(Self::BrvenicaMunicipality),
                "Butel Municipality" => Ok(Self::ButelMunicipality),
                "Centar Municipality" => Ok(Self::CentarMunicipality),
                "Centar Župa Municipality" => Ok(Self::CentarZupaMunicipality),
                "Debarca Municipality" => Ok(Self::DebarcaMunicipality),
                "Delčevo Municipality" => Ok(Self::DelcevoMunicipality),
                "Demir Hisar Municipality" => Ok(Self::DemirHisarMunicipality),
                "Demir Kapija Municipality" => Ok(Self::DemirKapijaMunicipality),
                "Dojran Municipality" => Ok(Self::DojranMunicipality),
                "Dolneni Municipality" => Ok(Self::DolneniMunicipality),
                "Drugovo Municipality" => Ok(Self::DrugovoMunicipality),
                "Gazi Baba Municipality" => Ok(Self::GaziBabaMunicipality),
                "Gevgelija Municipality" => Ok(Self::GevgelijaMunicipality),
                "Gjorče Petrov Municipality" => Ok(Self::GjorcePetrovMunicipality),
                "Gostivar Municipality" => Ok(Self::GostivarMunicipality),
                "Gradsko Municipality" => Ok(Self::GradskoMunicipality),
                "Greater Skopje" => Ok(Self::GreaterSkopje),
                "Ilinden Municipality" => Ok(Self::IlindenMunicipality),
                "Jegunovce Municipality" => Ok(Self::JegunovceMunicipality),
                "Karbinci" => Ok(Self::Karbinci),
                "Karpoš Municipality" => Ok(Self::KarposMunicipality),
                "Kavadarci Municipality" => Ok(Self::KavadarciMunicipality),
                "Kisela Voda Municipality" => Ok(Self::KiselaVodaMunicipality),
                "Kičevo Municipality" => Ok(Self::KicevoMunicipality),
                "Konče Municipality" => Ok(Self::KonceMunicipality),
                "Kočani Municipality" => Ok(Self::KocaniMunicipality),
                "Kratovo Municipality" => Ok(Self::KratovoMunicipality),
                "Kriva Palanka Municipality" => Ok(Self::KrivaPalankaMunicipality),
                "Krivogaštani Municipality" => Ok(Self::KrivogastaniMunicipality),
                "Kruševo Municipality" => Ok(Self::KrusevoMunicipality),
                "Kumanovo Municipality" => Ok(Self::KumanovoMunicipality),
                "Lipkovo Municipality" => Ok(Self::LipkovoMunicipality),
                "Lozovo Municipality" => Ok(Self::LozovoMunicipality),
                "Makedonska Kamenica Municipality" => Ok(Self::MakedonskaKamenicaMunicipality),
                "Makedonski Brod Municipality" => Ok(Self::MakedonskiBrodMunicipality),
                "Mavrovo and Rostuša Municipality" => Ok(Self::MavrovoAndRostusaMunicipality),
                "Mogila Municipality" => Ok(Self::MogilaMunicipality),
                "Negotino Municipality" => Ok(Self::NegotinoMunicipality),
                "Novaci Municipality" => Ok(Self::NovaciMunicipality),
                "Novo Selo Municipality" => Ok(Self::NovoSeloMunicipality),
                "Ohrid Municipality" => Ok(Self::OhridMunicipality),
                "Oslomej Municipality" => Ok(Self::OslomejMunicipality),
                "Pehčevo Municipality" => Ok(Self::PehcevoMunicipality),
                "Petrovec Municipality" => Ok(Self::PetrovecMunicipality),
                "Plasnica Municipality" => Ok(Self::PlasnicaMunicipality),
                "Prilep Municipality" => Ok(Self::PrilepMunicipality),
                "Probištip Municipality" => Ok(Self::ProbishtipMunicipality),
                "Radoviš Municipality" => Ok(Self::RadovisMunicipality),
                "Rankovce Municipality" => Ok(Self::RankovceMunicipality),
                "Resen Municipality" => Ok(Self::ResenMunicipality),
                "Rosoman Municipality" => Ok(Self::RosomanMunicipality),
                "Saraj Municipality" => Ok(Self::SarajMunicipality),
                "Sopište Municipality" => Ok(Self::SopisteMunicipality),
                "Staro Nagoričane Municipality" => Ok(Self::StaroNagoricaneMunicipality),
                "Struga Municipality" => Ok(Self::StrugaMunicipality),
                "Strumica Municipality" => Ok(Self::StrumicaMunicipality),
                "Studeničani Municipality" => Ok(Self::StudenicaniMunicipality),
                "Sveti Nikole Municipality" => Ok(Self::SvetiNikoleMunicipality),
                "Tearce Municipality" => Ok(Self::TearceMunicipality),
                "Tetovo Municipality" => Ok(Self::TetovoMunicipality),
                "Valandovo Municipality" => Ok(Self::ValandovoMunicipality),
                "Vasilevo Municipality" => Ok(Self::VasilevoMunicipality),
                "Veles Municipality" => Ok(Self::VelesMunicipality),
                "Vevčani Municipality" => Ok(Self::VevcaniMunicipality),
                "Vinica Municipality" => Ok(Self::VinicaMunicipality),
                "Vraneštica Municipality" => Ok(Self::VranesticaMunicipality),
                "Vrapčište Municipality" => Ok(Self::VrapcisteMunicipality),
                "Zajas Municipality" => Ok(Self::ZajasMunicipality),
                "Zelenikovo Municipality" => Ok(Self::ZelenikovoMunicipality),
                "Zrnovci Municipality" => Ok(Self::ZrnovciMunicipality),
                "Čair Municipality" => Ok(Self::CairMunicipality),
                "Čaška Municipality" => Ok(Self::CaskaMunicipality),
                "Češinovo-Obleševo Municipality" => Ok(Self::CesinovoOblesevoMunicipality),
                "Čučer-Sandevo Municipality" => Ok(Self::CucerSandevoMunicipality),
                "Štip Municipality" => Ok(Self::StipMunicipality),
                "Šuto Orizari Municipality" => Ok(Self::ShutoOrizariMunicipality),
                "Želino Municipality" => Ok(Self::ZelinoMunicipality),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for MontenegroStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "MontenegroStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Andrijevica Municipality" => Ok(Self::AndrijevicaMunicipality),
                "Bar Municipality" => Ok(Self::BarMunicipality),
                "Berane Municipality" => Ok(Self::BeraneMunicipality),
                "Bijelo Polje Municipality" => Ok(Self::BijeloPoljeMunicipality),
                "Budva Municipality" => Ok(Self::BudvaMunicipality),
                "Danilovgrad Municipality" => Ok(Self::DanilovgradMunicipality),
                "Gusinje Municipality" => Ok(Self::GusinjeMunicipality),
                "Kolašin Municipality" => Ok(Self::KolasinMunicipality),
                "Kotor Municipality" => Ok(Self::KotorMunicipality),
                "Mojkovac Municipality" => Ok(Self::MojkovacMunicipality),
                "Nikšić Municipality" => Ok(Self::NiksicMunicipality),
                "Petnjica Municipality" => Ok(Self::PetnjicaMunicipality),
                "Plav Municipality" => Ok(Self::PlavMunicipality),
                "Pljevlja Municipality" => Ok(Self::PljevljaMunicipality),
                "Plužine Municipality" => Ok(Self::PlužineMunicipality),
                "Podgorica Municipality" => Ok(Self::PodgoricaMunicipality),
                "Rožaje Municipality" => Ok(Self::RožajeMunicipality),
                "Tivat Municipality" => Ok(Self::TivatMunicipality),
                "Ulcinj Municipality" => Ok(Self::UlcinjMunicipality),
                "Žabljak Municipality" => Ok(Self::ŽabljakMunicipality),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for MonacoStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "MonacoStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Monaco" => Ok(Self::Monaco),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for NetherlandsStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "NetherlandsStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Bonaire" => Ok(Self::Bonaire),
                "Drenthe" => Ok(Self::Drenthe),
                "Flevoland" => Ok(Self::Flevoland),
                "Friesland" => Ok(Self::Friesland),
                "Gelderland" => Ok(Self::Gelderland),
                "Groningen" => Ok(Self::Groningen),
                "Limburg" => Ok(Self::Limburg),
                "North Brabant" => Ok(Self::NorthBrabant),
                "North Holland" => Ok(Self::NorthHolland),
                "Overijssel" => Ok(Self::Overijssel),
                "Saba" => Ok(Self::Saba),
                "Sint Eustatius" => Ok(Self::SintEustatius),
                "South Holland" => Ok(Self::SouthHolland),
                "Utrecht" => Ok(Self::Utrecht),
                "Zeeland" => Ok(Self::Zeeland),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for MoldovaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "MoldovaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Anenii Noi District" => Ok(Self::AneniiNoiDistrict),
                "Basarabeasca District" => Ok(Self::BasarabeascaDistrict),
                "Bender Municipality" => Ok(Self::BenderMunicipality),
                "Briceni District" => Ok(Self::BriceniDistrict),
                "Bălți Municipality" => Ok(Self::BălțiMunicipality),
                "Cahul District" => Ok(Self::CahulDistrict),
                "Cantemir District" => Ok(Self::CantemirDistrict),
                "Chișinău Municipality" => Ok(Self::ChișinăuMunicipality),
                "Cimișlia District" => Ok(Self::CimișliaDistrict),
                "Criuleni District" => Ok(Self::CriuleniDistrict),
                "Călărași District" => Ok(Self::CălărașiDistrict),
                "Căușeni District" => Ok(Self::CăușeniDistrict),
                "Dondușeni District" => Ok(Self::DondușeniDistrict),
                "Drochia District" => Ok(Self::DrochiaDistrict),
                "Dubăsari District" => Ok(Self::DubăsariDistrict),
                "Edineț District" => Ok(Self::EdinețDistrict),
                "Florești District" => Ok(Self::FloreștiDistrict),
                "Fălești District" => Ok(Self::FăleștiDistrict),
                "Găgăuzia" => Ok(Self::Găgăuzia),
                "Glodeni District" => Ok(Self::GlodeniDistrict),
                "Hîncești District" => Ok(Self::HînceștiDistrict),
                "Ialoveni District" => Ok(Self::IaloveniDistrict),
                "Nisporeni District" => Ok(Self::NisporeniDistrict),
                "Ocnița District" => Ok(Self::OcnițaDistrict),
                "Orhei District" => Ok(Self::OrheiDistrict),
                "Rezina District" => Ok(Self::RezinaDistrict),
                "Rîșcani District" => Ok(Self::RîșcaniDistrict),
                "Soroca District" => Ok(Self::SorocaDistrict),
                "Strășeni District" => Ok(Self::StrășeniDistrict),
                "Sîngerei District" => Ok(Self::SîngereiDistrict),
                "Taraclia District" => Ok(Self::TaracliaDistrict),
                "Telenești District" => Ok(Self::TeleneștiDistrict),
                "Transnistria Autonomous Territorial Unit" => {
                    Ok(Self::TransnistriaAutonomousTerritorialUnit)
                }
                "Ungheni District" => Ok(Self::UngheniDistrict),
                "Șoldănești District" => Ok(Self::ȘoldăneștiDistrict),
                "Ștefan Vodă District" => Ok(Self::ȘtefanVodăDistrict),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for LithuaniaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "LithuaniaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Akmenė District Municipality" => Ok(Self::AkmeneDistrictMunicipality),
                "Alytus City Municipality" => Ok(Self::AlytusCityMunicipality),
                "Alytus County" => Ok(Self::AlytusCounty),
                "Alytus District Municipality" => Ok(Self::AlytusDistrictMunicipality),
                "Birštonas Municipality" => Ok(Self::BirstonasMunicipality),
                "Biržai District Municipality" => Ok(Self::BirzaiDistrictMunicipality),
                "Druskininkai municipality" => Ok(Self::DruskininkaiMunicipality),
                "Elektrėnai municipality" => Ok(Self::ElektrenaiMunicipality),
                "Ignalina District Municipality" => Ok(Self::IgnalinaDistrictMunicipality),
                "Jonava District Municipality" => Ok(Self::JonavaDistrictMunicipality),
                "Joniškis District Municipality" => Ok(Self::JoniskisDistrictMunicipality),
                "Jurbarkas District Municipality" => Ok(Self::JurbarkasDistrictMunicipality),
                "Kaišiadorys District Municipality" => Ok(Self::KaisiadorysDistrictMunicipality),
                "Kalvarija municipality" => Ok(Self::KalvarijaMunicipality),
                "Kaunas City Municipality" => Ok(Self::KaunasCityMunicipality),
                "Kaunas County" => Ok(Self::KaunasCounty),
                "Kaunas District Municipality" => Ok(Self::KaunasDistrictMunicipality),
                "Kazlų Rūda municipality" => Ok(Self::KazluRudaMunicipality),
                "Kelmė District Municipality" => Ok(Self::KelmeDistrictMunicipality),
                "Klaipeda City Municipality" => Ok(Self::KlaipedaCityMunicipality),
                "Klaipėda County" => Ok(Self::KlaipedaCounty),
                "Klaipėda District Municipality" => Ok(Self::KlaipedaDistrictMunicipality),
                "Kretinga District Municipality" => Ok(Self::KretingaDistrictMunicipality),
                "Kupiškis District Municipality" => Ok(Self::KupiskisDistrictMunicipality),
                "Kėdainiai District Municipality" => Ok(Self::KedainiaiDistrictMunicipality),
                "Lazdijai District Municipality" => Ok(Self::LazdijaiDistrictMunicipality),
                "Marijampolė County" => Ok(Self::MarijampoleCounty),
                "Marijampolė Municipality" => Ok(Self::MarijampoleMunicipality),
                "Mažeikiai District Municipality" => Ok(Self::MazeikiaiDistrictMunicipality),
                "Molėtai District Municipality" => Ok(Self::MoletaiDistrictMunicipality),
                "Neringa Municipality" => Ok(Self::NeringaMunicipality),
                "Pagėgiai municipality" => Ok(Self::PagegiaiMunicipality),
                "Pakruojis District Municipality" => Ok(Self::PakruojisDistrictMunicipality),
                "Palanga City Municipality" => Ok(Self::PalangaCityMunicipality),
                "Panevėžys City Municipality" => Ok(Self::PanevezysCityMunicipality),
                "Panevėžys County" => Ok(Self::PanevezysCounty),
                "Panevėžys District Municipality" => Ok(Self::PanevezysDistrictMunicipality),
                "Pasvalys District Municipality" => Ok(Self::PasvalysDistrictMunicipality),
                "Plungė District Municipality" => Ok(Self::PlungeDistrictMunicipality),
                "Prienai District Municipality" => Ok(Self::PrienaiDistrictMunicipality),
                "Radviliškis District Municipality" => Ok(Self::RadviliskisDistrictMunicipality),
                "Raseiniai District Municipality" => Ok(Self::RaseiniaiDistrictMunicipality),
                "Rietavas municipality" => Ok(Self::RietavasMunicipality),
                "Rokiškis District Municipality" => Ok(Self::RokiskisDistrictMunicipality),
                "Skuodas District Municipality" => Ok(Self::SkuodasDistrictMunicipality),
                "Tauragė County" => Ok(Self::TaurageCounty),
                "Tauragė District Municipality" => Ok(Self::TaurageDistrictMunicipality),
                "Telšiai County" => Ok(Self::TelsiaiCounty),
                "Telšiai District Municipality" => Ok(Self::TelsiaiDistrictMunicipality),
                "Trakai District Municipality" => Ok(Self::TrakaiDistrictMunicipality),
                "Ukmergė District Municipality" => Ok(Self::UkmergeDistrictMunicipality),
                "Utena County" => Ok(Self::UtenaCounty),
                "Utena District Municipality" => Ok(Self::UtenaDistrictMunicipality),
                "Varėna District Municipality" => Ok(Self::VarenaDistrictMunicipality),
                "Vilkaviškis District Municipality" => Ok(Self::VilkaviskisDistrictMunicipality),
                "Vilnius City Municipality" => Ok(Self::VilniusCityMunicipality),
                "Vilnius County" => Ok(Self::VilniusCounty),
                "Vilnius District Municipality" => Ok(Self::VilniusDistrictMunicipality),
                "Visaginas Municipality" => Ok(Self::VisaginasMunicipality),
                "Zarasai District Municipality" => Ok(Self::ZarasaiDistrictMunicipality),
                "Šakiai District Municipality" => Ok(Self::SakiaiDistrictMunicipality),
                "Šalčininkai District Municipality" => Ok(Self::SalcininkaiDistrictMunicipality),
                "Šiauliai City Municipality" => Ok(Self::SiauliaiCityMunicipality),
                "Šiauliai County" => Ok(Self::SiauliaiCounty),
                "Šiauliai District Municipality" => Ok(Self::SiauliaiDistrictMunicipality),
                "Šilalė District Municipality" => Ok(Self::SilaleDistrictMunicipality),
                "Šilutė District Municipality" => Ok(Self::SiluteDistrictMunicipality),
                "Širvintos District Municipality" => Ok(Self::SirvintosDistrictMunicipality),
                "Švenčionys District Municipality" => Ok(Self::SvencionysDistrictMunicipality),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for LiechtensteinStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "LiechtensteinStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Balzers" => Ok(Self::Balzers),
                "Eschen" => Ok(Self::Eschen),
                "Gamprin" => Ok(Self::Gamprin),
                "Mauren" => Ok(Self::Mauren),
                "Planken" => Ok(Self::Planken),
                "Ruggell" => Ok(Self::Ruggell),
                "Schaan" => Ok(Self::Schaan),
                "Schellenberg" => Ok(Self::Schellenberg),
                "Triesen" => Ok(Self::Triesen),
                "Triesenberg" => Ok(Self::Triesenberg),
                "Vaduz" => Ok(Self::Vaduz),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for LatviaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "LatviaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Aglona Municipality" => Ok(Self::AglonaMunicipality),
                "Aizkraukle Municipality" => Ok(Self::AizkraukleMunicipality),
                "Aizpute Municipality" => Ok(Self::AizputeMunicipality),
                "Aknīste Municipality" => Ok(Self::AknīsteMunicipality),
                "Aloja Municipality" => Ok(Self::AlojaMunicipality),
                "Alsunga Municipality" => Ok(Self::AlsungaMunicipality),
                "Alūksne Municipality" => Ok(Self::AlūksneMunicipality),
                "Amata Municipality" => Ok(Self::AmataMunicipality),
                "Ape Municipality" => Ok(Self::ApeMunicipality),
                "Auce Municipality" => Ok(Self::AuceMunicipality),
                "Babīte Municipality" => Ok(Self::BabīteMunicipality),
                "Baldone Municipality" => Ok(Self::BaldoneMunicipality),
                "Baltinava Municipality" => Ok(Self::BaltinavaMunicipality),
                "Balvi Municipality" => Ok(Self::BalviMunicipality),
                "Bauska Municipality" => Ok(Self::BauskaMunicipality),
                "Beverīna Municipality" => Ok(Self::BeverīnaMunicipality),
                "Brocēni Municipality" => Ok(Self::BrocēniMunicipality),
                "Burtnieki Municipality" => Ok(Self::BurtniekiMunicipality),
                "Carnikava Municipality" => Ok(Self::CarnikavaMunicipality),
                "Cesvaine Municipality" => Ok(Self::CesvaineMunicipality),
                "Cibla Municipality" => Ok(Self::CiblaMunicipality),
                "Cēsis Municipality" => Ok(Self::CēsisMunicipality),
                "Dagda Municipality" => Ok(Self::DagdaMunicipality),
                "Daugavpils" => Ok(Self::Daugavpils),
                "Daugavpils Municipality" => Ok(Self::DaugavpilsMunicipality),
                "Dobele Municipality" => Ok(Self::DobeleMunicipality),
                "Dundaga Municipality" => Ok(Self::DundagaMunicipality),
                "Durbe Municipality" => Ok(Self::DurbeMunicipality),
                "Engure Municipality" => Ok(Self::EngureMunicipality),
                "Garkalne Municipality" => Ok(Self::GarkalneMunicipality),
                "Grobiņa Municipality" => Ok(Self::GrobiņaMunicipality),
                "Gulbene Municipality" => Ok(Self::GulbeneMunicipality),
                "Iecava Municipality" => Ok(Self::IecavaMunicipality),
                "Ikšķile Municipality" => Ok(Self::IkšķileMunicipality),
                "Ilūkste Municipalityy" => Ok(Self::IlūksteMunicipality),
                "Inčukalns Municipality" => Ok(Self::InčukalnsMunicipality),
                "Jaunjelgava Municipality" => Ok(Self::JaunjelgavaMunicipality),
                "Jaunpiebalga Municipality" => Ok(Self::JaunpiebalgaMunicipality),
                "Jaunpils Municipality" => Ok(Self::JaunpilsMunicipality),
                "Jelgava" => Ok(Self::Jelgava),
                "Jelgava Municipality" => Ok(Self::JelgavaMunicipality),
                "Jēkabpils" => Ok(Self::Jēkabpils),
                "Jēkabpils Municipality" => Ok(Self::JēkabpilsMunicipality),
                "Jūrmala" => Ok(Self::Jūrmala),
                "Kandava Municipality" => Ok(Self::KandavaMunicipality),
                "Kocēni Municipality" => Ok(Self::KocēniMunicipality),
                "Koknese Municipality" => Ok(Self::KokneseMunicipality),
                "Krimulda Municipality" => Ok(Self::KrimuldaMunicipality),
                "Krustpils Municipality" => Ok(Self::KrustpilsMunicipality),
                "Krāslava Municipality" => Ok(Self::KrāslavaMunicipality),
                "Kuldīga Municipality" => Ok(Self::KuldīgaMunicipality),
                "Kārsava Municipality" => Ok(Self::KārsavaMunicipality),
                "Lielvārde Municipality" => Ok(Self::LielvārdeMunicipality),
                "Liepāja" => Ok(Self::Liepāja),
                "Limbaži Municipality" => Ok(Self::LimbažiMunicipality),
                "Lubāna Municipality" => Ok(Self::LubānaMunicipality),
                "Ludza Municipality" => Ok(Self::LudzaMunicipality),
                "Līgatne Municipality" => Ok(Self::LīgatneMunicipality),
                "Līvāni Municipality" => Ok(Self::LīvāniMunicipality),
                "Madona Municipality" => Ok(Self::MadonaMunicipality),
                "Mazsalaca Municipality" => Ok(Self::MazsalacaMunicipality),
                "Mālpils Municipality" => Ok(Self::MālpilsMunicipality),
                "Mārupe Municipality" => Ok(Self::MārupeMunicipality),
                "Mērsrags Municipality" => Ok(Self::MērsragsMunicipality),
                "Naukšēni Municipality" => Ok(Self::NaukšēniMunicipality),
                "Nereta Municipality" => Ok(Self::NeretaMunicipality),
                "Nīca Municipality" => Ok(Self::NīcaMunicipality),
                "Ogre Municipality" => Ok(Self::OgreMunicipality),
                "Olaine Municipality" => Ok(Self::OlaineMunicipality),
                "Ozolnieki Municipality" => Ok(Self::OzolniekiMunicipality),
                "Preiļi Municipality" => Ok(Self::PreiļiMunicipality),
                "Priekule Municipality" => Ok(Self::PriekuleMunicipality),
                "Priekuļi Municipality" => Ok(Self::PriekuļiMunicipality),
                "Pārgauja Municipality" => Ok(Self::PārgaujaMunicipality),
                "Pāvilosta Municipality" => Ok(Self::PāvilostaMunicipality),
                "Pļaviņas Municipality" => Ok(Self::PļaviņasMunicipality),
                "Rauna Municipality" => Ok(Self::RaunaMunicipality),
                "Riebiņi Municipality" => Ok(Self::RiebiņiMunicipality),
                "Riga" => Ok(Self::Riga),
                "Roja Municipality" => Ok(Self::RojaMunicipality),
                "Ropaži Municipality" => Ok(Self::RopažiMunicipality),
                "Rucava Municipality" => Ok(Self::RucavaMunicipality),
                "Rugāji Municipality" => Ok(Self::RugājiMunicipality),
                "Rundāle Municipality" => Ok(Self::RundāleMunicipality),
                "Rēzekne" => Ok(Self::Rēzekne),
                "Rēzekne Municipality" => Ok(Self::RēzekneMunicipality),
                "Rūjiena Municipality" => Ok(Self::RūjienaMunicipality),
                "Sala Municipality" => Ok(Self::SalaMunicipality),
                "Salacgrīva Municipality" => Ok(Self::SalacgrīvaMunicipality),
                "Salaspils Municipality" => Ok(Self::SalaspilsMunicipality),
                "Saldus Municipality" => Ok(Self::SaldusMunicipality),
                "Saulkrasti Municipality" => Ok(Self::SaulkrastiMunicipality),
                "Sigulda Municipality" => Ok(Self::SiguldaMunicipality),
                "Skrunda Municipality" => Ok(Self::SkrundaMunicipality),
                "Skrīveri Municipality" => Ok(Self::SkrīveriMunicipality),
                "Smiltene Municipality" => Ok(Self::SmilteneMunicipality),
                "Stopiņi Municipality" => Ok(Self::StopiņiMunicipality),
                "Strenči Municipality" => Ok(Self::StrenčiMunicipality),
                "Sēja Municipality" => Ok(Self::SējaMunicipality),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for MaltaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "MaltaStatesAbbreviation");

        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Attard" => Ok(Self::Attard),
                "Balzan" => Ok(Self::Balzan),
                "Birgu" => Ok(Self::Birgu),
                "Birkirkara" => Ok(Self::Birkirkara),
                "Birżebbuġa" => Ok(Self::Birżebbuġa),
                "Cospicua" => Ok(Self::Cospicua),
                "Dingli" => Ok(Self::Dingli),
                "Fgura" => Ok(Self::Fgura),
                "Floriana" => Ok(Self::Floriana),
                "Fontana" => Ok(Self::Fontana),
                "Gudja" => Ok(Self::Gudja),
                "Gżira" => Ok(Self::Gżira),
                "Għajnsielem" => Ok(Self::Għajnsielem),
                "Għarb" => Ok(Self::Għarb),
                "Għargħur" => Ok(Self::Għargħur),
                "Għasri" => Ok(Self::Għasri),
                "Għaxaq" => Ok(Self::Għaxaq),
                "Ħamrun" => Ok(Self::Ħamrun),
                "Iklin" => Ok(Self::Iklin),
                "Senglea" => Ok(Self::Senglea),
                "Kalkara" => Ok(Self::Kalkara),
                "Kerċem" => Ok(Self::Kerċem),
                "Kirkop" => Ok(Self::Kirkop),
                "Lija" => Ok(Self::Lija),
                "Luqa" => Ok(Self::Luqa),
                "Marsa" => Ok(Self::Marsa),
                "Marsaskala" => Ok(Self::Marsaskala),
                "Marsaxlokk" => Ok(Self::Marsaxlokk),
                "Mdina" => Ok(Self::Mdina),
                "Mellieħa" => Ok(Self::Mellieħa),
                "Mosta" => Ok(Self::Mosta),
                "Mqabba" => Ok(Self::Mqabba),
                "Msida" => Ok(Self::Msida),
                "Mtarfa" => Ok(Self::Mtarfa),
                "Munxar" => Ok(Self::Munxar),
                "Mġarr" => Ok(Self::Mġarr),
                "Nadur" => Ok(Self::Nadur),
                "Naxxar" => Ok(Self::Naxxar),
                "Paola" => Ok(Self::Paola),
                "Pembroke" => Ok(Self::Pembroke),
                "Pietà" => Ok(Self::Pietà),
                "Qala" => Ok(Self::Qala),
                "Qormi" => Ok(Self::Qormi),
                "Qrendi" => Ok(Self::Qrendi),
                "Rabat" => Ok(Self::Rabat),
                "Saint Lawrence" => Ok(Self::SaintLawrence),
                "San Ġwann" => Ok(Self::SanĠwann),
                "Sannat" => Ok(Self::Sannat),
                "Santa Luċija" => Ok(Self::SantaLuċija),
                "Santa Venera" => Ok(Self::SantaVenera),
                "Siġġiewi" => Ok(Self::Siġġiewi),
                "Sliema" => Ok(Self::Sliema),
                "St. Julian's" => Ok(Self::StJulians),
                "St. Paul's Bay" => Ok(Self::StPaulsBay),
                "Swieqi" => Ok(Self::Swieqi),
                "Ta' Xbiex" => Ok(Self::TaXbiex),
                "Tarxien" => Ok(Self::Tarxien),
                "Valletta" => Ok(Self::Valletta),
                "Victoria" => Ok(Self::Victoria),
                "Xagħra" => Ok(Self::Xagħra),
                "Xewkija" => Ok(Self::Xewkija),
                "Xgħajra" => Ok(Self::Xgħajra),
                "Żabbar" => Ok(Self::Żabbar),
                "Żebbuġ Gozo" => Ok(Self::ŻebbuġGozo),
                "Żebbuġ Malta" => Ok(Self::ŻebbuġMalta),
                "Żejtun" => Ok(Self::Żejtun),
                "Żurrieq" => Ok(Self::Żurrieq),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for BelarusStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "BelarusStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Brest Region" => Ok(Self::BrestRegion),
                "Gomel Region" => Ok(Self::GomelRegion),
                "Grodno Region" => Ok(Self::GrodnoRegion),
                "Minsk" => Ok(Self::Minsk),
                "Minsk Region" => Ok(Self::MinskRegion),
                "Mogilev Region" => Ok(Self::MogilevRegion),
                "Vitebsk Region" => Ok(Self::VitebskRegion),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for IrelandStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "IrelandStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Connacht" => Ok(Self::Connacht),
                "County Carlow" => Ok(Self::CountyCarlow),
                "County Cavan" => Ok(Self::CountyCavan),
                "County Clare" => Ok(Self::CountyClare),
                "County Cork" => Ok(Self::CountyCork),
                "County Donegal" => Ok(Self::CountyDonegal),
                "County Dublin" => Ok(Self::CountyDublin),
                "County Galway" => Ok(Self::CountyGalway),
                "County Kerry" => Ok(Self::CountyKerry),
                "County Kildare" => Ok(Self::CountyKildare),
                "County Kilkenny" => Ok(Self::CountyKilkenny),
                "County Laois" => Ok(Self::CountyLaois),
                "County Limerick" => Ok(Self::CountyLimerick),
                "County Longford" => Ok(Self::CountyLongford),
                "County Louth" => Ok(Self::CountyLouth),
                "County Mayo" => Ok(Self::CountyMayo),
                "County Meath" => Ok(Self::CountyMeath),
                "County Monaghan" => Ok(Self::CountyMonaghan),
                "County Offaly" => Ok(Self::CountyOffaly),
                "County Roscommon" => Ok(Self::CountyRoscommon),
                "County Sligo" => Ok(Self::CountySligo),
                "County Tipperary" => Ok(Self::CountyTipperary),
                "County Waterford" => Ok(Self::CountyWaterford),
                "County Westmeath" => Ok(Self::CountyWestmeath),
                "County Wexford" => Ok(Self::CountyWexford),
                "County Wicklow" => Ok(Self::CountyWicklow),
                "Leinster" => Ok(Self::Leinster),
                "Munster" => Ok(Self::Munster),
                "Ulster" => Ok(Self::Ulster),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for IcelandStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "IcelandStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Capital Region" => Ok(Self::CapitalRegion),
                "Eastern Region" => Ok(Self::EasternRegion),
                "Northeastern Region" => Ok(Self::NortheasternRegion),
                "Northwestern Region" => Ok(Self::NorthwesternRegion),
                "Southern Peninsula Region" => Ok(Self::SouthernPeninsulaRegion),
                "Southern Region" => Ok(Self::SouthernRegion),
                "Western Region" => Ok(Self::WesternRegion),
                "Westfjords" => Ok(Self::Westfjords),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for HungaryStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "HungaryStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Baranya County" => Ok(Self::BaranyaCounty),
                "Borsod-Abaúj-Zemplén County" => Ok(Self::BorsodAbaujZemplenCounty),
                "Budapest" => Ok(Self::Budapest),
                "Bács-Kiskun County" => Ok(Self::BacsKiskunCounty),
                "Békés County" => Ok(Self::BekesCounty),
                "Békéscsaba" => Ok(Self::Bekescsaba),
                "Csongrád County" => Ok(Self::CsongradCounty),
                "Debrecen" => Ok(Self::Debrecen),
                "Dunaújváros" => Ok(Self::Dunaujvaros),
                "Eger" => Ok(Self::Eger),
                "Fejér County" => Ok(Self::FejerCounty),
                "Győr" => Ok(Self::Gyor),
                "Győr-Moson-Sopron County" => Ok(Self::GyorMosonSopronCounty),
                "Hajdú-Bihar County" => Ok(Self::HajduBiharCounty),
                "Heves County" => Ok(Self::HevesCounty),
                "Hódmezővásárhely" => Ok(Self::Hodmezovasarhely),
                "Jász-Nagykun-Szolnok County" => Ok(Self::JaszNagykunSzolnokCounty),
                "Kaposvár" => Ok(Self::Kaposvar),
                "Kecskemét" => Ok(Self::Kecskemet),
                "Miskolc" => Ok(Self::Miskolc),
                "Nagykanizsa" => Ok(Self::Nagykanizsa),
                "Nyíregyháza" => Ok(Self::Nyiregyhaza),
                "Nógrád County" => Ok(Self::NogradCounty),
                "Pest County" => Ok(Self::PestCounty),
                "Pécs" => Ok(Self::Pecs),
                "Salgótarján" => Ok(Self::Salgotarjan),
                "Somogy County" => Ok(Self::SomogyCounty),
                "Sopron" => Ok(Self::Sopron),
                "Szabolcs-Szatmár-Bereg County" => Ok(Self::SzabolcsSzatmarBeregCounty),
                "Szeged" => Ok(Self::Szeged),
                "Szekszárd" => Ok(Self::Szekszard),
                "Szolnok" => Ok(Self::Szolnok),
                "Szombathely" => Ok(Self::Szombathely),
                "Székesfehérvár" => Ok(Self::Szekesfehervar),
                "Tatabánya" => Ok(Self::Tatabanya),
                "Tolna County" => Ok(Self::TolnaCounty),
                "Vas County" => Ok(Self::VasCounty),
                "Veszprém" => Ok(Self::Veszprem),
                "Veszprém County" => Ok(Self::VeszpremCounty),
                "Zala County" => Ok(Self::ZalaCounty),
                "Zalaegerszeg" => Ok(Self::Zalaegerszeg),
                "Érd" => Ok(Self::Erd),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for GreeceStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "GreeceStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Achaea Regional Unit" => Ok(Self::AchaeaRegionalUnit),
                "Aetolia-Acarnania Regional Unit" => Ok(Self::AetoliaAcarnaniaRegionalUnit),
                "Arcadia Prefecture" => Ok(Self::ArcadiaPrefecture),
                "Argolis Regional Unit" => Ok(Self::ArgolisRegionalUnit),
                "Attica Region" => Ok(Self::AtticaRegion),
                "Boeotia Regional Unit" => Ok(Self::BoeotiaRegionalUnit),
                "Central Greece Region" => Ok(Self::CentralGreeceRegion),
                "Central Macedonia" => Ok(Self::CentralMacedonia),
                "Chania Regional Unit" => Ok(Self::ChaniaRegionalUnit),
                "Corfu Prefecture" => Ok(Self::CorfuPrefecture),
                "Corinthia Regional Unit" => Ok(Self::CorinthiaRegionalUnit),
                "Crete Region" => Ok(Self::CreteRegion),
                "Drama Regional Unit" => Ok(Self::DramaRegionalUnit),
                "East Attica Regional Unit" => Ok(Self::EastAtticaRegionalUnit),
                "East Macedonia and Thrace" => Ok(Self::EastMacedoniaAndThrace),
                "Epirus Region" => Ok(Self::EpirusRegion),
                "Euboea" => Ok(Self::Euboea),
                "Grevena Prefecture" => Ok(Self::GrevenaPrefecture),
                "Imathia Regional Unit" => Ok(Self::ImathiaRegionalUnit),
                "Ioannina Regional Unit" => Ok(Self::IoanninaRegionalUnit),
                "Ionian Islands Region" => Ok(Self::IonianIslandsRegion),
                "Karditsa Regional Unit" => Ok(Self::KarditsaRegionalUnit),
                "Kastoria Regional Unit" => Ok(Self::KastoriaRegionalUnit),
                "Kefalonia Prefecture" => Ok(Self::KefaloniaPrefecture),
                "Kilkis Regional Unit" => Ok(Self::KilkisRegionalUnit),
                "Kozani Prefecture" => Ok(Self::KozaniPrefecture),
                "Laconia" => Ok(Self::Laconia),
                "Larissa Prefecture" => Ok(Self::LarissaPrefecture),
                "Lefkada Regional Unit" => Ok(Self::LefkadaRegionalUnit),
                "Pella Regional Unit" => Ok(Self::PellaRegionalUnit),
                "Peloponnese Region" => Ok(Self::PeloponneseRegion),
                "Phthiotis Prefecture" => Ok(Self::PhthiotisPrefecture),
                "Preveza Prefecture" => Ok(Self::PrevezaPrefecture),
                "Serres Prefecture" => Ok(Self::SerresPrefecture),
                "South Aegean" => Ok(Self::SouthAegean),
                "Thessaloniki Regional Unit" => Ok(Self::ThessalonikiRegionalUnit),
                "West Greece Region" => Ok(Self::WestGreeceRegion),
                "West Macedonia Region" => Ok(Self::WestMacedoniaRegion),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for FinlandStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "FinlandStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Central Finland" => Ok(Self::CentralFinland),
                "Central Ostrobothnia" => Ok(Self::CentralOstrobothnia),
                "Eastern Finland Province" => Ok(Self::EasternFinlandProvince),
                "Finland Proper" => Ok(Self::FinlandProper),
                "Kainuu" => Ok(Self::Kainuu),
                "Kymenlaakso" => Ok(Self::Kymenlaakso),
                "Lapland" => Ok(Self::Lapland),
                "North Karelia" => Ok(Self::NorthKarelia),
                "Northern Ostrobothnia" => Ok(Self::NorthernOstrobothnia),
                "Northern Savonia" => Ok(Self::NorthernSavonia),
                "Ostrobothnia" => Ok(Self::Ostrobothnia),
                "Oulu Province" => Ok(Self::OuluProvince),
                "Pirkanmaa" => Ok(Self::Pirkanmaa),
                "Päijänne Tavastia" => Ok(Self::PaijanneTavastia),
                "Satakunta" => Ok(Self::Satakunta),
                "South Karelia" => Ok(Self::SouthKarelia),
                "Southern Ostrobothnia" => Ok(Self::SouthernOstrobothnia),
                "Southern Savonia" => Ok(Self::SouthernSavonia),
                "Tavastia Proper" => Ok(Self::TavastiaProper),
                "Uusimaa" => Ok(Self::Uusimaa),
                "Åland Islands" => Ok(Self::AlandIslands),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for DenmarkStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "DenmarkStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Capital Region of Denmark" => Ok(Self::CapitalRegionOfDenmark),
                "Central Denmark Region" => Ok(Self::CentralDenmarkRegion),
                "North Denmark Region" => Ok(Self::NorthDenmarkRegion),
                "Region Zealand" => Ok(Self::RegionZealand),
                "Region of Southern Denmark" => Ok(Self::RegionOfSouthernDenmark),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for CzechRepublicStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "CzechRepublicStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Benešov District" => Ok(Self::BenesovDistrict),
                "Beroun District" => Ok(Self::BerounDistrict),
                "Blansko District" => Ok(Self::BlanskoDistrict),
                "Brno-City District" => Ok(Self::BrnoCityDistrict),
                "Brno-Country District" => Ok(Self::BrnoCountryDistrict),
                "Bruntál District" => Ok(Self::BruntalDistrict),
                "Břeclav District" => Ok(Self::BreclavDistrict),
                "Central Bohemian Region" => Ok(Self::CentralBohemianRegion),
                "Cheb District" => Ok(Self::ChebDistrict),
                "Chomutov District" => Ok(Self::ChomutovDistrict),
                "Chrudim District" => Ok(Self::ChrudimDistrict),
                "Domažlice Distric" => Ok(Self::DomazliceDistrict),
                "Děčín District" => Ok(Self::DecinDistrict),
                "Frýdek-Místek District" => Ok(Self::FrydekMistekDistrict),
                "Havlíčkův Brod District" => Ok(Self::HavlickuvBrodDistrict),
                "Hodonín District" => Ok(Self::HodoninDistrict),
                "Horní Počernice" => Ok(Self::HorniPocernice),
                "Hradec Králové District" => Ok(Self::HradecKraloveDistrict),
                "Hradec Králové Region" => Ok(Self::HradecKraloveRegion),
                "Jablonec nad Nisou District" => Ok(Self::JablonecNadNisouDistrict),
                "Jeseník District" => Ok(Self::JesenikDistrict),
                "Jihlava District" => Ok(Self::JihlavaDistrict),
                "Jindřichův Hradec District" => Ok(Self::JindrichuvHradecDistrict),
                "Jičín District" => Ok(Self::JicinDistrict),
                "Karlovy Vary District" => Ok(Self::KarlovyVaryDistrict),
                "Karlovy Vary Region" => Ok(Self::KarlovyVaryRegion),
                "Karviná District" => Ok(Self::KarvinaDistrict),
                "Kladno District" => Ok(Self::KladnoDistrict),
                "Klatovy District" => Ok(Self::KlatovyDistrict),
                "Kolín District" => Ok(Self::KolinDistrict),
                "Kroměříž District" => Ok(Self::KromerizDistrict),
                "Liberec District" => Ok(Self::LiberecDistrict),
                "Liberec Region" => Ok(Self::LiberecRegion),
                "Litoměřice District" => Ok(Self::LitomericeDistrict),
                "Louny District" => Ok(Self::LounyDistrict),
                "Mladá Boleslav District" => Ok(Self::MladaBoleslavDistrict),
                "Moravian-Silesian Region" => Ok(Self::MoravianSilesianRegion),
                "Most District" => Ok(Self::MostDistrict),
                "Mělník District" => Ok(Self::MelnikDistrict),
                "Nový Jičín District" => Ok(Self::NovyJicinDistrict),
                "Nymburk District" => Ok(Self::NymburkDistrict),
                "Náchod District" => Ok(Self::NachodDistrict),
                "Olomouc District" => Ok(Self::OlomoucDistrict),
                "Olomouc Region" => Ok(Self::OlomoucRegion),
                "Opava District" => Ok(Self::OpavaDistrict),
                "Ostrava-City District" => Ok(Self::OstravaCityDistrict),
                "Pardubice District" => Ok(Self::PardubiceDistrict),
                "Pardubice Region" => Ok(Self::PardubiceRegion),
                "Pelhřimov District" => Ok(Self::PelhrimovDistrict),
                "Plzeň Region" => Ok(Self::PlzenRegion),
                "Plzeň-City District" => Ok(Self::PlzenCityDistrict),
                "Plzeň-North District" => Ok(Self::PlzenNorthDistrict),
                "Plzeň-South District" => Ok(Self::PlzenSouthDistrict),
                "Prachatice District" => Ok(Self::PrachaticeDistrict),
                "Prague" => Ok(Self::Prague),
                "Prague 1" => Ok(Self::Prague1),
                "Prague 10" => Ok(Self::Prague10),
                "Prague 11" => Ok(Self::Prague11),
                "Prague 12" => Ok(Self::Prague12),
                "Prague 13" => Ok(Self::Prague13),
                "Prague 14" => Ok(Self::Prague14),
                "Prague 15" => Ok(Self::Prague15),
                "Prague 16" => Ok(Self::Prague16),
                "Prague 2" => Ok(Self::Prague2),
                "Prague 21" => Ok(Self::Prague21),
                "Prague 3" => Ok(Self::Prague3),
                "Prague 4" => Ok(Self::Prague4),
                "Prague 5" => Ok(Self::Prague5),
                "Prague 6" => Ok(Self::Prague6),
                "Prague 7" => Ok(Self::Prague7),
                "Prague 8" => Ok(Self::Prague8),
                "Prague 9" => Ok(Self::Prague9),
                "Prague-East District" => Ok(Self::PragueEastDistrict),
                "Prague-West District" => Ok(Self::PragueWestDistrict),
                "Prostějov District" => Ok(Self::ProstejovDistrict),
                "Písek District" => Ok(Self::PisekDistrict),
                "Přerov District" => Ok(Self::PrerovDistrict),
                "Příbram District" => Ok(Self::PribramDistrict),
                "Rakovník District" => Ok(Self::RakovnikDistrict),
                "Rokycany District" => Ok(Self::RokycanyDistrict),
                "Rychnov nad Kněžnou District" => Ok(Self::RychnovNadKneznouDistrict),
                "Semily District" => Ok(Self::SemilyDistrict),
                "Sokolov District" => Ok(Self::SokolovDistrict),
                "South Bohemian Region" => Ok(Self::SouthBohemianRegion),
                "South Moravian Region" => Ok(Self::SouthMoravianRegion),
                "Strakonice District" => Ok(Self::StrakoniceDistrict),
                "Svitavy District" => Ok(Self::SvitavyDistrict),
                "Tachov District" => Ok(Self::TachovDistrict),
                "Teplice District" => Ok(Self::TepliceDistrict),
                "Trutnov District" => Ok(Self::TrutnovDistrict),
                "Tábor District" => Ok(Self::TaborDistrict),
                "Třebíč District" => Ok(Self::TrebicDistrict),
                "Uherské Hradiště District" => Ok(Self::UherskeHradisteDistrict),
                "Vsetín District" => Ok(Self::VsetinDistrict),
                "Vysočina Region" => Ok(Self::VysocinaRegion),
                "Vyškov District" => Ok(Self::VyskovDistrict),
                "Zlín District" => Ok(Self::ZlinDistrict),
                "Zlín Region" => Ok(Self::ZlinRegion),
                "Znojmo District" => Ok(Self::ZnojmoDistrict),
                "Ústí nad Labem District" => Ok(Self::UstiNadLabemDistrict),
                "Ústí nad Labem Region" => Ok(Self::UstiNadLabemRegion),
                "Ústí nad Orlicí District" => Ok(Self::UstiNadOrliciDistrict),
                "Česká Lípa District" => Ok(Self::CeskaLipaDistrict),
                "České Budějovice District" => Ok(Self::CeskeBudejoviceDistrict),
                "Český Krumlov District" => Ok(Self::CeskyKrumlovDistrict),
                "Šumperk District" => Ok(Self::SumperkDistrict),
                "Žďár nad Sázavou District" => Ok(Self::ZdarNadSazavouDistrict),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for CroatiaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "CroatiaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Bjelovar-Bilogora County" => Ok(Self::BjelovarBilogoraCounty),
                "Brod-Posavina County" => Ok(Self::BrodPosavinaCounty),
                "Dubrovnik-Neretva County" => Ok(Self::DubrovnikNeretvaCounty),
                "Istria County" => Ok(Self::IstriaCounty),
                "Koprivnica-Križevci County" => Ok(Self::KoprivnicaKrizevciCounty),
                "Krapina-Zagorje County" => Ok(Self::KrapinaZagorjeCounty),
                "Lika-Senj County" => Ok(Self::LikaSenjCounty),
                "Međimurje County" => Ok(Self::MedimurjeCounty),
                "Osijek-Baranja County" => Ok(Self::OsijekBaranjaCounty),
                "Požega-Slavonia County" => Ok(Self::PozegaSlavoniaCounty),
                "Primorje-Gorski Kotar County" => Ok(Self::PrimorjeGorskiKotarCounty),
                "Sisak-Moslavina County" => Ok(Self::SisakMoslavinaCounty),
                "Split-Dalmatia County" => Ok(Self::SplitDalmatiaCounty),
                "Varaždin County" => Ok(Self::VarazdinCounty),
                "Virovitica-Podravina County" => Ok(Self::ViroviticaPodravinaCounty),
                "Vukovar-Syrmia County" => Ok(Self::VukovarSyrmiaCounty),
                "Zadar County" => Ok(Self::ZadarCounty),
                "Zagreb" => Ok(Self::Zagreb),
                "Zagreb County" => Ok(Self::ZagrebCounty),
                "Šibenik-Knin County" => Ok(Self::SibenikKninCounty),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for BulgariaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "BulgariaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Blagoevgrad Province" => Ok(Self::BlagoevgradProvince),
                "Burgas Province" => Ok(Self::BurgasProvince),
                "Dobrich Province" => Ok(Self::DobrichProvince),
                "Gabrovo Province" => Ok(Self::GabrovoProvince),
                "Haskovo Province" => Ok(Self::HaskovoProvince),
                "Kardzhali Province" => Ok(Self::KardzhaliProvince),
                "Kyustendil Province" => Ok(Self::KyustendilProvince),
                "Lovech Province" => Ok(Self::LovechProvince),
                "Montana Province" => Ok(Self::MontanaProvince),
                "Pazardzhik Province" => Ok(Self::PazardzhikProvince),
                "Pernik Province" => Ok(Self::PernikProvince),
                "Pleven Province" => Ok(Self::PlevenProvince),
                "Plovdiv Province" => Ok(Self::PlovdivProvince),
                "Razgrad Province" => Ok(Self::RazgradProvince),
                "Ruse Province" => Ok(Self::RuseProvince),
                "Shumen" => Ok(Self::Shumen),
                "Silistra Province" => Ok(Self::SilistraProvince),
                "Sliven Province" => Ok(Self::SlivenProvince),
                "Smolyan Province" => Ok(Self::SmolyanProvince),
                "Sofia City Province" => Ok(Self::SofiaCityProvince),
                "Sofia Province" => Ok(Self::SofiaProvince),
                "Stara Zagora Province" => Ok(Self::StaraZagoraProvince),
                "Targovishte Provinc" => Ok(Self::TargovishteProvince),
                "Varna Province" => Ok(Self::VarnaProvince),
                "Veliko Tarnovo Province" => Ok(Self::VelikoTarnovoProvince),
                "Vidin Province" => Ok(Self::VidinProvince),
                "Vratsa Province" => Ok(Self::VratsaProvince),
                "Yambol Province" => Ok(Self::YambolProvince),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for BosniaAndHerzegovinaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "BosniaAndHerzegovinaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Bosnian Podrinje Canton" => Ok(Self::BosnianPodrinjeCanton),
                "Brčko District" => Ok(Self::BrckoDistrict),
                "Canton 10" => Ok(Self::Canton10),
                "Central Bosnia Canton" => Ok(Self::CentralBosniaCanton),
                "Federation of Bosnia and Herzegovina" => {
                    Ok(Self::FederationOfBosniaAndHerzegovina)
                }
                "Herzegovina-Neretva Canton" => Ok(Self::HerzegovinaNeretvaCanton),
                "Posavina Canton" => Ok(Self::PosavinaCanton),
                "Republika Srpska" => Ok(Self::RepublikaSrpska),
                "Sarajevo Canton" => Ok(Self::SarajevoCanton),
                "Tuzla Canton" => Ok(Self::TuzlaCanton),
                "Una-Sana Canton" => Ok(Self::UnaSanaCanton),
                "West Herzegovina Canton" => Ok(Self::WestHerzegovinaCanton),
                "Zenica-Doboj Canton" => Ok(Self::ZenicaDobojCanton),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for UnitedKingdomStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "UnitedKingdomStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Aberdeen" => Ok(Self::Aberdeen),
                "Aberdeenshire" => Ok(Self::Aberdeenshire),
                "Angus" => Ok(Self::Angus),
                "Antrim" => Ok(Self::Antrim),
                "Antrim and Newtownabbey" => Ok(Self::AntrimAndNewtownabbey),
                "Ards" => Ok(Self::Ards),
                "Ards and North Down" => Ok(Self::ArdsAndNorthDown),
                "Argyll and Bute" => Ok(Self::ArgyllAndBute),
                "Armagh City and District Council" => Ok(Self::ArmaghCityAndDistrictCouncil),
                "Armagh, Banbridge and Craigavon" => Ok(Self::ArmaghBanbridgeAndCraigavon),
                "Ascension Island" => Ok(Self::AscensionIsland),
                "Ballymena Borough" => Ok(Self::BallymenaBorough),
                "Ballymoney" => Ok(Self::Ballymoney),
                "Banbridge" => Ok(Self::Banbridge),
                "Barnsley" => Ok(Self::Barnsley),
                "Bath and North East Somerset" => Ok(Self::BathAndNorthEastSomerset),
                "Bedford" => Ok(Self::Bedford),
                "Belfast district" => Ok(Self::BelfastDistrict),
                "Birmingham" => Ok(Self::Birmingham),
                "Blackburn with Darwen" => Ok(Self::BlackburnWithDarwen),
                "Blackpool" => Ok(Self::Blackpool),
                "Blaenau Gwent County Borough" => Ok(Self::BlaenauGwentCountyBorough),
                "Bolton" => Ok(Self::Bolton),
                "Bournemouth" => Ok(Self::Bournemouth),
                "Bracknell Forest" => Ok(Self::BracknellForest),
                "Bradford" => Ok(Self::Bradford),
                "Bridgend County Borough" => Ok(Self::BridgendCountyBorough),
                "Brighton and Hove" => Ok(Self::BrightonAndHove),
                "Buckinghamshire" => Ok(Self::Buckinghamshire),
                "Bury" => Ok(Self::Bury),
                "Caerphilly County Borough" => Ok(Self::CaerphillyCountyBorough),
                "Calderdale" => Ok(Self::Calderdale),
                "Cambridgeshire" => Ok(Self::Cambridgeshire),
                "Carmarthenshire" => Ok(Self::Carmarthenshire),
                "Carrickfergus Borough Council" => Ok(Self::CarrickfergusBoroughCouncil),
                "Castlereagh" => Ok(Self::Castlereagh),
                "Causeway Coast and Glens" => Ok(Self::CausewayCoastAndGlens),
                "Central Bedfordshire" => Ok(Self::CentralBedfordshire),
                "Ceredigion" => Ok(Self::Ceredigion),
                "Cheshire East" => Ok(Self::CheshireEast),
                "Cheshire West and Chester" => Ok(Self::CheshireWestAndChester),
                "City and County of Cardiff" => Ok(Self::CityAndCountyOfCardiff),
                "City and County of Swansea" => Ok(Self::CityAndCountyOfSwansea),
                "City of Bristol" => Ok(Self::CityOfBristol),
                "City of Derby" => Ok(Self::CityOfDerby),
                "City of Kingston upon Hull" => Ok(Self::CityOfKingstonUponHull),
                "City of Leicester" => Ok(Self::CityOfLeicester),
                "City of London" => Ok(Self::CityOfLondon),
                "City of Nottingham" => Ok(Self::CityOfNottingham),
                "City of Peterborough" => Ok(Self::CityOfPeterborough),
                "City of Plymouth" => Ok(Self::CityOfPlymouth),
                "City of Portsmouth" => Ok(Self::CityOfPortsmouth),
                "City of Southampton" => Ok(Self::CityOfSouthampton),
                "City of Stoke-on-Trent" => Ok(Self::CityOfStokeOnTrent),
                "City of Sunderland" => Ok(Self::CityOfSunderland),
                "City of Westminster" => Ok(Self::CityOfWestminster),
                "City of Wolverhampton" => Ok(Self::CityOfWolverhampton),
                "City of York" => Ok(Self::CityOfYork),
                "Clackmannanshire" => Ok(Self::Clackmannanshire),
                "Coleraine Borough Council" => Ok(Self::ColeraineBoroughCouncil),
                "Conwy County Borough" => Ok(Self::ConwyCountyBorough),
                "Cookstown District Council" => Ok(Self::CookstownDistrictCouncil),
                "Cornwall" => Ok(Self::Cornwall),
                "County Durham" => Ok(Self::CountyDurham),
                "Coventry" => Ok(Self::Coventry),
                "Craigavon Borough Council" => Ok(Self::CraigavonBoroughCouncil),
                "Cumbria" => Ok(Self::Cumbria),
                "Darlington" => Ok(Self::Darlington),
                "Denbighshire" => Ok(Self::Denbighshire),
                "Derbyshire" => Ok(Self::Derbyshire),
                "Derry City and Strabane" => Ok(Self::DerryCityAndStrabane),
                "Derry City Council" => Ok(Self::DerryCityCouncil),
                "Devon" => Ok(Self::Devon),
                "Doncaster" => Ok(Self::Doncaster),
                "Dorset" => Ok(Self::Dorset),
                "Down District Council" => Ok(Self::DownDistrictCouncil),
                "Dudley" => Ok(Self::Dudley),
                "Dumfries and Galloway" => Ok(Self::DumfriesAndGalloway),
                "Dundee" => Ok(Self::Dundee),
                "Dungannon and South Tyrone Borough Council" => {
                    Ok(Self::DungannonAndSouthTyroneBoroughCouncil)
                }
                "East Ayrshire" => Ok(Self::EastAyrshire),
                "East Dunbartonshire" => Ok(Self::EastDunbartonshire),
                "East Lothian" => Ok(Self::EastLothian),
                "East Renfrewshire" => Ok(Self::EastRenfrewshire),
                "East Riding of Yorkshire" => Ok(Self::EastRidingOfYorkshire),
                "East Sussex" => Ok(Self::EastSussex),
                "Edinburgh" => Ok(Self::Edinburgh),
                "England" => Ok(Self::England),
                "Essex" => Ok(Self::Essex),
                "Falkirk" => Ok(Self::Falkirk),
                "Fermanagh and Omagh" => Ok(Self::FermanaghAndOmagh),
                "Fermanagh District Council" => Ok(Self::FermanaghDistrictCouncil),
                "Fife" => Ok(Self::Fife),
                "Flintshire" => Ok(Self::Flintshire),
                "Gateshead" => Ok(Self::Gateshead),
                "Glasgow" => Ok(Self::Glasgow),
                "Gloucestershire" => Ok(Self::Gloucestershire),
                "Gwynedd" => Ok(Self::Gwynedd),
                "Halton" => Ok(Self::Halton),
                "Hampshire" => Ok(Self::Hampshire),
                "Hartlepool" => Ok(Self::Hartlepool),
                "Herefordshire" => Ok(Self::Herefordshire),
                "Hertfordshire" => Ok(Self::Hertfordshire),
                "Highland" => Ok(Self::Highland),
                "Inverclyde" => Ok(Self::Inverclyde),
                "Isle of Wight" => Ok(Self::IsleOfWight),
                "Isles of Scilly" => Ok(Self::IslesOfScilly),
                "Kent" => Ok(Self::Kent),
                "Kirklees" => Ok(Self::Kirklees),
                "Knowsley" => Ok(Self::Knowsley),
                "Lancashire" => Ok(Self::Lancashire),
                "Larne Borough Council" => Ok(Self::LarneBoroughCouncil),
                "Leeds" => Ok(Self::Leeds),
                "Leicestershire" => Ok(Self::Leicestershire),
                "Limavady Borough Council" => Ok(Self::LimavadyBoroughCouncil),
                "Lincolnshire" => Ok(Self::Lincolnshire),
                "Lisburn and Castlereagh" => Ok(Self::LisburnAndCastlereagh),
                "Lisburn City Council" => Ok(Self::LisburnCityCouncil),
                "Liverpool" => Ok(Self::Liverpool),
                "London Borough of Barking and Dagenham" => {
                    Ok(Self::LondonBoroughOfBarkingAndDagenham)
                }
                "London Borough of Barnet" => Ok(Self::LondonBoroughOfBarnet),
                "London Borough of Bexley" => Ok(Self::LondonBoroughOfBexley),
                "London Borough of Brent" => Ok(Self::LondonBoroughOfBrent),
                "London Borough of Bromley" => Ok(Self::LondonBoroughOfBromley),
                "London Borough of Camden" => Ok(Self::LondonBoroughOfCamden),
                "London Borough of Croydon" => Ok(Self::LondonBoroughOfCroydon),
                "London Borough of Ealing" => Ok(Self::LondonBoroughOfEaling),
                "London Borough of Enfield" => Ok(Self::LondonBoroughOfEnfield),
                "London Borough of Hackney" => Ok(Self::LondonBoroughOfHackney),
                "London Borough of Hammersmith and Fulham" => {
                    Ok(Self::LondonBoroughOfHammersmithAndFulham)
                }
                "London Borough of Haringey" => Ok(Self::LondonBoroughOfHaringey),
                "London Borough of Harrow" => Ok(Self::LondonBoroughOfHarrow),
                "London Borough of Havering" => Ok(Self::LondonBoroughOfHavering),
                "London Borough of Hillingdon" => Ok(Self::LondonBoroughOfHillingdon),
                "London Borough of Hounslow" => Ok(Self::LondonBoroughOfHounslow),
                "London Borough of Islington" => Ok(Self::LondonBoroughOfIslington),
                "London Borough of Lambeth" => Ok(Self::LondonBoroughOfLambeth),
                "London Borough of Lewisham" => Ok(Self::LondonBoroughOfLewisham),
                "London Borough of Merton" => Ok(Self::LondonBoroughOfMerton),
                "London Borough of Newham" => Ok(Self::LondonBoroughOfNewham),
                "London Borough of Redbridge" => Ok(Self::LondonBoroughOfRedbridge),
                "London Borough of Richmond upon Thames" => {
                    Ok(Self::LondonBoroughOfRichmondUponThames)
                }
                "London Borough of Southwark" => Ok(Self::LondonBoroughOfSouthwark),
                "London Borough of Sutton" => Ok(Self::LondonBoroughOfSutton),
                "London Borough of Tower Hamlets" => Ok(Self::LondonBoroughOfTowerHamlets),
                "London Borough of Waltham Forest" => Ok(Self::LondonBoroughOfWalthamForest),
                "London Borough of Wandsworth" => Ok(Self::LondonBoroughOfWandsworth),
                "Magherafelt District Council" => Ok(Self::MagherafeltDistrictCouncil),
                "Manchester" => Ok(Self::Manchester),
                "Medway" => Ok(Self::Medway),
                "Merthyr Tydfil County Borough" => Ok(Self::MerthyrTydfilCountyBorough),
                "Metropolitan Borough of Wigan" => Ok(Self::MetropolitanBoroughOfWigan),
                "Mid and East Antrim" => Ok(Self::MidAndEastAntrim),
                "Mid Ulster" => Ok(Self::MidUlster),
                "Middlesbrough" => Ok(Self::Middlesbrough),
                "Midlothian" => Ok(Self::Midlothian),
                "Milton Keynes" => Ok(Self::MiltonKeynes),
                "Monmouthshire" => Ok(Self::Monmouthshire),
                "Moray" => Ok(Self::Moray),
                "Moyle District Council" => Ok(Self::MoyleDistrictCouncil),
                "Neath Port Talbot County Borough" => Ok(Self::NeathPortTalbotCountyBorough),
                "Newcastle upon Tyne" => Ok(Self::NewcastleUponTyne),
                "Newport" => Ok(Self::Newport),
                "Newry and Mourne District Council" => Ok(Self::NewryAndMourneDistrictCouncil),
                "Newry, Mourne and Down" => Ok(Self::NewryMourneAndDown),
                "Newtownabbey Borough Council" => Ok(Self::NewtownabbeyBoroughCouncil),
                "Norfolk" => Ok(Self::Norfolk),
                "North Ayrshire" => Ok(Self::NorthAyrshire),
                "North Down Borough Council" => Ok(Self::NorthDownBoroughCouncil),
                "North East Lincolnshire" => Ok(Self::NorthEastLincolnshire),
                "North Lanarkshire" => Ok(Self::NorthLanarkshire),
                "North Lincolnshire" => Ok(Self::NorthLincolnshire),
                "North Somerset" => Ok(Self::NorthSomerset),
                "North Tyneside" => Ok(Self::NorthTyneside),
                "North Yorkshire" => Ok(Self::NorthYorkshire),
                "Northamptonshire" => Ok(Self::Northamptonshire),
                "Northern Ireland" => Ok(Self::NorthernIreland),
                "Northumberland" => Ok(Self::Northumberland),
                "Nottinghamshire" => Ok(Self::Nottinghamshire),
                "Oldham" => Ok(Self::Oldham),
                "Omagh District Council" => Ok(Self::OmaghDistrictCouncil),
                "Orkney Islands" => Ok(Self::OrkneyIslands),
                "Outer Hebrides" => Ok(Self::OuterHebrides),
                "Oxfordshire" => Ok(Self::Oxfordshire),
                "Pembrokeshire" => Ok(Self::Pembrokeshire),
                "Perth and Kinross" => Ok(Self::PerthAndKinross),
                "Poole" => Ok(Self::Poole),
                "Powys" => Ok(Self::Powys),
                "Reading" => Ok(Self::Reading),
                "Redcar and Cleveland" => Ok(Self::RedcarAndCleveland),
                "Renfrewshire" => Ok(Self::Renfrewshire),
                "Rhondda Cynon Taf" => Ok(Self::RhonddaCynonTaf),
                "Rochdale" => Ok(Self::Rochdale),
                "Rotherham" => Ok(Self::Rotherham),
                "Royal Borough of Greenwich" => Ok(Self::RoyalBoroughOfGreenwich),
                "Royal Borough of Kensington and Chelsea" => {
                    Ok(Self::RoyalBoroughOfKensingtonAndChelsea)
                }
                "Royal Borough of Kingston upon Thames" => {
                    Ok(Self::RoyalBoroughOfKingstonUponThames)
                }
                "Rutland" => Ok(Self::Rutland),
                "Saint Helena" => Ok(Self::SaintHelena),
                "Salford" => Ok(Self::Salford),
                "Sandwell" => Ok(Self::Sandwell),
                "Scotland" => Ok(Self::Scotland),
                "Scottish Borders" => Ok(Self::ScottishBorders),
                "Sefton" => Ok(Self::Sefton),
                "Sheffield" => Ok(Self::Sheffield),
                "Shetland Islands" => Ok(Self::ShetlandIslands),
                "Shropshire" => Ok(Self::Shropshire),
                "Slough" => Ok(Self::Slough),
                "Solihull" => Ok(Self::Solihull),
                "Somerset" => Ok(Self::Somerset),
                "South Ayrshire" => Ok(Self::SouthAyrshire),
                "South Gloucestershire" => Ok(Self::SouthGloucestershire),
                "South Lanarkshire" => Ok(Self::SouthLanarkshire),
                "South Tyneside" => Ok(Self::SouthTyneside),
                "Southend-on-Sea" => Ok(Self::SouthendOnSea),
                "St Helens" => Ok(Self::StHelens),
                "Staffordshire" => Ok(Self::Staffordshire),
                "Stirling" => Ok(Self::Stirling),
                "Stockport" => Ok(Self::Stockport),
                "Stockton-on-Tees" => Ok(Self::StocktonOnTees),
                "Strabane District Council" => Ok(Self::StrabaneDistrictCouncil),
                "Suffolk" => Ok(Self::Suffolk),
                "Surrey" => Ok(Self::Surrey),
                "Swindon" => Ok(Self::Swindon),
                "Tameside" => Ok(Self::Tameside),
                "Telford and Wrekin" => Ok(Self::TelfordAndWrekin),
                "Thurrock" => Ok(Self::Thurrock),
                "Torbay" => Ok(Self::Torbay),
                "Torfaen" => Ok(Self::Torfaen),
                "Trafford" => Ok(Self::Trafford),
                "United Kingdom" => Ok(Self::UnitedKingdom),
                "Vale of Glamorgan" => Ok(Self::ValeOfGlamorgan),
                "Wakefield" => Ok(Self::Wakefield),
                "Wales" => Ok(Self::Wales),
                "Walsall" => Ok(Self::Walsall),
                "Warrington" => Ok(Self::Warrington),
                "Warwickshire" => Ok(Self::Warwickshire),
                "West Berkshire" => Ok(Self::WestBerkshire),
                "West Dunbartonshire" => Ok(Self::WestDunbartonshire),
                "West Lothian" => Ok(Self::WestLothian),
                "West Sussex" => Ok(Self::WestSussex),
                "Wiltshire" => Ok(Self::Wiltshire),
                "Windsor and Maidenhead" => Ok(Self::WindsorAndMaidenhead),
                "Wirral" => Ok(Self::Wirral),
                "Wokingham" => Ok(Self::Wokingham),
                "Worcestershire" => Ok(Self::Worcestershire),
                "Wrexham County Borough" => Ok(Self::WrexhamCountyBorough),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for BelgiumStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "BelgiumStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Antwerp" => Ok(Self::Antwerp),
                "Brussels-Capital Region" => Ok(Self::BrusselsCapitalRegion),
                "East Flanders" => Ok(Self::EastFlanders),
                "Flanders" => Ok(Self::Flanders),
                "Flemish Brabant" => Ok(Self::FlemishBrabant),
                "Hainaut" => Ok(Self::Hainaut),
                "Limburg" => Ok(Self::Limburg),
                "Liège" => Ok(Self::Liege),
                "Luxembourg" => Ok(Self::Luxembourg),
                "Namur" => Ok(Self::Namur),
                "Wallonia" => Ok(Self::Wallonia),
                "Walloon Brabant" => Ok(Self::WalloonBrabant),
                "West Flanders" => Ok(Self::WestFlanders),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for LuxembourgStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "LuxembourgStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Canton of Capellen" => Ok(Self::CantonOfCapellen),
                "Canton of Clervaux" => Ok(Self::CantonOfClervaux),
                "Canton of Diekirch" => Ok(Self::CantonOfDiekirch),
                "Canton of Echternach" => Ok(Self::CantonOfEchternach),
                "Canton of Esch-sur-Alzette" => Ok(Self::CantonOfEschSurAlzette),
                "Canton of Grevenmacher" => Ok(Self::CantonOfGrevenmacher),
                "Canton of Luxembourg" => Ok(Self::CantonOfLuxembourg),
                "Canton of Mersch" => Ok(Self::CantonOfMersch),
                "Canton of Redange" => Ok(Self::CantonOfRedange),
                "Canton of Remich" => Ok(Self::CantonOfRemich),
                "Canton of Vianden" => Ok(Self::CantonOfVianden),
                "Canton of Wiltz" => Ok(Self::CantonOfWiltz),
                "Diekirch District" => Ok(Self::DiekirchDistrict),
                "Grevenmacher District" => Ok(Self::GrevenmacherDistrict),
                "Luxembourg District" => Ok(Self::LuxembourgDistrict),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for RussiaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "RussiaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Altai Krai" => Ok(Self::AltaiKrai),
                "Altai Republic" => Ok(Self::AltaiRepublic),
                "Amur Oblast" => Ok(Self::AmurOblast),
                "Arkhangelsk" => Ok(Self::Arkhangelsk),
                "Astrakhan Oblast" => Ok(Self::AstrakhanOblast),
                "Belgorod Oblast" => Ok(Self::BelgorodOblast),
                "Bryansk Oblast" => Ok(Self::BryanskOblast),
                "Chechen Republic" => Ok(Self::ChechenRepublic),
                "Chelyabinsk Oblast" => Ok(Self::ChelyabinskOblast),
                "Chukotka Autonomous Okrug" => Ok(Self::ChukotkaAutonomousOkrug),
                "Chuvash Republic" => Ok(Self::ChuvashRepublic),
                "Irkutsk" => Ok(Self::Irkutsk),
                "Ivanovo Oblast" => Ok(Self::IvanovoOblast),
                "Jewish Autonomous Oblast" => Ok(Self::JewishAutonomousOblast),
                "Kabardino-Balkar Republic" => Ok(Self::KabardinoBalkarRepublic),
                "Kaliningrad" => Ok(Self::Kaliningrad),
                "Kaluga Oblast" => Ok(Self::KalugaOblast),
                "Kamchatka Krai" => Ok(Self::KamchatkaKrai),
                "Karachay-Cherkess Republic" => Ok(Self::KarachayCherkessRepublic),
                "Kemerovo Oblast" => Ok(Self::KemerovoOblast),
                "Khabarovsk Krai" => Ok(Self::KhabarovskKrai),
                "Khanty-Mansi Autonomous Okrug" => Ok(Self::KhantyMansiAutonomousOkrug),
                "Kirov Oblast" => Ok(Self::KirovOblast),
                "Komi Republic" => Ok(Self::KomiRepublic),
                "Kostroma Oblast" => Ok(Self::KostromaOblast),
                "Krasnodar Krai" => Ok(Self::KrasnodarKrai),
                "Krasnoyarsk Krai" => Ok(Self::KrasnoyarskKrai),
                "Kurgan Oblast" => Ok(Self::KurganOblast),
                "Kursk Oblast" => Ok(Self::KurskOblast),
                "Leningrad Oblast" => Ok(Self::LeningradOblast),
                "Lipetsk Oblast" => Ok(Self::LipetskOblast),
                "Magadan Oblast" => Ok(Self::MagadanOblast),
                "Mari El Republic" => Ok(Self::MariElRepublic),
                "Moscow" => Ok(Self::Moscow),
                "Moscow Oblast" => Ok(Self::MoscowOblast),
                "Murmansk Oblast" => Ok(Self::MurmanskOblast),
                "Nenets Autonomous Okrug" => Ok(Self::NenetsAutonomousOkrug),
                "Nizhny Novgorod Oblast" => Ok(Self::NizhnyNovgorodOblast),
                "Novgorod Oblast" => Ok(Self::NovgorodOblast),
                "Novosibirsk" => Ok(Self::Novosibirsk),
                "Omsk Oblast" => Ok(Self::OmskOblast),
                "Orenburg Oblast" => Ok(Self::OrenburgOblast),
                "Oryol Oblast" => Ok(Self::OryolOblast),
                "Penza Oblast" => Ok(Self::PenzaOblast),
                "Perm Krai" => Ok(Self::PermKrai),
                "Primorsky Krai" => Ok(Self::PrimorskyKrai),
                "Pskov Oblast" => Ok(Self::PskovOblast),
                "Republic of Adygea" => Ok(Self::RepublicOfAdygea),
                "Republic of Bashkortostan" => Ok(Self::RepublicOfBashkortostan),
                "Republic of Buryatia" => Ok(Self::RepublicOfBuryatia),
                "Republic of Dagestan" => Ok(Self::RepublicOfDagestan),
                "Republic of Ingushetia" => Ok(Self::RepublicOfIngushetia),
                "Republic of Kalmykia" => Ok(Self::RepublicOfKalmykia),
                "Republic of Karelia" => Ok(Self::RepublicOfKarelia),
                "Republic of Khakassia" => Ok(Self::RepublicOfKhakassia),
                "Republic of Mordovia" => Ok(Self::RepublicOfMordovia),
                "Republic of North Ossetia-Alania" => Ok(Self::RepublicOfNorthOssetiaAlania),
                "Republic of Tatarstan" => Ok(Self::RepublicOfTatarstan),
                "Rostov Oblast" => Ok(Self::RostovOblast),
                "Ryazan Oblast" => Ok(Self::RyazanOblast),
                "Saint Petersburg" => Ok(Self::SaintPetersburg),
                "Sakha Republic" => Ok(Self::SakhaRepublic),
                "Sakhalin" => Ok(Self::Sakhalin),
                "Samara Oblast" => Ok(Self::SamaraOblast),
                "Saratov Oblast" => Ok(Self::SaratovOblast),
                "Sevastopol" => Ok(Self::Sevastopol),
                "Smolensk Oblast" => Ok(Self::SmolenskOblast),
                "Stavropol Krai" => Ok(Self::StavropolKrai),
                "Sverdlovsk" => Ok(Self::Sverdlovsk),
                "Tambov Oblast" => Ok(Self::TambovOblast),
                "Tomsk Oblast" => Ok(Self::TomskOblast),
                "Tula Oblast" => Ok(Self::TulaOblast),
                "Tuva Republic" => Ok(Self::TuvaRepublic),
                "Tver Oblast" => Ok(Self::TverOblast),
                "Tyumen Oblast" => Ok(Self::TyumenOblast),
                "Udmurt Republic" => Ok(Self::UdmurtRepublic),
                "Ulyanovsk Oblast" => Ok(Self::UlyanovskOblast),
                "Vladimir Oblast" => Ok(Self::VladimirOblast),
                "Vologda Oblast" => Ok(Self::VologdaOblast),
                "Voronezh Oblast" => Ok(Self::VoronezhOblast),
                "Yamalo-Nenets Autonomous Okrug" => Ok(Self::YamaloNenetsAutonomousOkrug),
                "Yaroslavl Oblast" => Ok(Self::YaroslavlOblast),
                "Zabaykalsky Krai" => Ok(Self::ZabaykalskyKrai),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for SanMarinoStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "SanMarinoStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Acquaviva" => Ok(Self::Acquaviva),
                "Borgo Maggiore" => Ok(Self::BorgoMaggiore),
                "Chiesanuova" => Ok(Self::Chiesanuova),
                "Domagnano" => Ok(Self::Domagnano),
                "Faetano" => Ok(Self::Faetano),
                "Fiorentino" => Ok(Self::Fiorentino),
                "Montegiardino" => Ok(Self::Montegiardino),
                "San Marino" => Ok(Self::SanMarino),
                "Serravalle" => Ok(Self::Serravalle),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for SerbiaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "SerbiaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Belgrade" => Ok(Self::Belgrade),
                "Bor District" => Ok(Self::BorDistrict),
                "Braničevo District" => Ok(Self::BraničevoDistrict),
                "Central Banat District" => Ok(Self::CentralBanatDistrict),
                "Jablanica District" => Ok(Self::JablanicaDistrict),
                "Kolubara District" => Ok(Self::KolubaraDistrict),
                "Mačva District" => Ok(Self::MačvaDistrict),
                "Moravica District" => Ok(Self::MoravicaDistrict),
                "Nišava District" => Ok(Self::NišavaDistrict),
                "North Banat District" => Ok(Self::NorthBanatDistrict),
                "North Bačka District" => Ok(Self::NorthBačkaDistrict),
                "Pirot District" => Ok(Self::PirotDistrict),
                "Podunavlje District" => Ok(Self::PodunavljeDistrict),
                "Pomoravlje District" => Ok(Self::PomoravljeDistrict),
                "Pčinja District" => Ok(Self::PčinjaDistrict),
                "Rasina District" => Ok(Self::RasinaDistrict),
                "Raška District" => Ok(Self::RaškaDistrict),
                "South Banat District" => Ok(Self::SouthBanatDistrict),
                "South Bačka District" => Ok(Self::SouthBačkaDistrict),
                "Srem District" => Ok(Self::SremDistrict),
                "Toplica District" => Ok(Self::ToplicaDistrict),
                "Vojvodina" => Ok(Self::Vojvodina),
                "West Bačka District" => Ok(Self::WestBačkaDistrict),
                "Zaječar District" => Ok(Self::ZaječarDistrict),
                "Zlatibor District" => Ok(Self::ZlatiborDistrict),
                "Šumadija District" => Ok(Self::ŠumadijaDistrict),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for SlovakiaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "SlovakiaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Banská Bystrica Region" => Ok(Self::BanskaBystricaRegion),
                "Bratislava Region" => Ok(Self::BratislavaRegion),
                "Košice Region" => Ok(Self::KosiceRegion),
                "Nitra Region" => Ok(Self::NitraRegion),
                "Prešov Region" => Ok(Self::PresovRegion),
                "Trenčín Region" => Ok(Self::TrencinRegion),
                "Trnava Region" => Ok(Self::TrnavaRegion),
                "Žilina Region" => Ok(Self::ZilinaRegion),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for SwedenStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "SwedenStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Blekinge" => Ok(Self::Blekinge),
                "Dalarna County" => Ok(Self::DalarnaCounty),
                "Gotland County" => Ok(Self::GotlandCounty),
                "Gävleborg County" => Ok(Self::GävleborgCounty),
                "Halland County" => Ok(Self::HallandCounty),
                "Jönköping County" => Ok(Self::JönköpingCounty),
                "Kalmar County" => Ok(Self::KalmarCounty),
                "Kronoberg County" => Ok(Self::KronobergCounty),
                "Norrbotten County" => Ok(Self::NorrbottenCounty),
                "Skåne County" => Ok(Self::SkåneCounty),
                "Stockholm County" => Ok(Self::StockholmCounty),
                "Södermanland County" => Ok(Self::SödermanlandCounty),
                "Uppsala County" => Ok(Self::UppsalaCounty),
                "Värmland County" => Ok(Self::VärmlandCounty),
                "Västerbotten County" => Ok(Self::VästerbottenCounty),
                "Västernorrland County" => Ok(Self::VästernorrlandCounty),
                "Västmanland County" => Ok(Self::VästmanlandCounty),
                "Västra Götaland County" => Ok(Self::VästraGötalandCounty),
                "Örebro County" => Ok(Self::ÖrebroCounty),
                "Östergötland County" => Ok(Self::ÖstergötlandCounty),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for SloveniaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "SloveniaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Ajdovščina Municipality" => Ok(Self::Ajdovščina),
                "Ankaran Municipality" => Ok(Self::Ankaran),
                "Beltinci Municipality" => Ok(Self::Beltinci),
                "Benedikt Municipality" => Ok(Self::Benedikt),
                "Bistrica ob Sotli Municipality" => Ok(Self::BistricaObSotli),
                "Bled Municipality" => Ok(Self::Bled),
                "Bloke Municipality" => Ok(Self::Bloke),
                "Bohinj Municipality" => Ok(Self::Bohinj),
                "Borovnica Municipality" => Ok(Self::Borovnica),
                "Bovec Municipality" => Ok(Self::Bovec),
                "Braslovče Municipality" => Ok(Self::Braslovče),
                "Brda Municipality" => Ok(Self::Brda),
                "Brezovica Municipality" => Ok(Self::Brezovica),
                "Brežice Municipality" => Ok(Self::Brežice),
                "Cankova Municipality" => Ok(Self::Cankova),
                "Cerklje na Gorenjskem Municipality" => Ok(Self::CerkljeNaGorenjskem),
                "Cerknica Municipality" => Ok(Self::Cerknica),
                "Cerkno Municipality" => Ok(Self::Cerkno),
                "Cerkvenjak Municipality" => Ok(Self::Cerkvenjak),
                "City Municipality of Celje" => Ok(Self::CityMunicipalityOfCelje),
                "City Municipality of Novo Mesto" => Ok(Self::CityMunicipalityOfNovoMesto),
                "Destrnik Municipality" => Ok(Self::Destrnik),
                "Divača Municipality" => Ok(Self::Divača),
                "Dobje Municipality" => Ok(Self::Dobje),
                "Dobrepolje Municipality" => Ok(Self::Dobrepolje),
                "Dobrna Municipality" => Ok(Self::Dobrna),
                "Dobrova–Polhov Gradec Municipality" => Ok(Self::DobrovaPolhovGradec),
                "Dobrovnik Municipality" => Ok(Self::Dobrovnik),
                "Dol pri Ljubljani Municipality" => Ok(Self::DolPriLjubljani),
                "Dolenjske Toplice Municipality" => Ok(Self::DolenjskeToplice),
                "Domžale Municipality" => Ok(Self::Domžale),
                "Dornava Municipality" => Ok(Self::Dornava),
                "Dravograd Municipality" => Ok(Self::Dravograd),
                "Duplek Municipality" => Ok(Self::Duplek),
                "Gorenja Vas–Poljane Municipality" => Ok(Self::GorenjaVasPoljane),
                "Gorišnica Municipality" => Ok(Self::Gorišnica),
                "Gorje Municipality" => Ok(Self::Gorje),
                "Gornja Radgona Municipality" => Ok(Self::GornjaRadgona),
                "Gornji Grad Municipality" => Ok(Self::GornjiGrad),
                "Gornji Petrovci Municipality" => Ok(Self::GornjiPetrovci),
                "Grad Municipality" => Ok(Self::Grad),
                "Grosuplje Municipality" => Ok(Self::Grosuplje),
                "Hajdina Municipality" => Ok(Self::Hajdina),
                "Hodoš Municipality" => Ok(Self::Hodoš),
                "Horjul Municipality" => Ok(Self::Horjul),
                "Hoče–Slivnica Municipality" => Ok(Self::HočeSlivnica),
                "Hrastnik Municipality" => Ok(Self::Hrastnik),
                "Hrpelje–Kozina Municipality" => Ok(Self::HrpeljeKozina),
                "Idrija Municipality" => Ok(Self::Idrija),
                "Ig Municipality" => Ok(Self::Ig),
                "Ivančna Gorica Municipality" => Ok(Self::IvančnaGorica),
                "Izola Municipality" => Ok(Self::Izola),
                "Jesenice Municipality" => Ok(Self::Jesenice),
                "Jezersko Municipality" => Ok(Self::Jezersko),
                "Juršinci Municipality" => Ok(Self::Jursinci),
                "Kamnik Municipality" => Ok(Self::Kamnik),
                "Kanal ob Soči Municipality" => Ok(Self::KanalObSoci),
                "Kidričevo Municipality" => Ok(Self::Kidricevo),
                "Kobarid Municipality" => Ok(Self::Kobarid),
                "Kobilje Municipality" => Ok(Self::Kobilje),
                "Komen Municipality" => Ok(Self::Komen),
                "Komenda Municipality" => Ok(Self::Komenda),
                "Koper City Municipality" => Ok(Self::Koper),
                "Kostanjevica na Krki Municipality" => Ok(Self::KostanjevicaNaKrki),
                "Kostel Municipality" => Ok(Self::Kostel),
                "Kozje Municipality" => Ok(Self::Kozje),
                "Kočevje Municipality" => Ok(Self::Kocevje),
                "Kranj City Municipality" => Ok(Self::Kranj),
                "Kranjska Gora Municipality" => Ok(Self::KranjskaGora),
                "Križevci Municipality" => Ok(Self::Krizevci),
                "Kungota" => Ok(Self::Kungota),
                "Kuzma Municipality" => Ok(Self::Kuzma),
                "Laško Municipality" => Ok(Self::Lasko),
                "Lenart Municipality" => Ok(Self::Lenart),
                "Lendava Municipality" => Ok(Self::Lendava),
                "Litija Municipality" => Ok(Self::Litija),
                "Ljubljana City Municipality" => Ok(Self::Ljubljana),
                "Ljubno Municipality" => Ok(Self::Ljubno),
                "Ljutomer Municipality" => Ok(Self::Ljutomer),
                "Logatec Municipality" => Ok(Self::Logatec),
                "Log–Dragomer Municipality" => Ok(Self::LogDragomer),
                "Lovrenc na Pohorju Municipality" => Ok(Self::LovrencNaPohorju),
                "Loška Dolina Municipality" => Ok(Self::LoskaDolina),
                "Loški Potok Municipality" => Ok(Self::LoskiPotok),
                "Lukovica Municipality" => Ok(Self::Lukovica),
                "Luče Municipality" => Ok(Self::Luče),
                "Majšperk Municipality" => Ok(Self::Majsperk),
                "Makole Municipality" => Ok(Self::Makole),
                "Maribor City Municipality" => Ok(Self::Maribor),
                "Markovci Municipality" => Ok(Self::Markovci),
                "Medvode Municipality" => Ok(Self::Medvode),
                "Mengeš Municipality" => Ok(Self::Menges),
                "Metlika Municipality" => Ok(Self::Metlika),
                "Mežica Municipality" => Ok(Self::Mezica),
                "Miklavž na Dravskem Polju Municipality" => Ok(Self::MiklavzNaDravskemPolju),
                "Miren–Kostanjevica Municipality" => Ok(Self::MirenKostanjevica),
                "Mirna Municipality" => Ok(Self::Mirna),
                "Mirna Peč Municipality" => Ok(Self::MirnaPec),
                "Mislinja Municipality" => Ok(Self::Mislinja),
                "Mokronog–Trebelno Municipality" => Ok(Self::MokronogTrebelno),
                "Moravske Toplice Municipality" => Ok(Self::MoravskeToplice),
                "Moravče Municipality" => Ok(Self::Moravce),
                "Mozirje Municipality" => Ok(Self::Mozirje),
                "Municipality of Apače" => Ok(Self::Apače),
                "Municipality of Cirkulane" => Ok(Self::Cirkulane),
                "Municipality of Ilirska Bistrica" => Ok(Self::IlirskaBistrica),
                "Municipality of Krško" => Ok(Self::Krsko),
                "Municipality of Škofljica" => Ok(Self::Skofljica),
                "Murska Sobota City Municipality" => Ok(Self::MurskaSobota),
                "Muta Municipality" => Ok(Self::Muta),
                "Naklo Municipality" => Ok(Self::Naklo),
                "Nazarje Municipality" => Ok(Self::Nazarje),
                "Nova Gorica City Municipality" => Ok(Self::NovaGorica),
                "Odranci Municipality" => Ok(Self::Odranci),
                "Oplotnica" => Ok(Self::Oplotnica),
                "Ormož Municipality" => Ok(Self::Ormoz),
                "Osilnica Municipality" => Ok(Self::Osilnica),
                "Pesnica Municipality" => Ok(Self::Pesnica),
                "Piran Municipality" => Ok(Self::Piran),
                "Pivka Municipality" => Ok(Self::Pivka),
                "Podlehnik Municipality" => Ok(Self::Podlehnik),
                "Podvelka Municipality" => Ok(Self::Podvelka),
                "Podčetrtek Municipality" => Ok(Self::Podcetrtek),
                "Poljčane Municipality" => Ok(Self::Poljcane),
                "Polzela Municipality" => Ok(Self::Polzela),
                "Postojna Municipality" => Ok(Self::Postojna),
                "Prebold Municipality" => Ok(Self::Prebold),
                "Preddvor Municipality" => Ok(Self::Preddvor),
                "Prevalje Municipality" => Ok(Self::Prevalje),
                "Ptuj City Municipality" => Ok(Self::Ptuj),
                "Puconci Municipality" => Ok(Self::Puconci),
                "Radenci Municipality" => Ok(Self::Radenci),
                "Radeče Municipality" => Ok(Self::Radece),
                "Radlje ob Dravi Municipality" => Ok(Self::RadljeObDravi),
                "Radovljica Municipality" => Ok(Self::Radovljica),
                "Ravne na Koroškem Municipality" => Ok(Self::RavneNaKoroskem),
                "Razkrižje Municipality" => Ok(Self::Razkrizje),
                "Rače–Fram Municipality" => Ok(Self::RaceFram),
                "Renče–Vogrsko Municipality" => Ok(Self::RenčeVogrsko),
                "Rečica ob Savinji Municipality" => Ok(Self::RecicaObSavinji),
                "Ribnica Municipality" => Ok(Self::Ribnica),
                "Ribnica na Pohorju Municipality" => Ok(Self::RibnicaNaPohorju),
                "Rogatec Municipality" => Ok(Self::Rogatec),
                "Rogaška Slatina Municipality" => Ok(Self::RogaskaSlatina),
                "Rogašovci Municipality" => Ok(Self::Rogasovci),
                "Ruše Municipality" => Ok(Self::Ruse),
                "Selnica ob Dravi Municipality" => Ok(Self::SelnicaObDravi),
                "Semič Municipality" => Ok(Self::Semic),
                "Sevnica Municipality" => Ok(Self::Sevnica),
                "Sežana Municipality" => Ok(Self::Sezana),
                "Slovenj Gradec City Municipality" => Ok(Self::SlovenjGradec),
                "Slovenska Bistrica Municipality" => Ok(Self::SlovenskaBistrica),
                "Slovenske Konjice Municipality" => Ok(Self::SlovenskeKonjice),
                "Sodražica Municipality" => Ok(Self::Sodrazica),
                "Solčava Municipality" => Ok(Self::Solcava),
                "Središče ob Dravi" => Ok(Self::SredisceObDravi),
                "Starše Municipality" => Ok(Self::Starse),
                "Straža Municipality" => Ok(Self::Straza),
                "Sveta Ana Municipality" => Ok(Self::SvetaAna),
                "Sveta Trojica v Slovenskih Goricah Municipality" => Ok(Self::SvetaTrojica),
                "Sveti Andraž v Slovenskih Goricah Municipality" => Ok(Self::SvetiAndraz),
                "Sveti Jurij ob Ščavnici Municipality" => Ok(Self::SvetiJurijObScavnici),
                "Sveti Jurij v Slovenskih Goricah Municipality" => {
                    Ok(Self::SvetiJurijVSlovenskihGoricah)
                }
                "Sveti Tomaž Municipality" => Ok(Self::SvetiTomaz),
                "Tabor Municipality" => Ok(Self::Tabor),
                "Tišina Municipality" => Ok(Self::Tišina),
                "Tolmin Municipality" => Ok(Self::Tolmin),
                "Trbovlje Municipality" => Ok(Self::Trbovlje),
                "Trebnje Municipality" => Ok(Self::Trebnje),
                "Trnovska Vas Municipality" => Ok(Self::TrnovskaVas),
                "Trzin Municipality" => Ok(Self::Trzin),
                "Tržič Municipality" => Ok(Self::Tržič),
                "Turnišče Municipality" => Ok(Self::Turnišče),
                "Velika Polana Municipality" => Ok(Self::VelikaPolana),
                "Velike Lašče Municipality" => Ok(Self::VelikeLašče),
                "Veržej Municipality" => Ok(Self::Veržej),
                "Videm Municipality" => Ok(Self::Videm),
                "Vipava Municipality" => Ok(Self::Vipava),
                "Vitanje Municipality" => Ok(Self::Vitanje),
                "Vodice Municipality" => Ok(Self::Vodice),
                "Vojnik Municipality" => Ok(Self::Vojnik),
                "Vransko Municipality" => Ok(Self::Vransko),
                "Vrhnika Municipality" => Ok(Self::Vrhnika),
                "Vuzenica Municipality" => Ok(Self::Vuzenica),
                "Zagorje ob Savi Municipality" => Ok(Self::ZagorjeObSavi),
                "Zavrč Municipality" => Ok(Self::Zavrč),
                "Zreče Municipality" => Ok(Self::Zreče),
                "Črenšovci Municipality" => Ok(Self::Črenšovci),
                "Črna na Koroškem Municipality" => Ok(Self::ČrnaNaKoroškem),
                "Črnomelj Municipality" => Ok(Self::Črnomelj),
                "Šalovci Municipality" => Ok(Self::Šalovci),
                "Šempeter–Vrtojba Municipality" => Ok(Self::ŠempeterVrtojba),
                "Šentilj Municipality" => Ok(Self::Šentilj),
                "Šentjernej Municipality" => Ok(Self::Šentjernej),
                "Šentjur Municipality" => Ok(Self::Šentjur),
                "Šentrupert Municipality" => Ok(Self::Šentrupert),
                "Šenčur Municipality" => Ok(Self::Šenčur),
                "Škocjan Municipality" => Ok(Self::Škocjan),
                "Škofja Loka Municipality" => Ok(Self::ŠkofjaLoka),
                "Šmarje pri Jelšah Municipality" => Ok(Self::ŠmarjePriJelšah),
                "Šmarješke Toplice Municipality" => Ok(Self::ŠmarješkeToplice),
                "Šmartno ob Paki Municipality" => Ok(Self::ŠmartnoObPaki),
                "Šmartno pri Litiji Municipality" => Ok(Self::ŠmartnoPriLitiji),
                "Šoštanj Municipality" => Ok(Self::Šoštanj),
                "Štore Municipality" => Ok(Self::Štore),
                "Žalec Municipality" => Ok(Self::Žalec),
                "Železniki Municipality" => Ok(Self::Železniki),
                "Žetale Municipality" => Ok(Self::Žetale),
                "Žiri Municipality" => Ok(Self::Žiri),
                "Žirovnica Municipality" => Ok(Self::Žirovnica),
                "Žužemberk Municipality" => Ok(Self::Žužemberk),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
        }
    }
}

impl ForeignTryFrom<String> for UkraineStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.clone(), "UkraineStatesAbbreviation");

        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => match value.as_str() {
                "Autonomous Republic of Crimea" => Ok(Self::AutonomousRepublicOfCrimea),
                "Cherkasy Oblast" => Ok(Self::CherkasyOblast),
                "Chernihiv Oblast" => Ok(Self::ChernihivOblast),
                "Chernivtsi Oblast" => Ok(Self::ChernivtsiOblast),
                "Dnipropetrovsk Oblast" => Ok(Self::DnipropetrovskOblast),
                "Donetsk Oblast" => Ok(Self::DonetskOblast),
                "Ivano-Frankivsk Oblast" => Ok(Self::IvanoFrankivskOblast),
                "Kharkiv Oblast" => Ok(Self::KharkivOblast),
                "Kherson Oblast" => Ok(Self::KhersonOblast),
                "Khmelnytsky Oblast" => Ok(Self::KhmelnytskyOblast),
                "Kiev" => Ok(Self::Kiev),
                "Kirovohrad Oblast" => Ok(Self::KirovohradOblast),
                "Kyiv Oblast" => Ok(Self::KyivOblast),
                "Luhansk Oblast" => Ok(Self::LuhanskOblast),
                "Lviv Oblast" => Ok(Self::LvivOblast),
                "Mykolaiv Oblast" => Ok(Self::MykolaivOblast),
                "Odessa Oblast" => Ok(Self::OdessaOblast),
                "Rivne Oblast" => Ok(Self::RivneOblast),
                "Sumy Oblast" => Ok(Self::SumyOblast),
                "Ternopil Oblast" => Ok(Self::TernopilOblast),
                "Vinnytsia Oblast" => Ok(Self::VinnytsiaOblast),
                "Volyn Oblast" => Ok(Self::VolynOblast),
                "Zakarpattia Oblast" => Ok(Self::ZakarpattiaOblast),
                "Zaporizhzhya Oblast" => Ok(Self::ZaporizhzhyaOblast),
                "Zhytomyr Oblast" => Ok(Self::ZhytomyrOblast),
                _ => Err(errors::ConnectorError::InvalidDataFormat {
                    field_name: "address.state",
                }
                .into()),
            },
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
