use common_enums::enums;
use common_utils::{
    errors::ParsingError,
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
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
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
    device_data: DeviceData,
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
            PaymentMethodData::BankDebit(_)
            | PaymentMethodData::CardRedirect(_)
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
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("airwallex"),
                ))
            }
        }?;
        let device_data = get_device_data(item.router_data)?;

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

        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            payment_method,
            payment_method_options,
            return_url,
            device_data,
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
