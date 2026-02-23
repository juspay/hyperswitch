use common_enums::enums;
use common_utils::{
    errors::ParsingError,
    ext_traits::ValueExt,
    id_type,
    pii::{Email, IpAddress},
    request::Method,
    types::{MinorUnit, StringMajorUnit},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{
        BankRedirectData, BankTransferData, PayLaterData, PaymentMethodData, WalletData,
    },
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::{
        refunds::{Execute, RSync},
        PSync,
    },
    router_request_types::{PaymentsSyncData, ResponseId},
    router_response_types::{
        ConnectorCustomerResponseData, MandateReference, PaymentsResponseData, RedirectForm,
        RefundsResponseData,
    },
    types,
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;
use uuid::Uuid;

use crate::{
    types::{CreateOrderResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, BrowserInformationData, CardData as _, ForeignTryFrom, PaymentsAuthorizeRequestData,
        PhoneDetailsData, RouterData as _,
    },
};

pub struct AirwallexAuthType {
    pub x_api_key: Secret<String>,
    pub x_client_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for AirwallexAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                x_api_key: api_key.clone(),
                x_client_id: key1.clone(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ReferrerData {
    #[serde(rename = "type")]
    r_type: String,
    version: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexIntentRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    amount: StringMajorUnit,
    currency: enums::Currency,
    //ID created in merchant's order system that corresponds to this PaymentIntent.
    merchant_order_id: String,
    // This data is required to whitelist Hyperswitch at Airwallex.
    referrer_data: ReferrerData,
    order: Option<AirwallexOrderData>,
}

impl TryFrom<&AirwallexRouterData<&types::CreateOrderRouterData>> for AirwallexIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AirwallexRouterData<&types::CreateOrderRouterData>,
    ) -> Result<Self, Self::Error> {
        let referrer_data = ReferrerData {
            r_type: "hyperswitch".to_string(),
            version: "1.0.0".to_string(),
        };
        let amount = item.amount.clone();
        let currency = item.router_data.request.currency;

        let order = match item.router_data.request.payment_method_data {
            Some(PaymentMethodData::PayLater(_)) => Some(
                item.router_data
                    .request
                    .order_details
                    .as_ref()
                    .map(|order_data| AirwallexOrderData {
                        products: order_data
                            .iter()
                            .map(|product| AirwallexProductData {
                                name: product.product_name.clone(),
                                quantity: product.quantity,
                                unit_price: product.amount,
                            })
                            .collect(),
                        shipping: Some(AirwallexShippingData {
                            first_name: item.router_data.get_optional_shipping_first_name(),
                            last_name: item.router_data.get_optional_shipping_last_name(),
                            phone_number: item.router_data.get_optional_shipping_phone_number(),
                            address: AirwallexPLShippingAddress {
                                country_code: item.router_data.get_optional_shipping_country(),
                                city: item.router_data.get_optional_shipping_city(),
                                street: item.router_data.get_optional_shipping_line1(),
                                postcode: item.router_data.get_optional_shipping_zip(),
                            },
                        }),
                    })
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "order_details",
                    })?,
            ),
            _ => None,
        };

        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount,
            currency,
            merchant_order_id: item.router_data.connector_request_reference_id.clone(),
            referrer_data,
            order,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AirwallexOrderResponse {
    pub status: AirwallexPaymentStatus,
    pub id: String,
    pub payment_consent_id: Option<Secret<String>>,
    pub next_action: Option<AirwallexPaymentsNextAction>,
}

impl TryFrom<CreateOrderResponseRouterData<AirwallexOrderResponse>>
    for types::CreateOrderRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: CreateOrderResponseRouterData<AirwallexOrderResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::PaymentsCreateOrderResponse {
                order_id: item.response.id.clone(),
                session_token: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct AirwallexRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(StringMajorUnit, T)> for AirwallexRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, router_data): (StringMajorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct AirwallexPaymentsRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    payment_method: AirwallexPaymentMethod,
    payment_method_options: Option<AirwallexPaymentOptions>,
    return_url: Option<String>,
    device_data: Option<DeviceData>,
    payment_consent: Option<PaymentConsentData>,
    customer_id: Option<String>,
    payment_consent_id: Option<String>,
    triggered_by: Option<TriggeredBy>,
}

#[derive(Debug, Serialize)]
pub struct PaymentConsentData {
    next_triggered_by: TriggeredBy,
    merchant_trigger_reason: MerchantTriggeredReason,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MerchantTriggeredReason {
    Unscheduled,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggeredBy {
    Merchant,
    Customer,
}

#[derive(Debug, Serialize, Eq, PartialEq, Default)]
pub struct AirwallexOrderData {
    products: Vec<AirwallexProductData>,
    shipping: Option<AirwallexShippingData>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexProductData {
    name: String,
    quantity: u16,
    unit_price: MinorUnit,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexShippingData {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    phone_number: Option<Secret<String>>,
    address: AirwallexPLShippingAddress,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPLShippingAddress {
    country_code: Option<enums::CountryAlpha2>,
    city: Option<String>,
    street: Option<Secret<String>>,
    postcode: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub struct DeviceData {
    accept_header: String,
    browser: Browser,
    ip_address: Secret<String, IpAddress>,
    language: String,
    mobile: Option<Mobile>,
    screen_color_depth: u8,
    screen_height: u32,
    screen_width: u32,
    timezone: String,
}

#[derive(Debug, Serialize)]
pub struct Browser {
    java_enabled: bool,
    javascript_enabled: bool,
    user_agent: String,
}

#[derive(Debug, Serialize)]
pub struct Mobile {
    device_model: Option<String>,
    os_type: Option<String>,
    os_version: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AirwallexPaymentMethod {
    Card(AirwallexCard),
    Wallets(AirwallexWalletData),
    PayLater(AirwallexPayLaterData),
    BankRedirect(AirwallexBankRedirectData),
    BankTransfer(AirwallexBankTransferData),
    PaymentMethodId(AirwallexPaymentMethodId),
}

#[derive(Debug, Serialize)]
pub struct AirwallexPaymentMethodId {
    id: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct AirwallexCard {
    card: AirwallexCardDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}
#[derive(Debug, Serialize)]
pub struct AirwallexCardDetails {
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    number: cards::CardNumber,
    cvc: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AirwallexWalletData {
    GooglePay(GooglePayData),
    Paypal(PaypalData),
    Skrill(SkrillData),
}

#[derive(Debug, Serialize)]
pub struct GooglePayData {
    googlepay: GooglePayDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}

#[derive(Debug, Serialize)]
pub struct PaypalData {
    paypal: PaypalDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}

#[derive(Debug, Serialize)]
pub struct SkrillData {
    skrill: SkrillDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}

#[derive(Debug, Serialize)]
pub struct GooglePayDetails {
    encrypted_payment_token: Secret<String>,
    payment_data_type: GpayPaymentDataType,
}

#[derive(Debug, Serialize)]
pub struct PaypalDetails {
    shopper_name: Secret<String>,
    country_code: enums::CountryAlpha2,
}

#[derive(Debug, Serialize)]
pub struct SkrillDetails {
    shopper_name: Secret<String>,
    shopper_email: Email,
    country_code: enums::CountryAlpha2,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AirwallexPayLaterData {
    Klarna(Box<KlarnaData>),
    Atome(AtomeData),
}

#[derive(Debug, Serialize)]
pub struct KlarnaData {
    klarna: KlarnaDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}

#[derive(Debug, Serialize)]
pub struct KlarnaDetails {
    country_code: enums::CountryAlpha2,
    billing: Option<Billing>,
}

#[derive(Debug, Serialize)]
pub struct Billing {
    date_of_birth: Option<Secret<String>>,
    email: Option<Email>,
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    phone_number: Option<Secret<String>>,
    address: Option<AddressAirwallex>,
}

#[derive(Debug, Serialize)]
pub struct AddressAirwallex {
    country_code: Option<enums::CountryAlpha2>,
    city: Option<String>,
    street: Option<Secret<String>>,
    postcode: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub struct AtomeData {
    atome: AtomeDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}

#[derive(Debug, Serialize)]
pub struct AtomeDetails {
    shopper_phone: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AirwallexBankTransferData {
    IndonesianBankTransfer(IndonesianBankTransferData),
}

#[derive(Debug, Serialize)]
pub struct IndonesianBankTransferData {
    bank_transfer: IndonesianBankTransferDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}

#[derive(Debug, Serialize)]
pub struct IndonesianBankTransferDetails {
    shopper_name: Secret<String>,
    shopper_email: Email,
    bank_name: common_enums::BankNames,
    country_code: enums::CountryAlpha2,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AirwallexBankRedirectData {
    Trustly(TrustlyData),
    Blik(BlikData),
    Ideal(IdealData),
}

#[derive(Debug, Serialize)]
pub struct TrustlyData {
    trustly: TrustlyDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}

#[derive(Debug, Serialize)]
pub struct TrustlyDetails {
    shopper_name: Secret<String>,
    country_code: enums::CountryAlpha2,
}

#[derive(Debug, Serialize)]
pub struct BlikData {
    blik: BlikDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}

#[derive(Debug, Serialize)]
pub struct BlikDetails {
    shopper_name: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct IdealData {
    ideal: IdealDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}

#[derive(Debug, Serialize)]
pub struct IdealDetails {
    bank_name: Option<common_enums::BankNames>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AirwallexPaymentType {
    Card,
    Googlepay,
    Paypal,
    Klarna,
    Atome,
    Trustly,
    Blik,
    Ideal,
    Skrill,
    BankTransfer,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GpayPaymentDataType {
    EncryptedPaymentToken,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AirwallexPaymentOptions {
    Card(AirwallexCardPaymentOptions),
    Klarna(AirwallexPayLaterPaymentOptions),
    Atome(AirwallexPayLaterPaymentOptions),
}

#[derive(Debug, Serialize)]
pub struct AirwallexCardPaymentOptions {
    auto_capture: bool,
}

#[derive(Debug, Serialize)]
pub struct AirwallexPayLaterPaymentOptions {
    auto_capture: bool,
}

impl TryFrom<&AirwallexRouterData<&types::PaymentsAuthorizeRouterData>>
    for AirwallexPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AirwallexRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let mut payment_method_options = None;
        let request = &item.router_data.request;
        let payment_method = match request.payment_method_data.clone() {
            PaymentMethodData::Card(ccard) => {
                payment_method_options =
                    Some(AirwallexPaymentOptions::Card(AirwallexCardPaymentOptions {
                        auto_capture: matches!(
                            request.capture_method,
                            Some(enums::CaptureMethod::Automatic)
                                | Some(enums::CaptureMethod::SequentialAutomatic)
                                | None
                        ),
                    }));
                Ok(AirwallexPaymentMethod::Card(AirwallexCard {
                    card: AirwallexCardDetails {
                        number: ccard.card_number.clone(),
                        expiry_month: ccard.card_exp_month.clone(),
                        expiry_year: ccard.get_expiry_year_4_digit(),
                        cvc: ccard.card_cvc,
                    },
                    payment_method_type: AirwallexPaymentType::Card,
                }))
            }
            PaymentMethodData::Wallet(ref wallet_data) => get_wallet_details(wallet_data, item),
            PaymentMethodData::PayLater(ref paylater_data) => {
                let paylater_options = AirwallexPayLaterPaymentOptions {
                    auto_capture: item.router_data.request.is_auto_capture()?,
                };

                payment_method_options = match paylater_data {
                    PayLaterData::KlarnaRedirect { .. } => {
                        Some(AirwallexPaymentOptions::Klarna(paylater_options))
                    }
                    PayLaterData::AtomeRedirect { .. } => {
                        Some(AirwallexPaymentOptions::Atome(paylater_options))
                    }
                    _ => None,
                };

                get_paylater_details(paylater_data, item)
            }
            PaymentMethodData::BankTransfer(ref banktransfer_data) => {
                get_banktransfer_details(banktransfer_data, item)
            }
            PaymentMethodData::BankRedirect(ref bankredirect_data) => {
                get_bankredirect_details(bankredirect_data, item)
            }
            PaymentMethodData::MandatePayment => {
                let mandate_data = item
                    .router_data
                    .request
                    .get_connector_mandate_data()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_mandate_data",
                    })?;
                let mandate_metadata: AirwallexMandateMetadata = mandate_data
                    .get_mandate_metadata()
                    .ok_or(errors::ConnectorError::MissingConnectorMandateMetadata)?
                    .clone()
                    .parse_value("AirwallexMandateMetadata")
                    .change_context(errors::ConnectorError::ParsingFailed)?;

                Ok(AirwallexPaymentMethod::PaymentMethodId(
                    AirwallexPaymentMethodId {
                        id: mandate_metadata.id.ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "mandate_metadata.id",
                            },
                        )?,
                    },
                ))
            }
            PaymentMethodData::BankDebit(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_)
            | PaymentMethodData::CardWithLimitedDetails(_)
            | PaymentMethodData::DecryptedWalletTokenDetailsForNetworkTransactionId(_)
            | PaymentMethodData::NetworkTokenDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("airwallex"),
                ))
            }
        }?;

        let payment_consent = if item
            .router_data
            .request
            .is_customer_initiated_mandate_payment()
        {
            Some(PaymentConsentData {
                next_triggered_by: TriggeredBy::Merchant,
                merchant_trigger_reason: MerchantTriggeredReason::Unscheduled,
            })
        } else {
            None
        };

        let return_url = match &request.payment_method_data {
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::PaypalRedirect(_paypal_details) => {
                    item.router_data.request.router_return_url.clone()
                }
                WalletData::Skrill(_) => item.router_data.request.router_return_url.clone(),
                _ => request.complete_authorize_url.clone(),
            },
            PaymentMethodData::BankRedirect(_bankredirect_data) => {
                item.router_data.request.router_return_url.clone()
            }
            PaymentMethodData::PayLater(_paylater_data) => {
                item.router_data.request.router_return_url.clone()
            }
            _ => request.complete_authorize_url.clone(),
        };

        let is_mandate_payment = item.router_data.request.is_mit_payment()
            || item
                .router_data
                .request
                .is_customer_initiated_mandate_payment();

        let (device_data, customer_id) = if is_mandate_payment {
            let customer_id = item.router_data.get_connector_customer_id()?;
            (None, Some(customer_id))
        } else {
            let device_data = Some(get_device_data(item.router_data)?);
            (device_data, None)
        };

        let (payment_consent_id, triggered_by) = if item.router_data.request.is_mit_payment() {
            let mandate_id = item.router_data.request.connector_mandate_id().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "connector_mandate_id",
                },
            )?;

            (Some(mandate_id), Some(TriggeredBy::Merchant))
        } else {
            (None, None)
        };

        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            payment_method,
            payment_method_options,
            return_url,
            device_data,
            payment_consent,
            customer_id,
            payment_consent_id,
            triggered_by,
        })
    }
}

fn get_device_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<DeviceData, error_stack::Report<errors::ConnectorError>> {
    let info = item.request.get_browser_info()?;
    let browser = Browser {
        java_enabled: info.get_java_enabled()?,
        javascript_enabled: info.get_java_script_enabled()?,
        user_agent: info.get_user_agent()?,
    };
    let mobile = {
        let device_model = info.get_device_model().ok();
        let os_type = info.get_os_type().ok();
        let os_version = info.get_os_version().ok();

        if device_model.is_some() || os_type.is_some() || os_version.is_some() {
            Some(Mobile {
                device_model,
                os_type,
                os_version,
            })
        } else {
            None
        }
    };
    Ok(DeviceData {
        accept_header: info.get_accept_header()?,
        browser,
        ip_address: info.get_ip_address()?,
        mobile,
        screen_color_depth: info.get_color_depth()?,
        screen_height: info.get_screen_height()?,
        screen_width: info.get_screen_width()?,
        timezone: info.get_time_zone()?.to_string(),
        language: info.get_language()?,
    })
}

fn get_banktransfer_details(
    banktransfer_data: &BankTransferData,
    item: &AirwallexRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<AirwallexPaymentMethod, errors::ConnectorError> {
    let _bank_transfer_details = match banktransfer_data {
        BankTransferData::IndonesianBankTransfer { bank_name } => {
            AirwallexPaymentMethod::BankTransfer(AirwallexBankTransferData::IndonesianBankTransfer(
                IndonesianBankTransferData {
                    bank_transfer: IndonesianBankTransferDetails {
                        shopper_name: item.router_data.get_billing_full_name().map_err(|_| {
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "shopper_name",
                            }
                        })?,
                        shopper_email: item.router_data.get_billing_email().map_err(|_| {
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "shopper_email",
                            }
                        })?,
                        bank_name: bank_name.ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "bank_name",
                            },
                        )?,
                        country_code: item.router_data.get_billing_country().map_err(|_| {
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "country_code",
                            }
                        })?,
                    },
                    payment_method_type: AirwallexPaymentType::BankTransfer,
                },
            ))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("airwallex"),
        ))?,
    };
    let not_implemented = Err(errors::ConnectorError::NotImplemented(
        utils::get_unimplemented_payment_method_error_message("airwallex"),
    ))?;
    Ok(not_implemented)
}

fn get_paylater_details(
    paylater_data: &PayLaterData,
    item: &AirwallexRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<AirwallexPaymentMethod, errors::ConnectorError> {
    let paylater_details = match paylater_data {
        PayLaterData::KlarnaRedirect {} => {
            AirwallexPaymentMethod::PayLater(AirwallexPayLaterData::Klarna(Box::new(KlarnaData {
                klarna: KlarnaDetails {
                    country_code: item.router_data.get_billing_country().map_err(|_| {
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "country_code",
                        }
                    })?,
                    billing: Some(Billing {
                        date_of_birth: None,
                        first_name: item.router_data.get_optional_billing_first_name(),
                        last_name: item.router_data.get_optional_billing_last_name(),
                        email: item.router_data.get_optional_billing_email(),
                        phone_number: item.router_data.get_optional_billing_phone_number(),
                        address: Some(AddressAirwallex {
                            country_code: item.router_data.get_optional_billing_country(),
                            city: item.router_data.get_optional_billing_city(),
                            street: item.router_data.get_optional_billing_line1(),
                            postcode: item.router_data.get_optional_billing_zip(),
                        }),
                    }),
                },
                payment_method_type: AirwallexPaymentType::Klarna,
            })))
        }
        PayLaterData::AtomeRedirect {} => {
            AirwallexPaymentMethod::PayLater(AirwallexPayLaterData::Atome(AtomeData {
                atome: AtomeDetails {
                    shopper_phone: item
                        .router_data
                        .get_billing_phone()
                        .map_err(|_| errors::ConnectorError::MissingRequiredField {
                            field_name: "shopper_phone",
                        })?
                        .get_number_with_country_code()
                        .map_err(|_| errors::ConnectorError::MissingRequiredField {
                            field_name: "country_code",
                        })?,
                },
                payment_method_type: AirwallexPaymentType::Atome,
            }))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("airwallex"),
        ))?,
    };
    Ok(paylater_details)
}

fn get_bankredirect_details(
    bankredirect_data: &BankRedirectData,
    item: &AirwallexRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<AirwallexPaymentMethod, errors::ConnectorError> {
    let bank_redirect_details = match bankredirect_data {
        BankRedirectData::Trustly { .. } => {
            AirwallexPaymentMethod::BankRedirect(AirwallexBankRedirectData::Trustly(TrustlyData {
                trustly: TrustlyDetails {
                    shopper_name: item.router_data.get_billing_full_name().map_err(|_| {
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "shopper_name",
                        }
                    })?,
                    country_code: item.router_data.get_billing_country().map_err(|_| {
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "country_code",
                        }
                    })?,
                },
                payment_method_type: AirwallexPaymentType::Trustly,
            }))
        }
        BankRedirectData::Blik { .. } => {
            AirwallexPaymentMethod::BankRedirect(AirwallexBankRedirectData::Blik(BlikData {
                blik: BlikDetails {
                    shopper_name: item.router_data.get_billing_full_name().map_err(|_| {
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "shopper_name",
                        }
                    })?,
                },
                payment_method_type: AirwallexPaymentType::Blik,
            }))
        }
        BankRedirectData::Ideal { .. } => {
            AirwallexPaymentMethod::BankRedirect(AirwallexBankRedirectData::Ideal(IdealData {
                ideal: IdealDetails { bank_name: None },
                payment_method_type: AirwallexPaymentType::Ideal,
            }))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("airwallex"),
        ))?,
    };
    Ok(bank_redirect_details)
}

fn get_wallet_details(
    wallet_data: &WalletData,
    item: &AirwallexRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<AirwallexPaymentMethod, errors::ConnectorError> {
    let wallet_details: AirwallexPaymentMethod = match wallet_data {
        WalletData::GooglePay(gpay_details) => {
            let token = gpay_details
                .tokenization_data
                .get_encrypted_google_pay_token()
                .attach_printable("Failed to get gpay wallet token")
                .map_err(|_| errors::ConnectorError::MissingRequiredField {
                    field_name: "gpay wallet_token",
                })?;
            AirwallexPaymentMethod::Wallets(AirwallexWalletData::GooglePay(GooglePayData {
                googlepay: GooglePayDetails {
                    encrypted_payment_token: Secret::new(token.clone()),
                    payment_data_type: GpayPaymentDataType::EncryptedPaymentToken,
                },
                payment_method_type: AirwallexPaymentType::Googlepay,
            }))
        }
        WalletData::PaypalRedirect(_paypal_details) => {
            AirwallexPaymentMethod::Wallets(AirwallexWalletData::Paypal(PaypalData {
                paypal: PaypalDetails {
                    shopper_name: item
                        .router_data
                        .request
                        .customer_name
                        .as_ref()
                        .cloned()
                        .or_else(|| item.router_data.get_billing_full_name().ok())
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "shopper_name",
                        })?,
                    country_code: item.router_data.get_billing_country().map_err(|_| {
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "country_code",
                        }
                    })?,
                },
                payment_method_type: AirwallexPaymentType::Paypal,
            }))
        }
        WalletData::Skrill(_skrill_details) => {
            AirwallexPaymentMethod::Wallets(AirwallexWalletData::Skrill(SkrillData {
                skrill: SkrillDetails {
                    shopper_name: item
                        .router_data
                        .request
                        .customer_name
                        .as_ref()
                        .cloned()
                        .or_else(|| item.router_data.get_billing_full_name().ok())
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "shopper_name",
                        })?,
                    shopper_email: item.router_data.get_billing_email().map_err(|_| {
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "shopper_email",
                        }
                    })?,
                    country_code: item.router_data.get_billing_country().map_err(|_| {
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "country_code",
                        }
                    })?,
                },
                payment_method_type: AirwallexPaymentType::Skrill,
            }))
        }
        WalletData::AliPayQr(_)
        | WalletData::AliPayRedirect(_)
        | WalletData::AliPayHkRedirect(_)
        | WalletData::AmazonPayRedirect(_)
        | WalletData::Paysera(_)
        | WalletData::MomoRedirect(_)
        | WalletData::KakaoPayRedirect(_)
        | WalletData::GoPayRedirect(_)
        | WalletData::GcashRedirect(_)
        | WalletData::AmazonPay(_)
        | WalletData::ApplePay(_)
        | WalletData::BluecodeRedirect {}
        | WalletData::ApplePayRedirect(_)
        | WalletData::ApplePayThirdPartySdk(_)
        | WalletData::DanaRedirect {}
        | WalletData::GooglePayRedirect(_)
        | WalletData::GooglePayThirdPartySdk(_)
        | WalletData::MbWayRedirect(_)
        | WalletData::MobilePayRedirect(_)
        | WalletData::PaypalSdk(_)
        | WalletData::Paze(_)
        | WalletData::SamsungPay(_)
        | WalletData::TwintRedirect {}
        | WalletData::VippsRedirect {}
        | WalletData::TouchNGoRedirect(_)
        | WalletData::WeChatPayRedirect(_)
        | WalletData::WeChatPayQr(_)
        | WalletData::CashappQr(_)
        | WalletData::SwishQr(_)
        | WalletData::Mifinity(_)
        | WalletData::RevolutPay(_) => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("airwallex"),
        ))?,
    };
    Ok(wallet_details)
}

#[derive(Deserialize, Debug, Serialize)]
pub struct AirwallexAuthUpdateResponse {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    expires_at: PrimitiveDateTime,
    token: Secret<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, AirwallexAuthUpdateResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AirwallexAuthUpdateResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        let expires = (item.response.expires_at - common_utils::date_time::now()).whole_seconds();
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.token,
                expires,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexCompleteRequest {
    request_id: String,
    three_ds: AirwallexThreeDsData,
    #[serde(rename = "type")]
    three_ds_type: AirwallexThreeDsType,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexThreeDsData {
    acs_response: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub enum AirwallexThreeDsType {
    #[default]
    #[serde(rename = "3ds_continue")]
    ThreeDSContinue,
}

impl TryFrom<&types::PaymentsCompleteAuthorizeRouterData> for AirwallexCompleteRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            three_ds: AirwallexThreeDsData {
                acs_response: item
                    .request
                    .redirect_response
                    .as_ref()
                    .map(|f| f.payload.to_owned())
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "redirect_response.payload",
                    })?
                    .as_ref()
                    .map(|data| serde_json::to_string(data.peek()))
                    .transpose()
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?
                    .map(Secret::new),
            },
            three_ds_type: AirwallexThreeDsType::ThreeDSContinue,
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentsCaptureRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    amount: Option<String>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for AirwallexPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: Some(utils::to_currency_base_unit(
                item.request.amount_to_capture,
                item.request.currency,
            )?),
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentsCancelRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    cancellation_reason: Option<String>,
}

impl TryFrom<&types::PaymentsCancelRouterData> for AirwallexPaymentsCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            cancellation_reason: item.request.cancellation_reason.clone(),
        })
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AirwallexPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
    RequiresPaymentMethod,
    RequiresCustomerAction,
    RequiresCapture,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AirwallexNextAction {
    Payments(AirwallexPaymentsNextAction),
    Redirect(AirwallexRedirectNextAction),
}

fn get_payment_status(
    status: &AirwallexPaymentStatus,
    next_action: &Option<AirwallexNextAction>,
) -> enums::AttemptStatus {
    match status.clone() {
        AirwallexPaymentStatus::Succeeded => enums::AttemptStatus::Charged,
        AirwallexPaymentStatus::Failed => enums::AttemptStatus::Failure,
        AirwallexPaymentStatus::Pending => enums::AttemptStatus::Pending,
        AirwallexPaymentStatus::RequiresPaymentMethod => enums::AttemptStatus::PaymentMethodAwaited,
        AirwallexPaymentStatus::RequiresCustomerAction => next_action.as_ref().map_or(
            enums::AttemptStatus::AuthenticationPending,
            |next_action| match next_action {
                AirwallexNextAction::Payments(payments_next_action) => {
                    match payments_next_action.stage {
                        AirwallexNextActionStage::WaitingDeviceDataCollection => {
                            enums::AttemptStatus::DeviceDataCollectionPending
                        }
                        AirwallexNextActionStage::WaitingUserInfoInput => {
                            enums::AttemptStatus::AuthenticationPending
                        }
                    }
                }
                AirwallexNextAction::Redirect(_) => enums::AttemptStatus::AuthenticationPending,
            },
        ),
        AirwallexPaymentStatus::RequiresCapture => enums::AttemptStatus::Authorized,
        AirwallexPaymentStatus::Cancelled => enums::AttemptStatus::Voided,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AirwallexNextActionStage {
    WaitingDeviceDataCollection,
    WaitingUserInfoInput,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AirwallexRedirectFormData {
    #[serde(rename = "JWT")]
    jwt: Option<Secret<String>>,
    #[serde(rename = "threeDSMethodData")]
    three_ds_method_data: Option<Secret<String>>,
    token: Option<Secret<String>>,
    provider: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AirwallexPaymentsNextAction {
    url: Url,
    method: Method,
    data: AirwallexRedirectFormData,
    stage: AirwallexNextActionStage,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AirwallexRedirectNextAction {
    url: Url,
    method: Method,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AirwallexPaymentsResponse {
    status: AirwallexPaymentStatus,
    //Unique identifier for the PaymentIntent
    id: String,
    amount: Option<f32>,
    //ID of the PaymentConsent related to this PaymentIntent
    payment_consent_id: Option<Secret<String>>,
    next_action: Option<AirwallexPaymentsNextAction>,
    latest_payment_attempt: Option<AirwallexPaymentAttemptResponse>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AirwallexPaymentAttemptResponse {
    payment_method: Option<AirwallexPaymentMethodResponse>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AirwallexPaymentMethodResponse {
    id: Option<Secret<String>>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AirwallexMandateMetadata {
    id: Option<Secret<String>>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AirwallexRedirectResponse {
    status: AirwallexPaymentStatus,
    id: String,
    amount: Option<f32>,
    payment_consent_id: Option<Secret<String>>,
    next_action: Option<AirwallexRedirectNextAction>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AirwallexPaymentsSyncResponse {
    status: AirwallexPaymentStatus,
    //Unique identifier for the PaymentIntent
    id: String,
    amount: Option<f32>,
    //ID of the PaymentConsent related to this PaymentIntent
    payment_consent_id: Option<Secret<String>>,
    next_action: Option<AirwallexPaymentsNextAction>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AirwallexAuthorizeResponse {
    AirwallexPaymentsResponse(AirwallexPaymentsResponse),
    AirwallexRedirectResponse(AirwallexRedirectResponse),
}

fn get_redirection_form(response_url_data: AirwallexPaymentsNextAction) -> Option<RedirectForm> {
    Some(RedirectForm::Form {
        endpoint: response_url_data.url.to_string(),
        method: response_url_data.method,
        form_fields: std::collections::HashMap::from([
            //Some form fields might be empty based on the authentication type by the connector
            (
                "JWT".to_string(),
                response_url_data
                    .data
                    .jwt
                    .map(|jwt| jwt.expose())
                    .unwrap_or_default(),
            ),
            (
                "threeDSMethodData".to_string(),
                response_url_data
                    .data
                    .three_ds_method_data
                    .map(|three_ds_method_data| three_ds_method_data.expose())
                    .unwrap_or_default(),
            ),
            (
                "token".to_string(),
                response_url_data
                    .data
                    .token
                    .map(|token: Secret<String>| token.expose())
                    .unwrap_or_default(),
            ),
            (
                "provider".to_string(),
                response_url_data.data.provider.unwrap_or_default(),
            ),
            (
                "version".to_string(),
                response_url_data.data.version.unwrap_or_default(),
            ),
        ]),
    })
}

impl<F, T>
    ForeignTryFrom<ResponseRouterData<F, AirwallexAuthorizeResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: ResponseRouterData<F, AirwallexAuthorizeResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let ResponseRouterData {
            response,
            data,
            http_code,
        } = item;

        match response {
            AirwallexAuthorizeResponse::AirwallexPaymentsResponse(res) => {
                Self::try_from(ResponseRouterData::<
                    F,
                    AirwallexPaymentsResponse,
                    T,
                    PaymentsResponseData,
                > {
                    response: res,
                    data,
                    http_code,
                })
            }
            AirwallexAuthorizeResponse::AirwallexRedirectResponse(res) => {
                Self::try_from(ResponseRouterData::<
                    F,
                    AirwallexRedirectResponse,
                    T,
                    PaymentsResponseData,
                > {
                    response: res,
                    data,
                    http_code,
                })
            }
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, AirwallexPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AirwallexPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, redirection_data) = item.response.next_action.clone().map_or(
            // If no next action is there, map the status and set redirection form as None
            (
                get_payment_status(
                    &item.response.status,
                    &item
                        .response
                        .next_action
                        .clone()
                        .map(AirwallexNextAction::Payments)
                        .clone(),
                ),
                None,
            ),
            |response_url_data| {
                // If the connector sends a customer action response that is already under
                // process from our end it can cause an infinite loop to break this this check
                // is added and fail the payment
                if matches!(
                    (
                        response_url_data.stage.clone(),
                        item.data.status,
                        item.response.status.clone(),
                    ),
                    // If the connector sends waiting for DDC and our status is already DDC Pending
                    // that means we initiated the call to collect the data and now we expect a different response
                    (
                            AirwallexNextActionStage::WaitingDeviceDataCollection,
                            enums::AttemptStatus::DeviceDataCollectionPending,
                            _
                        )
                        // If the connector sends waiting for Customer Action and our status is already Authenticaition Pending
                        // that means we initiated the call to authenticate and now we do not expect a requires_customer action
                        // it will start a loop
                        | (
                            _,
                            enums::AttemptStatus::AuthenticationPending,
                            AirwallexPaymentStatus::RequiresCustomerAction,
                        )
                ) {
                    // Fail the payment for above conditions
                    (enums::AttemptStatus::AuthenticationFailed, None)
                } else {
                    (
                        //Build the redirect form and update the payment status
                        get_payment_status(
                            &item.response.status,
                            &item
                                .response
                                .next_action
                                .map(AirwallexNextAction::Payments)
                                .clone(),
                        ),
                        get_redirection_form(response_url_data),
                    )
                }
            },
        );

        let mandate_reference = Box::new(Some(MandateReference {
            connector_mandate_id: item
                .response
                .payment_consent_id
                .clone()
                .map(|id| id.expose()),
            payment_method_id: None,
            mandate_metadata: item
                .response
                .latest_payment_attempt
                .and_then(|attempt| attempt.payment_method)
                .map(|pm| Secret::new(serde_json::json!(AirwallexMandateMetadata { id: pm.id }))),
            connector_mandate_request_reference_id: None,
        }));

        Ok(Self {
            status,
            reference_id: Some(item.response.id.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, AirwallexRedirectResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AirwallexRedirectResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, redirection_data) = item.response.next_action.clone().map_or(
            (
                get_payment_status(
                    &item.response.status,
                    &item
                        .response
                        .next_action
                        .clone()
                        .map(AirwallexNextAction::Redirect)
                        .clone(),
                ),
                None,
            ),
            |response_url_data| {
                let redirection_data =
                    Some(RedirectForm::from((response_url_data.url, Method::Get)));
                (
                    get_payment_status(
                        &item.response.status,
                        &item
                            .response
                            .next_action
                            .map(AirwallexNextAction::Redirect)
                            .clone(),
                    ),
                    redirection_data,
                )
            },
        );

        Ok(Self {
            status,
            reference_id: Some(item.response.id.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            PSync,
            AirwallexPaymentsSyncResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    > for types::PaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            PSync,
            AirwallexPaymentsSyncResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = get_payment_status(
            &item.response.status,
            &item
                .response
                .next_action
                .clone()
                .map(AirwallexNextAction::Payments)
                .clone(),
        );
        let redirection_data = if let Some(redirect_url_data) = item.response.next_action {
            get_redirection_form(redirect_url_data)
        } else {
            None
        };
        Ok(Self {
            status,
            reference_id: Some(item.response.id.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct AirwallexRefundRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    amount: Option<StringMajorUnit>,
    reason: Option<String>,
    //Identifier for the PaymentIntent for which Refund is requested
    payment_intent_id: String,
}

impl<F> TryFrom<&AirwallexRouterData<&types::RefundsRouterData<F>>> for AirwallexRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AirwallexRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: Some(item.amount.to_owned()),
            reason: item.router_data.request.reason.clone(),
            payment_intent_id: item.router_data.request.connector_transaction_id.clone(),
        })
    }
}

// Type definition for Refund Response
#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Received,
    Accepted,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Received | RefundStatus::Accepted => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    //A unique number that tags a credit or debit card transaction when it goes from the merchant's bank through to the cardholder's bank.
    acquirer_reference_number: Option<String>,
    amount: f32,
    //Unique identifier for the Refund
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<ParsingError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for types::RefundsRouterData<RSync> {
    type Error = error_stack::Report<ParsingError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AirwallexWebhookData {
    pub source_id: Option<String>,
    pub name: AirwallexWebhookEventType,
    pub data: AirwallexObjectData,
}

#[derive(Debug, Deserialize, strum::Display, PartialEq)]
pub enum AirwallexWebhookEventType {
    #[serde(rename = "payment_intent.created")]
    PaymentIntentCreated,
    #[serde(rename = "payment_intent.requires_payment_method")]
    PaymentIntentRequiresPaymentMethod,
    #[serde(rename = "payment_intent.cancelled")]
    PaymentIntentCancelled,
    #[serde(rename = "payment_intent.succeeded")]
    PaymentIntentSucceeded,
    #[serde(rename = "payment_intent.requires_capture")]
    PaymentIntentRequiresCapture,
    #[serde(rename = "payment_intent.requires_customer_action")]
    PaymentIntentRequiresCustomerAction,
    #[serde(rename = "payment_attempt.authorized")]
    PaymentAttemptAuthorized,
    #[serde(rename = "payment_attempt.authorization_failed")]
    PaymentAttemptAuthorizationFailed,
    #[serde(rename = "payment_attempt.capture_requested")]
    PaymentAttemptCaptureRequested,
    #[serde(rename = "payment_attempt.capture_failed")]
    PaymentAttemptCaptureFailed,
    #[serde(rename = "payment_attempt.authentication_redirected")]
    PaymentAttemptAuthenticationRedirected,
    #[serde(rename = "payment_attempt.authentication_failed")]
    PaymentAttemptAuthenticationFailed,
    #[serde(rename = "payment_attempt.failed_to_process")]
    PaymentAttemptFailedToProcess,
    #[serde(rename = "payment_attempt.cancelled")]
    PaymentAttemptCancelled,
    #[serde(rename = "payment_attempt.expired")]
    PaymentAttemptExpired,
    #[serde(rename = "payment_attempt.risk_declined")]
    PaymentAttemptRiskDeclined,
    #[serde(rename = "payment_attempt.settled")]
    PaymentAttemptSettled,
    #[serde(rename = "payment_attempt.paid")]
    PaymentAttemptPaid,
    #[serde(rename = "refund.received")]
    RefundReceived,
    #[serde(rename = "refund.accepted")]
    RefundAccepted,
    #[serde(rename = "refund.succeeded")]
    RefundSucceeded,
    #[serde(rename = "refund.failed")]
    RefundFailed,
    #[serde(rename = "dispute.rfi_responded_by_merchant")]
    DisputeRfiRespondedByMerchant,
    #[serde(rename = "dispute.dispute.pre_chargeback_accepted")]
    DisputePreChargebackAccepted,
    #[serde(rename = "dispute.accepted")]
    DisputeAccepted,
    #[serde(rename = "dispute.dispute_received_by_merchant")]
    DisputeReceivedByMerchant,
    #[serde(rename = "dispute.dispute_responded_by_merchant")]
    DisputeRespondedByMerchant,
    #[serde(rename = "dispute.won")]
    DisputeWon,
    #[serde(rename = "dispute.lost")]
    DisputeLost,
    #[serde(rename = "dispute.dispute_reversed")]
    DisputeReversed,
    #[serde(other)]
    Unknown,
}

pub fn is_transaction_event(event_code: &AirwallexWebhookEventType) -> bool {
    matches!(
        event_code,
        AirwallexWebhookEventType::PaymentAttemptFailedToProcess
            | AirwallexWebhookEventType::PaymentAttemptAuthorized
    )
}

pub fn is_refund_event(event_code: &AirwallexWebhookEventType) -> bool {
    matches!(
        event_code,
        AirwallexWebhookEventType::RefundSucceeded | AirwallexWebhookEventType::RefundFailed
    )
}

pub fn is_dispute_event(event_code: &AirwallexWebhookEventType) -> bool {
    matches!(
        event_code,
        AirwallexWebhookEventType::DisputeAccepted
            | AirwallexWebhookEventType::DisputePreChargebackAccepted
            | AirwallexWebhookEventType::DisputeRespondedByMerchant
            | AirwallexWebhookEventType::DisputeWon
            | AirwallexWebhookEventType::DisputeLost
    )
}

#[derive(Debug, Deserialize)]
pub struct AirwallexObjectData {
    pub object: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AirwallexDisputeObject {
    pub payment_intent_id: String,
    pub dispute_amount: MinorUnit,
    pub dispute_currency: enums::Currency,
    pub stage: AirwallexDisputeStage,
    pub dispute_id: String,
    pub dispute_reason_type: Option<String>,
    pub dispute_original_reason_code: Option<String>,
    pub status: String,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize, strum::Display, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AirwallexDisputeStage {
    Rfi,
    Dispute,
    Arbitration,
}

#[derive(Debug, Deserialize)]
pub struct AirwallexWebhookDataResource {
    // Should this be a secret by default since it represents webhook payload
    pub object: Secret<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AirwallexWebhookObjectResource {
    pub data: AirwallexWebhookDataResource,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AirwallexErrorResponse {
    pub code: String,
    pub message: String,
    pub source: Option<String>,
    pub provider_original_response_code: Option<String>,
}

pub fn map_error_code_to_message(code: String) -> Option<String> {
    match code.as_str() {
        "01" => Some("Contact card issuer".to_string()),
        "03" => Some("Invalid Merchant".to_string()),
        "04" => Some("Pick up card(no fraud)".to_string()),
        "05" => Some("Do not honor".to_string()),
        "06" => Some("Error".to_string()),
        "07" => Some("Pick up card, special condition (fraud account)".to_string()),
        "12" => Some("Invalid transaction".to_string()),
        "13" => Some("Invalid amount".to_string()),
        "14" => Some("Invalid card number".to_string()),
        "15" => Some("Invalid issuer".to_string()),
        "19" => Some("Re-enter transaction".to_string()),
        "21" => Some("No action taken".to_string()),
        "22" => Some("Operation error".to_string()),
        "30" => Some("Format error".to_string()),
        "34" => Some("Fraudulent card".to_string()),
        "40" => Some("Transaction that is not supported by the Issuer".to_string()),
        "41" => Some("Lost card".to_string()),
        "43" => Some("Stolen card".to_string()),
        "46" => Some("Closed account".to_string()),
        "51" => Some("Insufficient funds/over credit limit / Not sufficient funds".to_string()),
        "52" => Some("No checking account".to_string()),
        "53" => Some("No savings account".to_string()),
        "54" => Some("Expired card".to_string()),
        "55" => Some("Incorrect PIN".to_string()),
        "57" => Some("Transaction not permitted to issuer/cardholder".to_string()),
        "58" => Some("Transaction not permitted to acquirer/terminal".to_string()),
        "59" => Some("Suspected fraud".to_string()),
        "61" => Some("Exceeds withdrawal limit".to_string()),
        "62" => Some("Restricted card".to_string()),
        "63" => Some("Security violation".to_string()),
        "64" => Some("AML requirement failure / Original transaction amount mismatch".to_string()),
        "65" => Some("Exceeds withdrawal count limit / Additional customer authentication required".to_string()),
        "6P" => Some("Customer ID verification failed".to_string()),
        "70" => Some("Contact Card Issuer".to_string()),
        "72" => Some("Account not yet activated".to_string()),
        "78" => Some("Invalid/nonexistent account specified (general)".to_string()),
        "79" => Some("Life Cycle".to_string()),
        "80" => Some("Credit issuer unavailable	".to_string()),
        "82" => Some("Policy / Negative online CAM, dCVV, iCVV, CVV, or CAVV results or Offline PIN authentication interrupted".to_string()),
        "83" => Some("Fraud / Security violation".to_string()),
        "85" => Some("No reason to decline".to_string()),
        "90" => Some("Decline due to daily cutoff being in progress".to_string()),
        "91" => Some("Authorization Platform or issuer system inoperative / Issuer not available OR Issuer unavailable or switch inoperative".to_string()),
        "92" => Some("Destination cannot be found for routing / Unable to route transaction".to_string()),
        "93" => Some("Transaction cannot be completed; violation of law".to_string()),
        "96" => Some("System malfunction".to_string()),
        "1A" => Some("Authentication Required".to_string()),
        "R0" => Some("Stop payment order".to_string()),
        "R1" => Some("Revocation of authorisation order".to_string()),
        "R3" => Some("Revocation of all authorisation orders".to_string()),
        "N7" => Some("Decline for CVV2 failure".to_string()),
        "5C" => Some("Transaction not supported / blocked by issuer".to_string()),
        "9G" => Some("Blocked by cardholder / contact cardholder".to_string()),
        "100" => Some("Deny / Do Not Honor".to_string()),
        "101" => Some("Expired Card / Invalid Expiration Date".to_string()),
        "109" => Some("Invalid merchant".to_string()),
        "110" => Some("Invalid amount".to_string()),
        "111" => Some("Invalid account / Invalid MICR (Travelers Cheque) / Invalid Card Number".to_string()),
        "115" => Some("Requested function not supported".to_string()),
        "116" => Some("Not sufficient funds".to_string()),
        "119" => Some("Cardmember not enrolled / not permitted".to_string()),
        "121" => Some("Limit exceeded".to_string()),
        "122" => Some("Invalid card security code (a.k.a., CID, 4DBC, 4CSC) / Card Validity Period Exceeded".to_string()),
        "130" => Some("Additional customer identification required".to_string()),
        "181" => Some("Format error".to_string()),
        "183" => Some("Invalid currency code".to_string()),
        "187" => Some("Deny - new card issued".to_string()),
        "189" => Some("Deny - Canceled or Closed Merchant/SE".to_string()),
        "190" => Some("National ID mismatch".to_string()),
        "200" => Some("Deny - Pick up card / Do Not Honor".to_string()),
        "909" => Some("System Malfunction (Cryptographic error)".to_string()),
        "912" => Some("Issuer not available".to_string()),
        "978" => Some("Invalid Payment Times".to_string()),
        "800.100.100" => Some("Transaction declined for unknown reason".to_string()),
        "800.100.150" => Some("Transaction declined (refund on gambling tx not allowed)".to_string()),
        "800.100.151" => Some("Transaction declined (invalid card)".to_string()),
        "800.100.152" => Some("Transaction declined by authorization system".to_string()),
        "800.100.153" => Some("Transaction declined (invalid CVV)".to_string()),
        "800.100.154" => Some("Transaction declined (transaction marked as invalid)".to_string()),
        "800.100.155" => Some("Transaction declined (amount exceeds credit)".to_string()),
        "800.100.156" => Some("Transaction declined (format error)".to_string()),
        "800.100.157" => Some("Transaction declined (wrong expiry date)".to_string()),
        "800.100.158" => Some("Transaction declined (suspecting manipulation)".to_string()),
        "800.100.159" => Some("Transaction declined (stolen card)".to_string()),
        "800.100.160" => Some("Transaction declined (card blocked)".to_string()),
        "800.100.161" => Some("Transaction declined (too many invalid tries)".to_string()),
        "800.100.162" => Some("Transaction declined (limit exceeded)".to_string()),
        "800.100.163" => Some("Transaction declined (maximum transaction frequency exceeded)".to_string()),
        "800.100.164" => Some("Transaction declined (merchants limit exceeded)".to_string()),
        "800.100.165" => Some("Transaction declined (card lost)".to_string()),
        "800.100.168" => Some("Transaction declined (restricted card)".to_string()),
        "800.100.169" => Some("Transaction declined (card type is not processed by the authorization center)".to_string()),
        "800.100.170" => Some("Transaction declined (transaction not permitted)".to_string()),
        "800.100.171" => Some("Transaction declined (pick up card)".to_string()),
        "800.100.172" => Some("Transaction declined (account blocked)".to_string()),
        "800.100.173" => Some("Transaction declined (invalid currency, not processed by authorization center)".to_string()),
        "800.100.174" => Some("Insufficient Funds".to_string()),
        "800.100.176" => Some("Transaction declined (account temporarily not available. Please try again later)".to_string()),
        "800.100.179" => Some("Transaction declined (exceeds withdrawal count limit)".to_string()),
        "800.100.190" => Some("Transaction declined (invalid configuration data)".to_string()),
        "800.100.192" => Some("Transaction declined (invalid CVV, Amount has still been reserved on the customer's card and will be released in a few business days.)".to_string()),
        "800.100.195" => Some("Transaction declined (UserAccount Number/ID unknown)".to_string()),
        "800.100.200" => Some("Refer to Payer due to reason not specified".to_string()),
        "800.100.201" => Some("Account or Bank Details Incorrect".to_string()),
        "800.100.202" => Some("Account Closed".to_string()),
        "800.100.203" => Some("Insufficient Funds".to_string()),
        "800.100.204" => Some("Mandate Expired".to_string()),
        "800.100.205" => Some("Mandate Discarded".to_string()),
        "800.100.402" => Some("CC/bank account holder not valid".to_string()),
        "800.100.403" => Some("Transaction declined (revocation of authorisation order)".to_string()),
        "800.100.500" => Some("The card holder has advised his bank to stop this recurring payment".to_string()),
        "800.100.501" => Some("Card holder has advised his bank to stop all recurring payments for this merchant".to_string()),
        "081" => Some("Approved by Issuer".to_string()),
        "102" => Some("Suspected Fraud".to_string()),
        "103" => Some("Customer Authentication Required".to_string()),
        "104" => Some("Restricted Card".to_string()),
        "106" => Some("Allowable PIN Tries Exceeded".to_string()),
        "117" => Some("Incorrect PIN".to_string()),
        "118" => Some("Cycle Range Suspended".to_string()),
        "120" => Some("Transaction Not Permitted To Originator".to_string()),
        "124" => Some("Violation Of Law".to_string()),
        "125" => Some("Card Not Effective".to_string()),
        "129" => Some("Suspected Counterfeit Card".to_string()),
        "163" => Some("Security Violations".to_string()),
        "182" => Some("Decline Given By Issuer".to_string()),
        "192" => Some("Restricted Merchant".to_string()),
        "197" => Some("Card Account Verification Failed".to_string()),
        "198" => Some("TVR or CVR Validation Failed".to_string()),
        "201" => Some("Expired Card".to_string()),
        "202" => Some("Suspected Fraud".to_string()),
        "204" => Some("Restricted Card".to_string()),
        "206" => Some("Allowable Pin Tries Exceeded".to_string()),
        "207" => Some("Special Conditions".to_string()),
        "208" => Some("Lost Card".to_string()),
        "209" => Some("Stolen Card".to_string()),
        "210" => Some("Suspected Counterfeit Card".to_string()),
        _ => None,
    }
}

impl TryFrom<AirwallexWebhookEventType> for api_models::webhooks::IncomingWebhookEvent {
    type Error = errors::ConnectorError;
    fn try_from(value: AirwallexWebhookEventType) -> Result<Self, Self::Error> {
        Ok(match value {
            AirwallexWebhookEventType::PaymentAttemptFailedToProcess => Self::PaymentIntentFailure,
            AirwallexWebhookEventType::PaymentAttemptAuthorized => Self::PaymentIntentSuccess,
            AirwallexWebhookEventType::RefundSucceeded => Self::RefundSuccess,
            AirwallexWebhookEventType::RefundFailed => Self::RefundFailure,
            AirwallexWebhookEventType::DisputeAccepted
            | AirwallexWebhookEventType::DisputePreChargebackAccepted => Self::DisputeAccepted,
            AirwallexWebhookEventType::DisputeRespondedByMerchant => Self::DisputeChallenged,
            AirwallexWebhookEventType::DisputeWon | AirwallexWebhookEventType::DisputeReversed => {
                Self::DisputeWon
            }
            AirwallexWebhookEventType::DisputeLost => Self::DisputeLost,
            AirwallexWebhookEventType::Unknown
            | AirwallexWebhookEventType::PaymentAttemptAuthenticationRedirected
            | AirwallexWebhookEventType::PaymentIntentCreated
            | AirwallexWebhookEventType::PaymentIntentRequiresPaymentMethod
            | AirwallexWebhookEventType::PaymentIntentCancelled
            | AirwallexWebhookEventType::PaymentIntentSucceeded
            | AirwallexWebhookEventType::PaymentIntentRequiresCapture
            | AirwallexWebhookEventType::PaymentIntentRequiresCustomerAction
            | AirwallexWebhookEventType::PaymentAttemptAuthorizationFailed
            | AirwallexWebhookEventType::PaymentAttemptCaptureRequested
            | AirwallexWebhookEventType::PaymentAttemptCaptureFailed
            | AirwallexWebhookEventType::PaymentAttemptAuthenticationFailed
            | AirwallexWebhookEventType::PaymentAttemptCancelled
            | AirwallexWebhookEventType::PaymentAttemptExpired
            | AirwallexWebhookEventType::PaymentAttemptRiskDeclined
            | AirwallexWebhookEventType::PaymentAttemptSettled
            | AirwallexWebhookEventType::PaymentAttemptPaid
            | AirwallexWebhookEventType::RefundReceived
            | AirwallexWebhookEventType::RefundAccepted
            | AirwallexWebhookEventType::DisputeRfiRespondedByMerchant
            | AirwallexWebhookEventType::DisputeReceivedByMerchant => Self::EventNotSupported,
        })
    }
}

impl From<AirwallexDisputeStage> for api_models::enums::DisputeStage {
    fn from(code: AirwallexDisputeStage) -> Self {
        match code {
            AirwallexDisputeStage::Rfi => Self::PreDispute,
            AirwallexDisputeStage::Dispute => Self::Dispute,
            AirwallexDisputeStage::Arbitration => Self::PreArbitration,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CustomerRequest {
    pub request_id: String,
    pub email: Option<Email>,
    pub phone_number: Option<Secret<String>>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub merchant_customer_id: id_type::CustomerId,
}

impl TryFrom<&types::ConnectorCustomerRouterData> for CustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            email: item.request.email.to_owned(),
            phone_number: item.request.phone.to_owned(),
            first_name: item.request.name.to_owned(),
            last_name: item.request.name.to_owned(),
            merchant_customer_id: item.customer_id.to_owned().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "customer_id",
                },
            )?,
        })
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AirwallexCustomerResponse {
    pub id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, AirwallexCustomerResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AirwallexCustomerResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new_with_customer_id(item.response.id),
            )),
            ..item.data
        })
    }
}
