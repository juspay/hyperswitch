use cards::CardNumber;
use common_enums::enums;
use common_utils::{pii::Email, request::Method, types::StringMajorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{
        BankDebitData, BankRedirectData, PayLaterData, PaymentMethodData, WalletData,
    },
    router_data::{ConnectorAuthType, ErrorResponse, PaymentMethodToken, RouterData},
    router_request_types::ResponseId,
    router_response_types::{
        ConnectorCustomerResponseData, MandateReference, PaymentsResponseData, RedirectForm,
        RefundsResponseData,
    },
    types,
};
use hyperswitch_interfaces::{consts, errors};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::{
        convert_amount, get_unimplemented_payment_method_error_message, AddressData,
        AddressDetailsData, BrowserInformationData, CardData as CardDataUtil, CustomerData,
        OrderDetailsWithAmountData, PaymentMethodTokenizationRequestData,
        PaymentsAuthorizeRequestData, PaymentsSetupMandateRequestData,
        RouterData as OtherRouterData,
    },
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
pub struct MollieRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for MollieRouterData<T> {
    fn from((amount, router_data): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MolliePaymentsRequest {
    amount: Amount,
    description: String,
    redirect_url: String,
    cancel_url: Option<String>,
    webhook_url: String,
    locale: Option<String>,
    #[serde(flatten)]
    payment_method_data: MolliePaymentMethodData,
    metadata: Option<MollieMetadata>,
    sequence_type: SequenceType,
    customer_id: Option<String>,
    capture_mode: Option<MollieCaptureMode>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Amount {
    currency: enums::Currency,
    value: StringMajorUnit,
}

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
#[serde(rename_all = "lowercase")]
pub enum MolliePaymentMethodData {
    Applepay(Box<ApplePayMethodData>),
    Eps,
    Giropay,
    Ideal(Box<IdealMethodData>),
    Paypal(Box<PaypalMethodData>),
    Sofort,
    Przelewy24(Box<Przelewy24MethodData>),
    Bancontact,
    CreditCard(Box<CreditCardMethodData>),
    DirectDebit(Box<DirectDebitMethodData>),
    Klarna(Box<KlarnaMethodData>),
    #[serde(untagged)]
    MandatePayment(Box<MandatePaymentMethodData>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayMethodData {
    apple_pay_payment_token: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdealMethodData {
    issuer: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaypalMethodData {
    billing_address: Option<Address>,
    shipping_address: Option<Address>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KlarnaMethodData {
    billing_address: Address,
    lines: Vec<MollieLinesItems>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MollieLinesItems {
    description: String,
    quantity: i32,
    quantity_unit: Option<String>,
    unit_price: OrderItemUnitPrice,
    total_amount: OrderItemUnitPrice,
    discount_amount: Option<OrderItemUnitPrice>,
    sku: Option<String>,
    image_url: Option<String>,
}

impl TryFrom<(types::OrderDetailsWithAmount, enums::Currency)> for MollieLinesItems {
    type Error = Error;
    fn try_from(
        (order_details, currency): (types::OrderDetailsWithAmount, enums::Currency),
    ) -> Result<Self, Self::Error> {
        let description = order_details.get_order_description()?;
        let quantity = i32::from(order_details.get_order_quantity());
        let quantity_unit = order_details.get_optional_order_quantity_unit();
        let sku = order_details.get_optional_sku();
        let image_url = order_details.get_optional_product_img_link();
        let mollie_converter = super::Mollie::new().amount_converter;
        let unit_price_value = convert_amount(
            mollie_converter,
            order_details.get_order_unit_price(),
            currency,
        )?;

        let discount_amount_value = order_details
            .get_optional_unit_discount_amount()
            .map(|unit_discount_amount| {
                convert_amount(mollie_converter, unit_discount_amount, currency)
            })
            .transpose()?;

        let total_amount_value = convert_amount(
            mollie_converter,
            order_details.get_order_total_amount()?,
            currency,
        )?;

        Ok(Self {
            description,
            quantity,
            quantity_unit,
            unit_price: OrderItemUnitPrice {
                currency,
                value: unit_price_value,
            },
            total_amount: OrderItemUnitPrice {
                currency,
                value: total_amount_value,
            },
            discount_amount: discount_amount_value
                .map(|value| OrderItemUnitPrice { currency, value }),
            sku,
            image_url,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItemUnitPrice {
    currency: enums::Currency,
    value: StringMajorUnit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Przelewy24MethodData {
    billing_email: Option<Email>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectDebitMethodData {
    consumer_name: Option<Secret<String>>,
    consumer_account: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditCardMethodData {
    billing_address: Option<Address>,
    shipping_address: Option<Address>,
    card_token: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandatePaymentMethodData {
    mandate_id: Secret<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SequenceType {
    #[default]
    Oneoff,
    First,
    Recurring,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub street_and_number: Secret<String>,
    pub postal_code: Secret<String>,
    pub city: String,
    pub region: Option<Secret<String>>,
    pub country: api_models::enums::CountryAlpha2,
    pub given_name: Option<Secret<String>>,
    pub family_name: Option<Secret<String>>,
    pub email: Option<Email>,
}

impl Address {
    fn validate_and_build_klarna_billing_address(
        address_details: hyperswitch_domain_models::address::Address,
    ) -> Result<Self, Error> {
        let address = address_details.address.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "Billing Address details for Klarna",
            },
        )?;

        Ok(Self {
            street_and_number: address.get_combined_address_line()?,
            postal_code: address.get_zip()?.to_owned(),
            city: address.get_city()?.to_owned(),
            region: address.get_optional_state(),
            country: address.get_country()?.to_owned(),
            given_name: Some(address.get_first_name()?.clone()),
            family_name: Some(address.get_last_name()?.clone()),
            email: Some(address_details.get_email()?.clone()),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MollieMetadata {
    pub order_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MollieCaptureMode {
    Automatic,
    Manual,
}

#[derive(Debug, Serialize)]
pub struct MollieCustomerRequest {
    pub name: Option<Secret<String>>,
    pub email: Option<Email>,
}

impl TryFrom<&types::ConnectorCustomerRouterData> for MollieCustomerRequest {
    type Error = Error;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            name: item.request.get_optional_name(),
            email: item.request.get_optional_email(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MollieCustomerResponse {
    pub id: String,
    pub name: Option<Secret<String>>,
    pub email: Option<Email>,
}

impl<F, T> TryFrom<ResponseRouterData<F, MollieCustomerResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, MollieCustomerResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            connector_customer: Some(item.response.id.clone()),
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new_with_customer_id(item.response.id),
            )),
            ..item.data
        })
    }
}

impl TryFrom<&MollieRouterData<&types::SetupMandateRouterData>> for MolliePaymentsRequest {
    type Error = Error;
    fn try_from(
        item: &MollieRouterData<&types::SetupMandateRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_method_data = match &item.router_data.request.payment_method_data {
            PaymentMethodData::Card(_) => {
                let pm_token = item.router_data.get_payment_method_token()?;
                MolliePaymentMethodData::CreditCard(Box::new(CreditCardMethodData {
                    billing_address: get_address_details(
                        item.router_data
                            .get_optional_billing()
                            .and_then(|billing| billing.address.as_ref()),
                    )?,
                    shipping_address: get_address_details(
                        item.router_data
                            .get_optional_shipping()
                            .and_then(|shipping| shipping.address.as_ref()),
                    )?,
                    card_token: Some(match pm_token {
                        PaymentMethodToken::Token(token) => token,
                        PaymentMethodToken::ApplePayDecrypt(_) => Err(
                            unimplemented_payment_method!("Apple Pay", "Simplified", "Mollie"),
                        )?,
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "Mollie"))?
                        }
                        PaymentMethodToken::GooglePayDecrypt(_) => {
                            Err(unimplemented_payment_method!("Google Pay", "Mollie"))?
                        }
                    }),
                }))
            }
            PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_)
            | PaymentMethodData::DecryptedWalletTokenDetailsForNetworkTransactionId(_)
            | PaymentMethodData::NetworkTokenDetailsForNetworkTransactionId(_)
            | PaymentMethodData::CardWithLimitedDetails(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::MobilePayment(_) => {
                return Err(errors::ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("Mollie"),
                )
                .into());
            }
        };

        Ok(Self {
            amount: Amount {
                currency: item.router_data.request.currency,
                value: item.amount.clone(),
            },
            description: item.router_data.get_description()?,
            redirect_url: item.router_data.request.get_router_return_url()?,
            cancel_url: None,
            /* webhook_url is a mandatory field. */
            webhook_url: "".to_string(),
            locale: None,
            payment_method_data,
            metadata: Some(MollieMetadata {
                order_id: item.router_data.connector_request_reference_id.clone(),
            }),
            sequence_type: SequenceType::First,
            capture_mode: None,
            customer_id: Some(item.router_data.get_connector_customer_id()?),
        })
    }
}

impl TryFrom<&MollieRouterData<&types::PaymentsAuthorizeRouterData>> for MolliePaymentsRequest {
    type Error = Error;
    fn try_from(
        item: &MollieRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.router_data.request.currency,
            value: item.amount.clone(),
        };
        let description = item.router_data.get_description()?;
        let redirect_url = item.router_data.request.get_router_return_url()?;

        let sequence_type = if item
            .router_data
            .request
            .is_customer_initiated_mandate_payment()
        {
            SequenceType::First
        } else if item.router_data.request.is_mandate_payment() {
            SequenceType::Recurring
        } else {
            SequenceType::Oneoff
        };

        let capture_mode = if sequence_type == SequenceType::Oneoff {
            Some(if item.router_data.request.is_auto_capture()? {
                MollieCaptureMode::Automatic
            } else {
                MollieCaptureMode::Manual
            })
        } else {
            None
        };

        let customer_id = if item.router_data.request.is_mandate_payment() {
            Some(item.router_data.get_connector_customer_id()?)
        } else {
            None
        };

        let payment_method_data = match &item.router_data.request.payment_method_data {
            PaymentMethodData::Card(_) => {
                let pm_token = item.router_data.get_payment_method_token()?;
                Ok(MolliePaymentMethodData::CreditCard(Box::new(
                    CreditCardMethodData {
                        billing_address: get_billing_details(item.router_data)?,
                        shipping_address: get_shipping_details(item.router_data)?,
                        card_token: Some(match pm_token {
                            PaymentMethodToken::Token(token) => token,
                            PaymentMethodToken::ApplePayDecrypt(_) => Err(
                                unimplemented_payment_method!("Apple Pay", "Simplified", "Mollie"),
                            )?,
                            PaymentMethodToken::PazeDecrypt(_) => {
                                Err(unimplemented_payment_method!("Paze", "Mollie"))?
                            }
                            PaymentMethodToken::GooglePayDecrypt(_) => {
                                Err(unimplemented_payment_method!("Google Pay", "Mollie"))?
                            }
                        }),
                    },
                )))
            }
            PaymentMethodData::PayLater(ref paylater_data) => {
                MolliePaymentMethodData::try_from((item.router_data, paylater_data))
            }
            PaymentMethodData::BankRedirect(ref redirect_data) => {
                MolliePaymentMethodData::try_from((item.router_data, redirect_data))
            }
            PaymentMethodData::Wallet(ref wallet_data) => {
                get_payment_method_for_wallet(item.router_data, wallet_data)
            }
            PaymentMethodData::BankDebit(ref directdebit_data) => {
                MolliePaymentMethodData::try_from((directdebit_data, item.router_data))
            }
            PaymentMethodData::MandatePayment => Ok(MolliePaymentMethodData::MandatePayment(
                Box::new(MandatePaymentMethodData {
                    mandate_id: item.router_data.request.get_connector_mandate_id()?.into(),
                }),
            )),
            _ => Err(errors::ConnectorError::NotImplemented("Payment Method".to_string()).into()),
        }?;
        Ok(Self {
            amount,
            description,
            redirect_url,
            cancel_url: None,
            /* webhook_url is a mandatory field.
            But we can't support webhook in our core hence keeping it as empty string */
            webhook_url: "".to_string(),
            locale: None,
            payment_method_data,
            metadata: Some(MollieMetadata {
                order_id: item.router_data.connector_request_reference_id.clone(),
            }),
            sequence_type,
            capture_mode,
            customer_id,
        })
    }
}

impl TryFrom<(&types::PaymentsAuthorizeRouterData, &PayLaterData)> for MolliePaymentMethodData {
    type Error = Error;
    fn try_from(
        (item, value): (&types::PaymentsAuthorizeRouterData, &PayLaterData),
    ) -> Result<Self, Self::Error> {
        match value {
            PayLaterData::KlarnaRedirect {} => {
                let billing_address = Address::validate_and_build_klarna_billing_address(
                    item.get_billing()?.clone(),
                )?;

                let lines = item
                    .request
                    .get_order_details()?
                    .into_iter()
                    .map(|order_detail| {
                        MollieLinesItems::try_from((order_detail, item.request.currency))
                    })
                    .collect::<Result<Vec<MollieLinesItems>, Error>>()?;

                Ok(Self::Klarna(Box::new(KlarnaMethodData {
                    billing_address,
                    lines,
                })))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl TryFrom<(&types::PaymentsAuthorizeRouterData, &BankRedirectData)> for MolliePaymentMethodData {
    type Error = Error;
    fn try_from(
        (item, value): (&types::PaymentsAuthorizeRouterData, &BankRedirectData),
    ) -> Result<Self, Self::Error> {
        match value {
            BankRedirectData::Eps { .. } => Ok(Self::Eps),
            BankRedirectData::Giropay { .. } => Ok(Self::Giropay),
            BankRedirectData::Ideal { .. } => {
                Ok(Self::Ideal(Box::new(IdealMethodData {
                    // To do if possible this should be from the payment request
                    issuer: None,
                })))
            }
            BankRedirectData::Sofort { .. } => Ok(Self::Sofort),
            BankRedirectData::Przelewy24 { .. } => {
                Ok(Self::Przelewy24(Box::new(Przelewy24MethodData {
                    billing_email: item.get_optional_billing_email(),
                })))
            }
            BankRedirectData::BancontactCard { .. } => Ok(Self::Bancontact),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl TryFrom<(&BankDebitData, &types::PaymentsAuthorizeRouterData)> for MolliePaymentMethodData {
    type Error = Error;
    fn try_from(
        (bank_debit_data, item): (&BankDebitData, &types::PaymentsAuthorizeRouterData),
    ) -> Result<Self, Self::Error> {
        match bank_debit_data {
            BankDebitData::SepaBankDebit { iban, .. } => {
                Ok(Self::DirectDebit(Box::new(DirectDebitMethodData {
                    consumer_name: item.get_optional_billing_full_name(),
                    consumer_account: iban.clone(),
                })))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MollieCardTokenRequest {
    card_holder: Secret<String>,
    card_number: CardNumber,
    card_cvv: Secret<String>,
    card_expiry_date: Secret<String>,
    locale: String,
    testmode: bool,
    profile_token: Secret<String>,
}

impl TryFrom<&types::TokenizationRouterData> for MollieCardTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(ccard) => {
                let auth = MollieAuthType::try_from(&item.connector_auth_type)?;
                let card_holder = item
                    .get_optional_billing_full_name()
                    .unwrap_or(Secret::new("".to_string()));
                let card_number = ccard.card_number.clone();
                let card_expiry_date =
                    ccard.get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned())?;
                let card_cvv = ccard.card_cvc;
                let locale = item.request.get_browser_info()?.get_language()?;
                let testmode =
                    item.test_mode
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "test_mode",
                        })?;
                let profile_token = auth
                    .profile_token
                    .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
                Ok(Self {
                    card_holder,
                    card_number,
                    card_cvv,
                    card_expiry_date,
                    locale,
                    testmode,
                    profile_token,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))?,
        }
    }
}

fn get_payment_method_for_wallet(
    item: &types::PaymentsAuthorizeRouterData,
    wallet_data: &WalletData,
) -> Result<MolliePaymentMethodData, Error> {
    match wallet_data {
        WalletData::PaypalRedirect { .. } => Ok(MolliePaymentMethodData::Paypal(Box::new(
            PaypalMethodData {
                billing_address: get_billing_details(item)?,
                shipping_address: get_shipping_details(item)?,
            },
        ))),
        WalletData::ApplePay(applepay_wallet_data) => {
            let apple_pay_encrypted_data = applepay_wallet_data
                .payment_data
                .get_encrypted_apple_pay_payment_data_mandatory()
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "Apple pay encrypted data",
                })?;
            Ok(MolliePaymentMethodData::Applepay(Box::new(
                ApplePayMethodData {
                    apple_pay_payment_token: Secret::new(apple_pay_encrypted_data.to_owned()),
                },
            )))
        }
        _ => Err(errors::ConnectorError::NotImplemented("Payment Method".to_string()).into()),
    }
}

fn get_shipping_details(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<Option<Address>, Error> {
    let shipping_address = item
        .get_optional_shipping()
        .and_then(|shipping| shipping.address.as_ref());
    get_address_details(shipping_address)
}

fn get_billing_details(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<Option<Address>, Error> {
    let billing_address = item
        .get_optional_billing()
        .and_then(|billing| billing.address.as_ref());
    get_address_details(billing_address)
}

fn get_address_details(
    address: Option<&hyperswitch_domain_models::address::AddressDetails>,
) -> Result<Option<Address>, Error> {
    let address_details = match address {
        Some(address) => {
            let street_and_number = address.get_combined_address_line()?;
            let postal_code = address.get_zip()?.to_owned();
            let city = address.get_city()?.to_owned();
            let region = None;
            let country = address.get_country()?.to_owned();
            let given_name = address.get_optional_first_name();
            let family_name = address.get_optional_last_name();
            Some(Address {
                street_and_number,
                postal_code,
                city,
                region,
                country,
                given_name,
                family_name,
                email: None,
            })
        }
        None => None,
    };
    Ok(address_details)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MolliePaymentsResponse {
    pub resource: String,
    pub id: String,
    pub amount: Amount,
    pub description: Option<String>,
    pub metadata: Option<MollieMetadata>,
    pub status: MolliePaymentStatus,
    pub is_cancelable: Option<bool>,
    pub sequence_type: Option<SequenceType>,
    pub redirect_url: Option<String>,
    pub webhook_url: Option<String>,
    #[serde(rename = "_links")]
    pub links: Links,
    pub mandate_id: Option<Secret<String>>,
    pub payment_id: Option<String>,
    pub details: Option<MolliePaymentDetails>,
}

/// Details object containing failure information for failed payments
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MolliePaymentDetails {
    pub failure_reason: Option<String>,
    pub failure_message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MolliePaymentStatus {
    Open,
    Canceled,
    #[default]
    Pending,
    Authorized,
    Expired,
    Failed,
    Paid,
}

impl From<MolliePaymentStatus> for enums::AttemptStatus {
    fn from(item: MolliePaymentStatus) -> Self {
        match item {
            MolliePaymentStatus::Paid => Self::Charged,
            MolliePaymentStatus::Failed => Self::Failure,
            MolliePaymentStatus::Pending => Self::Pending,
            MolliePaymentStatus::Open => Self::AuthenticationPending,
            MolliePaymentStatus::Canceled => Self::Voided,
            MolliePaymentStatus::Authorized => Self::Authorized,
            MolliePaymentStatus::Expired => Self::Failure,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Link {
    href: Url,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Links {
    #[serde(rename = "self")]
    self_: Option<Link>,
    checkout: Option<Link>,
    dashboard: Option<Link>,
    documentation: Option<Link>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardDetails {
    pub card_number: Secret<String>,
    pub card_holder: Secret<String>,
    pub card_expiry_date: Secret<String>,
    pub card_cvv: Secret<String>,
}

pub struct MollieAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) profile_token: Option<Secret<String>>,
}

impl TryFrom<&ConnectorAuthType> for MollieAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
                profile_token: None,
            }),
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                profile_token: Some(key1.to_owned()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MollieCardTokenResponse {
    card_token: Secret<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, MollieCardTokenResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MollieCardTokenResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Pending,
            payment_method_token: Some(PaymentMethodToken::Token(item.response.card_token.clone())),
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.card_token.expose(),
            }),
            ..item.data
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, MolliePaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, MolliePaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.status.clone());

        // Handle failed payments: extract error details from the details object
        // Mollie returns 2xx but with status "failed" when payment fails after 3DS authentication
        if crate::utils::is_payment_failure(status) {
            let (failure_reason, failure_message) = item
                .response
                .details
                .as_ref()
                .map(|details| {
                    (
                        details.failure_reason.clone(),
                        details.failure_message.clone(),
                    )
                })
                .unwrap_or((None, None));

            let error_code = failure_reason
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string());
            let error_message = failure_message
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string());

            return Ok(Self {
                status,
                response: Err(ErrorResponse {
                    status_code: item.http_code,
                    code: error_code,
                    message: error_message.clone(),
                    reason: Some(error_message),
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.id),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                    connector_response_reference_id: None,
                }),
                ..item.data
            });
        }

        let url = item
            .response
            .links
            .checkout
            .map(|link| RedirectForm::from((link.href, Method::Get)));

        let mandate_reference = item
            .response
            .mandate_id
            .as_ref()
            .map(|id| MandateReference {
                connector_mandate_id: Some(id.clone().expose()),
                payment_method_id: None,
                mandate_metadata: None,
                connector_mandate_request_reference_id: None,
            });
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response
                        .payment_id
                        .unwrap_or_else(|| item.response.id.clone()),
                ),
                redirection_data: Box::new(url),
                mandate_reference: Box::new(mandate_reference),
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

#[derive(Default, Debug, Serialize)]
pub struct MollieCaptureRequest {
    pub amount: Option<Amount>,
    pub description: String,
}

impl TryFrom<&MollieRouterData<&types::PaymentsCaptureRouterData>> for MollieCaptureRequest {
    type Error = Error;
    fn try_from(
        item: &MollieRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: Some(Amount {
                value: item.amount.clone(),
                currency: item.router_data.request.currency,
            }),
            description: item.router_data.get_description()?,
        })
    }
}

// REFUND :
#[derive(Default, Debug, Serialize)]
pub struct MollieRefundRequest {
    amount: Amount,
    description: Option<String>,
    metadata: Option<MollieMetadata>,
}

impl<F> TryFrom<&MollieRouterData<&types::RefundsRouterData<F>>> for MollieRefundRequest {
    type Error = Error;
    fn try_from(
        item: &MollieRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.router_data.request.currency,
            value: item.amount.clone(),
        };
        Ok(Self {
            amount,
            description: item.router_data.request.reason.to_owned(),
            metadata: Some(MollieMetadata {
                order_id: item.router_data.request.refund_id.clone(),
            }),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    resource: String,
    id: String,
    amount: Amount,
    settlement_id: Option<String>,
    settlement_amount: Option<Amount>,
    status: MollieRefundStatus,
    description: Option<String>,
    metadata: Option<MollieMetadata>,
    payment_id: String,
    #[serde(rename = "_links")]
    links: Links,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MollieRefundStatus {
    Queued,
    #[default]
    Pending,
    Processing,
    Refunded,
    Failed,
    Canceled,
}

impl From<MollieRefundStatus> for enums::RefundStatus {
    fn from(item: MollieRefundStatus) -> Self {
        match item {
            MollieRefundStatus::Queued
            | MollieRefundStatus::Pending
            | MollieRefundStatus::Processing => Self::Pending,
            MollieRefundStatus::Refunded => Self::Success,
            MollieRefundStatus::Failed | MollieRefundStatus::Canceled => Self::Failure,
        }
    }
}

impl<T> TryFrom<RefundsResponseRouterData<T, RefundResponse>> for types::RefundsRouterData<T> {
    type Error = Error;
    fn try_from(item: RefundsResponseRouterData<T, RefundResponse>) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MollieErrorResponse {
    pub status: u16,
    pub title: Option<String>,
    pub detail: String,
    pub field: Option<String>,
}
