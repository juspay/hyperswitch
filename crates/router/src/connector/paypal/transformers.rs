use api_models::enums;
use base64::Engine;
#[cfg(feature = "payouts")]
use common_utils::pii::Email;
use common_utils::{errors::CustomResult, types::StringMajorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::router_response_types::MandateReference;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;

use crate::{
    connector::utils::{
        self, missing_field_err, to_connector_meta, AccessTokenRequestInfo, AddressDetailsData,
        CardData, PaymentsAuthorizeRequestData, PaymentsPostSessionTokensRequestData, RouterData,
    },
    consts,
    core::errors,
    services,
    types::{
        self, api, domain,
        storage::enums as storage_enums,
        transformers::{ForeignFrom, ForeignTryFrom},
        ConnectorAuthType, VerifyWebhookSourceResponseData,
    },
};

#[derive(Debug, Serialize)]
pub struct PaypalRouterData<T> {
    pub amount: StringMajorUnit,
    pub shipping_cost: Option<StringMajorUnit>,
    pub order_tax_amount: Option<StringMajorUnit>,
    pub order_amount: Option<StringMajorUnit>,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        StringMajorUnit,
        Option<StringMajorUnit>,
        Option<StringMajorUnit>,
        Option<StringMajorUnit>,
        T,
    )> for PaypalRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (amount, shipping_cost, order_tax_amount, order_amount, item): (
            StringMajorUnit,
            Option<StringMajorUnit>,
            Option<StringMajorUnit>,
            Option<StringMajorUnit>,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            shipping_cost,
            order_tax_amount,
            order_amount,
            router_data: item,
        })
    }
}

mod webhook_headers {
    pub const PAYPAL_TRANSMISSION_ID: &str = "paypal-transmission-id";
    pub const PAYPAL_TRANSMISSION_TIME: &str = "paypal-transmission-time";
    pub const PAYPAL_TRANSMISSION_SIG: &str = "paypal-transmission-sig";
    pub const PAYPAL_CERT_URL: &str = "paypal-cert-url";
    pub const PAYPAL_AUTH_ALGO: &str = "paypal-auth-algo";
}
pub mod auth_headers {
    pub const PAYPAL_PARTNER_ATTRIBUTION_ID: &str = "PayPal-Partner-Attribution-Id";
    pub const PREFER: &str = "Prefer";
    pub const PAYPAL_REQUEST_ID: &str = "PayPal-Request-Id";
    pub const PAYPAL_AUTH_ASSERTION: &str = "PayPal-Auth-Assertion";
}

const ORDER_QUANTITY: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaypalPaymentIntent {
    Capture,
    Authorize,
    Authenticate,
}

#[derive(Default, Debug, Clone, Serialize, Eq, PartialEq, Deserialize)]
pub struct OrderAmount {
    pub currency_code: storage_enums::Currency,
    pub value: StringMajorUnit,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct OrderRequestAmount {
    pub currency_code: storage_enums::Currency,
    pub value: StringMajorUnit,
    pub breakdown: AmountBreakdown,
}

impl From<&PaypalRouterData<&types::PaymentsAuthorizeRouterData>> for OrderRequestAmount {
    fn from(item: &PaypalRouterData<&types::PaymentsAuthorizeRouterData>) -> Self {
        Self {
            currency_code: item.router_data.request.currency,
            value: item.amount.clone(),
            breakdown: AmountBreakdown {
                item_total: OrderAmount {
                    currency_code: item.router_data.request.currency,
                    value: item.amount.clone(),
                },
                tax_total: None,
                shipping: Some(OrderAmount {
                    currency_code: item.router_data.request.currency,
                    value: item
                        .shipping_cost
                        .clone()
                        .unwrap_or(StringMajorUnit::zero()),
                }),
            },
        }
    }
}

impl TryFrom<&PaypalRouterData<&types::PaymentsPostSessionTokensRouterData>>
    for OrderRequestAmount
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::PaymentsPostSessionTokensRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            currency_code: item.router_data.request.currency,
            value: item.amount.clone(),
            breakdown: AmountBreakdown {
                item_total: OrderAmount {
                    currency_code: item.router_data.request.currency,
                    value: item.order_amount.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "order_amount",
                        },
                    )?,
                },
                tax_total: None,
                shipping: Some(OrderAmount {
                    currency_code: item.router_data.request.currency,
                    value: item
                        .shipping_cost
                        .clone()
                        .unwrap_or(StringMajorUnit::zero()),
                }),
            },
        })
    }
}

impl TryFrom<&PaypalRouterData<&types::SdkSessionUpdateRouterData>> for OrderRequestAmount {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::SdkSessionUpdateRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            currency_code: item.router_data.request.currency,
            value: item.amount.clone(),
            breakdown: AmountBreakdown {
                item_total: OrderAmount {
                    currency_code: item.router_data.request.currency,
                    value: item.order_amount.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "order_amount",
                        },
                    )?,
                },
                tax_total: Some(OrderAmount {
                    currency_code: item.router_data.request.currency,
                    value: item.order_tax_amount.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "order_tax_amount",
                        },
                    )?,
                }),
                shipping: Some(OrderAmount {
                    currency_code: item.router_data.request.currency,
                    value: item
                        .shipping_cost
                        .clone()
                        .unwrap_or(StringMajorUnit::zero()),
                }),
            },
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AmountBreakdown {
    item_total: OrderAmount,
    tax_total: Option<OrderAmount>,
    shipping: Option<OrderAmount>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PurchaseUnitRequest {
    reference_id: Option<String>, //reference for an item in purchase_units
    invoice_id: Option<String>, //The API caller-provided external invoice number for this order. Appears in both the payer's transaction history and the emails that the payer receives.
    custom_id: Option<String>,  //Used to reconcile client transactions with PayPal transactions.
    amount: OrderRequestAmount,
    #[serde(skip_serializing_if = "Option::is_none")]
    payee: Option<Payee>,
    shipping: Option<ShippingAddress>,
    items: Vec<ItemDetails>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct Payee {
    merchant_id: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ItemDetails {
    name: String,
    quantity: u16,
    unit_amount: OrderAmount,
    tax: Option<OrderAmount>,
}

impl From<&PaypalRouterData<&types::PaymentsAuthorizeRouterData>> for ItemDetails {
    fn from(item: &PaypalRouterData<&types::PaymentsAuthorizeRouterData>) -> Self {
        Self {
            name: format!(
                "Payment for invoice {}",
                item.router_data.connector_request_reference_id
            ),
            quantity: ORDER_QUANTITY,
            unit_amount: OrderAmount {
                currency_code: item.router_data.request.currency,
                value: item.amount.clone(),
            },
            tax: None,
        }
    }
}

impl TryFrom<&PaypalRouterData<&types::PaymentsPostSessionTokensRouterData>> for ItemDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::PaymentsPostSessionTokensRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            name: format!(
                "Payment for invoice {}",
                item.router_data.connector_request_reference_id
            ),
            quantity: ORDER_QUANTITY,
            unit_amount: OrderAmount {
                currency_code: item.router_data.request.currency,
                value: item.order_amount.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "order_amount",
                    },
                )?,
            },
            tax: None,
        })
    }
}

impl TryFrom<&PaypalRouterData<&types::SdkSessionUpdateRouterData>> for ItemDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::SdkSessionUpdateRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            name: format!(
                "Payment for invoice {}",
                item.router_data.connector_request_reference_id
            ),
            quantity: ORDER_QUANTITY,
            unit_amount: OrderAmount {
                currency_code: item.router_data.request.currency,
                value: item.order_amount.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "order_amount",
                    },
                )?,
            },
            tax: Some(OrderAmount {
                currency_code: item.router_data.request.currency,
                value: item.order_tax_amount.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "order_tax_amount",
                    },
                )?,
            }),
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq, Deserialize)]
pub struct Address {
    address_line_1: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    country_code: enums::CountryAlpha2,
    admin_area_2: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ShippingAddress {
    address: Option<Address>,
    name: Option<ShippingName>,
}

impl From<&PaypalRouterData<&types::PaymentsAuthorizeRouterData>> for ShippingAddress {
    fn from(item: &PaypalRouterData<&types::PaymentsAuthorizeRouterData>) -> Self {
        Self {
            address: get_address_info(item.router_data.get_optional_shipping()),
            name: Some(ShippingName {
                full_name: item
                    .router_data
                    .get_optional_shipping()
                    .and_then(|inner_data| inner_data.address.as_ref())
                    .and_then(|inner_data| inner_data.first_name.clone()),
            }),
        }
    }
}

impl From<&PaypalRouterData<&types::PaymentsPostSessionTokensRouterData>> for ShippingAddress {
    fn from(item: &PaypalRouterData<&types::PaymentsPostSessionTokensRouterData>) -> Self {
        Self {
            address: get_address_info(item.router_data.get_optional_shipping()),
            name: Some(ShippingName {
                full_name: item
                    .router_data
                    .get_optional_shipping()
                    .and_then(|inner_data| inner_data.address.as_ref())
                    .and_then(|inner_data| inner_data.first_name.clone()),
            }),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct PaypalUpdateOrderRequest(Vec<Operation>);

impl PaypalUpdateOrderRequest {
    pub fn get_inner_value(self) -> Vec<Operation> {
        self.0
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Operation {
    pub op: PaypalOperationType,
    pub path: String,
    pub value: Value,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PaypalOperationType {
    Add,
    Remove,
    Replace,
    Move,
    Copy,
    Test,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Value {
    Amount(OrderRequestAmount),
    Items(Vec<ItemDetails>),
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ShippingName {
    full_name: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub struct CardRequestStruct {
    billing_address: Option<Address>,
    expiry: Option<Secret<String>>,
    name: Option<Secret<String>>,
    number: Option<cards::CardNumber>,
    security_code: Option<Secret<String>>,
    attributes: Option<CardRequestAttributes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultStruct {
    vault_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CardRequest {
    CardRequestStruct(CardRequestStruct),
    CardVaultStruct(VaultStruct),
}
#[derive(Debug, Serialize)]
pub struct CardRequestAttributes {
    vault: Option<PaypalVault>,
    verification: Option<ThreeDsMethod>,
}

#[derive(Debug, Serialize)]
pub struct ThreeDsMethod {
    method: ThreeDsType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThreeDsType {
    ScaAlways,
}

#[derive(Debug, Serialize)]
pub struct RedirectRequest {
    name: Secret<String>,
    country_code: enums::CountryAlpha2,
    experience_context: ContextStruct,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContextStruct {
    return_url: Option<String>,
    cancel_url: Option<String>,
    user_action: Option<UserAction>,
    shipping_preference: ShippingPreference,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UserAction {
    #[serde(rename = "PAY_NOW")]
    PayNow,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ShippingPreference {
    #[serde(rename = "SET_PROVIDED_ADDRESS")]
    SetProvidedAddress,
    #[serde(rename = "GET_FROM_FILE")]
    GetFromFile,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaypalRedirectionRequest {
    PaypalRedirectionStruct(PaypalRedirectionStruct),
    PaypalVaultStruct(VaultStruct),
}

#[derive(Debug, Serialize)]
pub struct PaypalRedirectionStruct {
    experience_context: ContextStruct,
    attributes: Option<Attributes>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Attributes {
    vault: PaypalVault,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaypalRedirectionResponse {
    attributes: Option<AttributeResponse>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EpsRedirectionResponse {
    name: Option<Secret<String>>,
    country_code: Option<enums::CountryAlpha2>,
    bic: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdealRedirectionResponse {
    name: Option<Secret<String>>,
    country_code: Option<enums::CountryAlpha2>,
    bic: Option<Secret<String>>,
    iban_last_chars: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttributeResponse {
    vault: PaypalVaultResponse,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaypalVault {
    store_in_vault: StoreInVault,
    usage_type: UsageType,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaypalVaultResponse {
    id: String,
    status: String,
    customer: CustomerId,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomerId {
    id: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StoreInVault {
    OnSuccess,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UsageType {
    Merchant,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentSourceItem {
    Card(CardRequest),
    Paypal(PaypalRedirectionRequest),
    IDeal(RedirectRequest),
    Eps(RedirectRequest),
    Giropay(RedirectRequest),
    Sofort(RedirectRequest),
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CardVaultResponse {
    attributes: Option<AttributeResponse>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PaymentSourceItemResponse {
    Card(CardVaultResponse),
    Paypal(PaypalRedirectionResponse),
    Eps(EpsRedirectionResponse),
    Ideal(IdealRedirectionResponse),
}

#[derive(Debug, Serialize)]
pub struct PaypalPaymentsRequest {
    intent: PaypalPaymentIntent,
    purchase_units: Vec<PurchaseUnitRequest>,
    payment_source: Option<PaymentSourceItem>,
}

#[derive(Debug, Serialize)]
pub struct PaypalZeroMandateRequest {
    payment_source: ZeroMandateSourceItem,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ZeroMandateSourceItem {
    Card(CardMandateRequest),
    Paypal(PaypalMandateStruct),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaypalMandateStruct {
    experience_context: Option<ContextStruct>,
    usage_type: UsageType,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CardMandateRequest {
    billing_address: Option<Address>,
    expiry: Option<Secret<String>>,
    name: Option<Secret<String>>,
    number: Option<cards::CardNumber>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaypalSetupMandatesResponse {
    id: String,
    customer: Customer,
    payment_source: ZeroMandateSourceItem,
    links: Vec<PaypalLinks>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Customer {
    id: String,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, PaypalSetupMandatesResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PaypalSetupMandatesResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let info_response = item.response;

        let mandate_reference = Some(MandateReference {
            connector_mandate_id: Some(info_response.id.clone()),
            payment_method_id: None,
            mandate_metadata: None,
            connector_mandate_request_reference_id: None,
        });
        // https://developer.paypal.com/docs/api/payment-tokens/v3/#payment-tokens_create
        // If 201 status code, then order is captured, other status codes are handled by the error handler
        let status = if item.http_code == 201 {
            enums::AttemptStatus::Charged
        } else {
            enums::AttemptStatus::Failure
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(info_response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(info_response.id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}
impl TryFrom<&types::SetupMandateRouterData> for PaypalZeroMandateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SetupMandateRouterData) -> Result<Self, Self::Error> {
        let payment_source = match item.request.payment_method_data.clone() {
            domain::PaymentMethodData::Card(ccard) => {
                ZeroMandateSourceItem::Card(CardMandateRequest {
                    billing_address: get_address_info(item.get_optional_billing()),
                    expiry: Some(ccard.get_expiry_date_as_yyyymm("-")),
                    name: item.get_optional_billing_full_name(),
                    number: Some(ccard.card_number),
                })
            }

            domain::PaymentMethodData::Wallet(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::RealTimePayment(_)
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_)
            | domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_)
            | domain::PaymentMethodData::NetworkToken(_)
            | domain::PaymentMethodData::OpenBanking(_)
            | domain::PaymentMethodData::MobilePayment(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Paypal"),
                ))?
            }
        };

        Ok(Self { payment_source })
    }
}

fn get_address_info(
    payment_address: Option<&hyperswitch_domain_models::address::Address>,
) -> Option<Address> {
    let address = payment_address.and_then(|payment_address| payment_address.address.as_ref());
    match address {
        Some(address) => address.get_optional_country().map(|country| Address {
            country_code: country.to_owned(),
            address_line_1: address.line1.clone(),
            postal_code: address.zip.clone(),
            admin_area_2: address.city.clone(),
        }),
        None => None,
    }
}
fn get_payment_source(
    item: &types::PaymentsAuthorizeRouterData,
    bank_redirection_data: &domain::BankRedirectData,
) -> Result<PaymentSourceItem, error_stack::Report<errors::ConnectorError>> {
    match bank_redirection_data {
        domain::BankRedirectData::Eps { bank_name: _, .. } => {
            Ok(PaymentSourceItem::Eps(RedirectRequest {
                name: item.get_billing_full_name()?,
                country_code: item.get_billing_country()?,
                experience_context: ContextStruct {
                    return_url: item.request.complete_authorize_url.clone(),
                    cancel_url: item.request.complete_authorize_url.clone(),
                    shipping_preference: if item.get_optional_shipping_country().is_some() {
                        ShippingPreference::SetProvidedAddress
                    } else {
                        ShippingPreference::GetFromFile
                    },
                    user_action: Some(UserAction::PayNow),
                },
            }))
        }
        domain::BankRedirectData::Giropay { .. } => {
            Ok(PaymentSourceItem::Giropay(RedirectRequest {
                name: item.get_billing_full_name()?,
                country_code: item.get_billing_country()?,
                experience_context: ContextStruct {
                    return_url: item.request.complete_authorize_url.clone(),
                    cancel_url: item.request.complete_authorize_url.clone(),
                    shipping_preference: if item.get_optional_shipping_country().is_some() {
                        ShippingPreference::SetProvidedAddress
                    } else {
                        ShippingPreference::GetFromFile
                    },
                    user_action: Some(UserAction::PayNow),
                },
            }))
        }
        domain::BankRedirectData::Ideal { bank_name: _, .. } => {
            Ok(PaymentSourceItem::IDeal(RedirectRequest {
                name: item.get_billing_full_name()?,
                country_code: item.get_billing_country()?,
                experience_context: ContextStruct {
                    return_url: item.request.complete_authorize_url.clone(),
                    cancel_url: item.request.complete_authorize_url.clone(),
                    shipping_preference: if item.get_optional_shipping_country().is_some() {
                        ShippingPreference::SetProvidedAddress
                    } else {
                        ShippingPreference::GetFromFile
                    },
                    user_action: Some(UserAction::PayNow),
                },
            }))
        }
        domain::BankRedirectData::Sofort {
            preferred_language: _,
            ..
        } => Ok(PaymentSourceItem::Sofort(RedirectRequest {
            name: item.get_billing_full_name()?,
            country_code: item.get_billing_country()?,
            experience_context: ContextStruct {
                return_url: item.request.complete_authorize_url.clone(),
                cancel_url: item.request.complete_authorize_url.clone(),
                shipping_preference: if item.get_optional_shipping_country().is_some() {
                    ShippingPreference::SetProvidedAddress
                } else {
                    ShippingPreference::GetFromFile
                },
                user_action: Some(UserAction::PayNow),
            },
        })),
        domain::BankRedirectData::BancontactCard { .. }
        | domain::BankRedirectData::Blik { .. }
        | domain::BankRedirectData::Przelewy24 { .. } => {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Paypal"),
            )
            .into())
        }
        domain::BankRedirectData::Bizum {}
        | domain::BankRedirectData::Interac { .. }
        | domain::BankRedirectData::OnlineBankingCzechRepublic { .. }
        | domain::BankRedirectData::OnlineBankingFinland { .. }
        | domain::BankRedirectData::OnlineBankingPoland { .. }
        | domain::BankRedirectData::OnlineBankingSlovakia { .. }
        | domain::BankRedirectData::OpenBankingUk { .. }
        | domain::BankRedirectData::Trustly { .. }
        | domain::BankRedirectData::OnlineBankingFpx { .. }
        | domain::BankRedirectData::OnlineBankingThailand { .. }
        | domain::BankRedirectData::LocalBankRedirect {} => {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Paypal"),
            ))?
        }
    }
}

fn get_payee(auth_type: &PaypalAuthType) -> Option<Payee> {
    auth_type
        .get_credentials()
        .ok()
        .and_then(|credentials| credentials.get_payer_id())
        .map(|payer_id| Payee {
            merchant_id: payer_id,
        })
}

impl TryFrom<&PaypalRouterData<&types::PaymentsPostSessionTokensRouterData>>
    for PaypalPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::PaymentsPostSessionTokensRouterData>,
    ) -> Result<Self, Self::Error> {
        let intent = if item.router_data.request.is_auto_capture()? {
            PaypalPaymentIntent::Capture
        } else {
            PaypalPaymentIntent::Authorize
        };
        let paypal_auth: PaypalAuthType =
            PaypalAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payee = get_payee(&paypal_auth);

        let amount = OrderRequestAmount::try_from(item)?;
        let connector_request_reference_id =
            item.router_data.connector_request_reference_id.clone();

        let shipping_address = ShippingAddress::from(item);
        let item_details = vec![ItemDetails::try_from(item)?];

        let purchase_units = vec![PurchaseUnitRequest {
            reference_id: Some(connector_request_reference_id.clone()),
            custom_id: item.router_data.request.merchant_order_reference_id.clone(),
            invoice_id: Some(connector_request_reference_id),
            amount,
            payee,
            shipping: Some(shipping_address),
            items: item_details,
        }];
        let payment_source = Some(PaymentSourceItem::Paypal(
            PaypalRedirectionRequest::PaypalRedirectionStruct(PaypalRedirectionStruct {
                experience_context: ContextStruct {
                    return_url: item.router_data.request.router_return_url.clone(),
                    cancel_url: item.router_data.request.router_return_url.clone(),
                    shipping_preference: ShippingPreference::GetFromFile,
                    user_action: Some(UserAction::PayNow),
                },
                attributes: match item.router_data.request.setup_future_usage {
                    Some(setup_future_usage) => match setup_future_usage {
                        enums::FutureUsage::OffSession => Some(Attributes {
                            vault: PaypalVault {
                                store_in_vault: StoreInVault::OnSuccess,
                                usage_type: UsageType::Merchant,
                            },
                        }),
                        enums::FutureUsage::OnSession => None,
                    },
                    None => None,
                },
            }),
        ));

        Ok(Self {
            intent,
            purchase_units,
            payment_source,
        })
    }
}

impl TryFrom<&PaypalRouterData<&types::SdkSessionUpdateRouterData>> for PaypalUpdateOrderRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &PaypalRouterData<&types::SdkSessionUpdateRouterData>,
    ) -> Result<Self, Self::Error> {
        let op = PaypalOperationType::Replace;

        // Create separate paths for amount and items
        let reference_id = &item.router_data.connector_request_reference_id;

        let amount_path = format!("/purchase_units/@reference_id=='{}'/amount", reference_id);
        let items_path = format!("/purchase_units/@reference_id=='{}'/items", reference_id);

        let amount_value = Value::Amount(OrderRequestAmount::try_from(item)?);

        let items_value = Value::Items(vec![ItemDetails::try_from(item)?]);

        Ok(Self(vec![
            Operation {
                op: op.clone(),
                path: amount_path,
                value: amount_value,
            },
            Operation {
                op,
                path: items_path,
                value: items_value,
            },
        ]))
    }
}

impl TryFrom<&PaypalRouterData<&types::PaymentsAuthorizeRouterData>> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let paypal_auth: PaypalAuthType =
            PaypalAuthType::try_from(&item.router_data.connector_auth_type)?;
        let payee = get_payee(&paypal_auth);

        let amount = OrderRequestAmount::from(item);

        let intent = if item.router_data.request.is_auto_capture()? {
            PaypalPaymentIntent::Capture
        } else {
            PaypalPaymentIntent::Authorize
        };

        let connector_request_reference_id =
            item.router_data.connector_request_reference_id.clone();

        let shipping_address = ShippingAddress::from(item);
        let item_details = vec![ItemDetails::from(item)];

        let purchase_units = vec![PurchaseUnitRequest {
            reference_id: Some(connector_request_reference_id.clone()),
            custom_id: item.router_data.request.merchant_order_reference_id.clone(),
            invoice_id: Some(connector_request_reference_id),
            amount,
            payee,
            shipping: Some(shipping_address),
            items: item_details,
        }];

        match item.router_data.request.payment_method_data {
            domain::PaymentMethodData::Card(ref ccard) => {
                let card = item.router_data.request.get_card()?;
                let expiry = Some(card.get_expiry_date_as_yyyymm("-"));

                let verification = match item.router_data.auth_type {
                    enums::AuthenticationType::ThreeDs => Some(ThreeDsMethod {
                        method: ThreeDsType::ScaAlways,
                    }),
                    enums::AuthenticationType::NoThreeDs => None,
                };

                let payment_source = Some(PaymentSourceItem::Card(CardRequest::CardRequestStruct(
                    CardRequestStruct {
                        billing_address: get_address_info(item.router_data.get_optional_billing()),
                        expiry,
                        name: item.router_data.get_optional_billing_full_name(),
                        number: Some(ccard.card_number.clone()),
                        security_code: Some(ccard.card_cvc.clone()),
                        attributes: Some(CardRequestAttributes {
                            vault: match item.router_data.request.setup_future_usage {
                                Some(setup_future_usage) => match setup_future_usage {
                                    enums::FutureUsage::OffSession => Some(PaypalVault {
                                        store_in_vault: StoreInVault::OnSuccess,
                                        usage_type: UsageType::Merchant,
                                    }),

                                    enums::FutureUsage::OnSession => None,
                                },
                                None => None,
                            },
                            verification,
                        }),
                    },
                )));

                Ok(Self {
                    intent,
                    purchase_units,
                    payment_source,
                })
            }
            domain::PaymentMethodData::Wallet(ref wallet_data) => match wallet_data {
                domain::WalletData::PaypalRedirect(_) => {
                    let payment_source = Some(PaymentSourceItem::Paypal(
                        PaypalRedirectionRequest::PaypalRedirectionStruct(
                            PaypalRedirectionStruct {
                                experience_context: ContextStruct {
                                    return_url: item
                                        .router_data
                                        .request
                                        .complete_authorize_url
                                        .clone(),
                                    cancel_url: item
                                        .router_data
                                        .request
                                        .complete_authorize_url
                                        .clone(),
                                    shipping_preference: if item
                                        .router_data
                                        .get_optional_shipping()
                                        .is_some()
                                    {
                                        ShippingPreference::SetProvidedAddress
                                    } else {
                                        ShippingPreference::GetFromFile
                                    },
                                    user_action: Some(UserAction::PayNow),
                                },
                                attributes: match item.router_data.request.setup_future_usage {
                                    Some(setup_future_usage) => match setup_future_usage {
                                        enums::FutureUsage::OffSession => Some(Attributes {
                                            vault: PaypalVault {
                                                store_in_vault: StoreInVault::OnSuccess,
                                                usage_type: UsageType::Merchant,
                                            },
                                        }),
                                        enums::FutureUsage::OnSession => None,
                                    },
                                    None => None,
                                },
                            },
                        ),
                    ));

                    Ok(Self {
                        intent,
                        purchase_units,
                        payment_source,
                    })
                }
                domain::WalletData::PaypalSdk(_) => {
                    let payment_source = Some(PaymentSourceItem::Paypal(
                        PaypalRedirectionRequest::PaypalRedirectionStruct(
                            PaypalRedirectionStruct {
                                experience_context: ContextStruct {
                                    return_url: None,
                                    cancel_url: None,
                                    shipping_preference: ShippingPreference::GetFromFile,
                                    user_action: Some(UserAction::PayNow),
                                },
                                attributes: match item.router_data.request.setup_future_usage {
                                    Some(setup_future_usage) => match setup_future_usage {
                                        enums::FutureUsage::OffSession => Some(Attributes {
                                            vault: PaypalVault {
                                                store_in_vault: StoreInVault::OnSuccess,
                                                usage_type: UsageType::Merchant,
                                            },
                                        }),
                                        enums::FutureUsage::OnSession => None,
                                    },
                                    None => None,
                                },
                            },
                        ),
                    ));

                    Ok(Self {
                        intent,
                        purchase_units,
                        payment_source,
                    })
                }
                domain::WalletData::AliPayQr(_)
                | domain::WalletData::AliPayRedirect(_)
                | domain::WalletData::AliPayHkRedirect(_)
                | domain::WalletData::AmazonPayRedirect(_)
                | domain::WalletData::MomoRedirect(_)
                | domain::WalletData::KakaoPayRedirect(_)
                | domain::WalletData::GoPayRedirect(_)
                | domain::WalletData::GcashRedirect(_)
                | domain::WalletData::ApplePay(_)
                | domain::WalletData::ApplePayRedirect(_)
                | domain::WalletData::ApplePayThirdPartySdk(_)
                | domain::WalletData::DanaRedirect {}
                | domain::WalletData::GooglePay(_)
                | domain::WalletData::GooglePayRedirect(_)
                | domain::WalletData::GooglePayThirdPartySdk(_)
                | domain::WalletData::MbWayRedirect(_)
                | domain::WalletData::MobilePayRedirect(_)
                | domain::WalletData::SamsungPay(_)
                | domain::WalletData::TwintRedirect {}
                | domain::WalletData::VippsRedirect {}
                | domain::WalletData::TouchNGoRedirect(_)
                | domain::WalletData::WeChatPayRedirect(_)
                | domain::WalletData::WeChatPayQr(_)
                | domain::WalletData::CashappQr(_)
                | domain::WalletData::SwishQr(_)
                | domain::WalletData::Mifinity(_)
                | domain::WalletData::Paze(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Paypal"),
                ))?,
            },
            domain::PaymentMethodData::BankRedirect(ref bank_redirection_data) => {
                let bank_redirect_intent = if item.router_data.request.is_auto_capture()? {
                    PaypalPaymentIntent::Capture
                } else {
                    Err(errors::ConnectorError::FlowNotSupported {
                        flow: "Manual capture method for Bank Redirect".to_string(),
                        connector: "Paypal".to_string(),
                    })?
                };

                let payment_source =
                    Some(get_payment_source(item.router_data, bank_redirection_data)?);

                Ok(Self {
                    intent: bank_redirect_intent,
                    purchase_units,
                    payment_source,
                })
            }
            domain::PaymentMethodData::CardRedirect(ref card_redirect_data) => {
                Self::try_from(card_redirect_data)
            }
            domain::PaymentMethodData::PayLater(ref paylater_data) => Self::try_from(paylater_data),
            domain::PaymentMethodData::BankDebit(ref bank_debit_data) => {
                Self::try_from(bank_debit_data)
            }
            domain::PaymentMethodData::BankTransfer(ref bank_transfer_data) => {
                Self::try_from(bank_transfer_data.as_ref())
            }
            domain::PaymentMethodData::Voucher(ref voucher_data) => Self::try_from(voucher_data),
            domain::PaymentMethodData::GiftCard(ref giftcard_data) => {
                Self::try_from(giftcard_data.as_ref())
            }
            domain::PaymentMethodData::MandatePayment => {
                let payment_method_type = item
                    .router_data
                    .get_recurring_mandate_payment_data()?
                    .payment_method_type
                    .ok_or_else(missing_field_err("payment_method_type"))?;

                let connector_mandate_id = item.router_data.request.connector_mandate_id().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_mandate_id",
                    },
                )?;

                let payment_source = match payment_method_type {
                    enums::PaymentMethodType::Credit | enums::PaymentMethodType::Debit => Ok(Some(
                        PaymentSourceItem::Card(CardRequest::CardVaultStruct(VaultStruct {
                            vault_id: connector_mandate_id.into(),
                        })),
                    )),
                    enums::PaymentMethodType::Paypal => Ok(Some(PaymentSourceItem::Paypal(
                        PaypalRedirectionRequest::PaypalVaultStruct(VaultStruct {
                            vault_id: connector_mandate_id.into(),
                        }),
                    ))),
                    enums::PaymentMethodType::Ach
                    | enums::PaymentMethodType::Affirm
                    | enums::PaymentMethodType::AfterpayClearpay
                    | enums::PaymentMethodType::Alfamart
                    | enums::PaymentMethodType::AliPay
                    | enums::PaymentMethodType::AliPayHk
                    | enums::PaymentMethodType::Alma
                    | enums::PaymentMethodType::AmazonPay
                    | enums::PaymentMethodType::ApplePay
                    | enums::PaymentMethodType::Atome
                    | enums::PaymentMethodType::Bacs
                    | enums::PaymentMethodType::BancontactCard
                    | enums::PaymentMethodType::Becs
                    | enums::PaymentMethodType::Benefit
                    | enums::PaymentMethodType::Bizum
                    | enums::PaymentMethodType::Blik
                    | enums::PaymentMethodType::Boleto
                    | enums::PaymentMethodType::BcaBankTransfer
                    | enums::PaymentMethodType::BniVa
                    | enums::PaymentMethodType::BriVa
                    | enums::PaymentMethodType::CardRedirect
                    | enums::PaymentMethodType::CimbVa
                    | enums::PaymentMethodType::ClassicReward
                    | enums::PaymentMethodType::CryptoCurrency
                    | enums::PaymentMethodType::Cashapp
                    | enums::PaymentMethodType::Dana
                    | enums::PaymentMethodType::DanamonVa
                    | enums::PaymentMethodType::DirectCarrierBilling
                    | enums::PaymentMethodType::DuitNow
                    | enums::PaymentMethodType::Efecty
                    | enums::PaymentMethodType::Eps
                    | enums::PaymentMethodType::Fps
                    | enums::PaymentMethodType::Evoucher
                    | enums::PaymentMethodType::Giropay
                    | enums::PaymentMethodType::Givex
                    | enums::PaymentMethodType::GooglePay
                    | enums::PaymentMethodType::GoPay
                    | enums::PaymentMethodType::Gcash
                    | enums::PaymentMethodType::Ideal
                    | enums::PaymentMethodType::Interac
                    | enums::PaymentMethodType::Indomaret
                    | enums::PaymentMethodType::Klarna
                    | enums::PaymentMethodType::KakaoPay
                    | enums::PaymentMethodType::LocalBankRedirect
                    | enums::PaymentMethodType::MandiriVa
                    | enums::PaymentMethodType::Knet
                    | enums::PaymentMethodType::MbWay
                    | enums::PaymentMethodType::MobilePay
                    | enums::PaymentMethodType::Momo
                    | enums::PaymentMethodType::MomoAtm
                    | enums::PaymentMethodType::Multibanco
                    | enums::PaymentMethodType::OnlineBankingThailand
                    | enums::PaymentMethodType::OnlineBankingCzechRepublic
                    | enums::PaymentMethodType::OnlineBankingFinland
                    | enums::PaymentMethodType::OnlineBankingFpx
                    | enums::PaymentMethodType::OnlineBankingPoland
                    | enums::PaymentMethodType::OnlineBankingSlovakia
                    | enums::PaymentMethodType::OpenBankingPIS
                    | enums::PaymentMethodType::Oxxo
                    | enums::PaymentMethodType::PagoEfectivo
                    | enums::PaymentMethodType::PermataBankTransfer
                    | enums::PaymentMethodType::OpenBankingUk
                    | enums::PaymentMethodType::PayBright
                    | enums::PaymentMethodType::Pix
                    | enums::PaymentMethodType::PaySafeCard
                    | enums::PaymentMethodType::Przelewy24
                    | enums::PaymentMethodType::PromptPay
                    | enums::PaymentMethodType::Pse
                    | enums::PaymentMethodType::RedCompra
                    | enums::PaymentMethodType::RedPagos
                    | enums::PaymentMethodType::SamsungPay
                    | enums::PaymentMethodType::Sepa
                    | enums::PaymentMethodType::Sofort
                    | enums::PaymentMethodType::Swish
                    | enums::PaymentMethodType::TouchNGo
                    | enums::PaymentMethodType::Trustly
                    | enums::PaymentMethodType::Twint
                    | enums::PaymentMethodType::UpiCollect
                    | enums::PaymentMethodType::UpiIntent
                    | enums::PaymentMethodType::Vipps
                    | enums::PaymentMethodType::VietQr
                    | enums::PaymentMethodType::Venmo
                    | enums::PaymentMethodType::Walley
                    | enums::PaymentMethodType::WeChatPay
                    | enums::PaymentMethodType::SevenEleven
                    | enums::PaymentMethodType::Lawson
                    | enums::PaymentMethodType::MiniStop
                    | enums::PaymentMethodType::FamilyMart
                    | enums::PaymentMethodType::Seicomart
                    | enums::PaymentMethodType::PayEasy
                    | enums::PaymentMethodType::LocalBankTransfer
                    | enums::PaymentMethodType::Mifinity
                    | enums::PaymentMethodType::Paze => {
                        Err(errors::ConnectorError::NotImplemented(
                            utils::get_unimplemented_payment_method_error_message("paypal"),
                        ))
                    }
                };

                Ok(Self {
                    intent,
                    purchase_units,
                    payment_source: payment_source?,
                })
            }
            domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::RealTimePayment(_)
            | domain::PaymentMethodData::MobilePayment(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::OpenBanking(_)
            | domain::PaymentMethodData::CardToken(_)
            | domain::PaymentMethodData::NetworkToken(_)
            | domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Paypal"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&domain::payments::CardRedirectData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &domain::payments::CardRedirectData) -> Result<Self, Self::Error> {
        match value {
            domain::payments::CardRedirectData::Knet {}
            | domain::payments::CardRedirectData::Benefit {}
            | domain::payments::CardRedirectData::MomoAtm {}
            | domain::payments::CardRedirectData::CardRedirect {} => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Paypal"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&domain::PayLaterData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &domain::PayLaterData) -> Result<Self, Self::Error> {
        match value {
            domain::PayLaterData::KlarnaRedirect { .. }
            | domain::PayLaterData::KlarnaSdk { .. }
            | domain::PayLaterData::AffirmRedirect {}
            | domain::PayLaterData::AfterpayClearpayRedirect { .. }
            | domain::PayLaterData::PayBrightRedirect {}
            | domain::PayLaterData::WalleyRedirect {}
            | domain::PayLaterData::AlmaRedirect {}
            | domain::PayLaterData::AtomeRedirect {} => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Paypal"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&domain::BankDebitData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &domain::BankDebitData) -> Result<Self, Self::Error> {
        match value {
            domain::BankDebitData::AchBankDebit { .. }
            | domain::BankDebitData::SepaBankDebit { .. }
            | domain::BankDebitData::BecsBankDebit { .. }
            | domain::BankDebitData::BacsBankDebit { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Paypal"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&domain::BankTransferData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &domain::BankTransferData) -> Result<Self, Self::Error> {
        match value {
            domain::BankTransferData::AchBankTransfer { .. }
            | domain::BankTransferData::SepaBankTransfer { .. }
            | domain::BankTransferData::BacsBankTransfer { .. }
            | domain::BankTransferData::MultibancoBankTransfer { .. }
            | domain::BankTransferData::PermataBankTransfer { .. }
            | domain::BankTransferData::BcaBankTransfer { .. }
            | domain::BankTransferData::BniVaBankTransfer { .. }
            | domain::BankTransferData::BriVaBankTransfer { .. }
            | domain::BankTransferData::CimbVaBankTransfer { .. }
            | domain::BankTransferData::DanamonVaBankTransfer { .. }
            | domain::BankTransferData::MandiriVaBankTransfer { .. }
            | domain::BankTransferData::Pix { .. }
            | domain::BankTransferData::Pse {}
            | domain::BankTransferData::LocalBankTransfer { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Paypal"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&domain::VoucherData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &domain::VoucherData) -> Result<Self, Self::Error> {
        match value {
            domain::VoucherData::Boleto(_)
            | domain::VoucherData::Efecty
            | domain::VoucherData::PagoEfectivo
            | domain::VoucherData::RedCompra
            | domain::VoucherData::RedPagos
            | domain::VoucherData::Alfamart(_)
            | domain::VoucherData::Indomaret(_)
            | domain::VoucherData::Oxxo
            | domain::VoucherData::SevenEleven(_)
            | domain::VoucherData::Lawson(_)
            | domain::VoucherData::MiniStop(_)
            | domain::VoucherData::FamilyMart(_)
            | domain::VoucherData::Seicomart(_)
            | domain::VoucherData::PayEasy(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Paypal"),
            )
            .into()),
        }
    }
}

impl TryFrom<&domain::GiftCardData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &domain::GiftCardData) -> Result<Self, Self::Error> {
        match value {
            domain::GiftCardData::Givex(_) | domain::GiftCardData::PaySafeCard {} => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Paypal"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PaypalAuthUpdateRequest {
    grant_type: String,
    client_id: Secret<String>,
    client_secret: Secret<String>,
}
impl TryFrom<&types::RefreshTokenRouterData> for PaypalAuthUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            grant_type: "client_credentials".to_string(),
            client_id: item.get_request_id()?,
            client_secret: item.request.app_id.clone(),
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct PaypalAuthUpdateResponse {
    pub access_token: Secret<String>,
    pub token_type: String,
    pub expires_in: i64,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PaypalAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaypalAuthUpdateResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Debug)]
pub enum PaypalAuthType {
    TemporaryAuth,
    AuthWithDetails(PaypalConnectorCredentials),
}

#[derive(Debug)]
pub enum PaypalConnectorCredentials {
    StandardIntegration(StandardFlowCredentials),
    PartnerIntegration(PartnerFlowCredentials),
}

impl PaypalConnectorCredentials {
    pub fn get_client_id(&self) -> Secret<String> {
        match self {
            Self::StandardIntegration(item) => item.client_id.clone(),
            Self::PartnerIntegration(item) => item.client_id.clone(),
        }
    }

    pub fn get_client_secret(&self) -> Secret<String> {
        match self {
            Self::StandardIntegration(item) => item.client_secret.clone(),
            Self::PartnerIntegration(item) => item.client_secret.clone(),
        }
    }

    pub fn get_payer_id(&self) -> Option<Secret<String>> {
        match self {
            Self::StandardIntegration(_) => None,
            Self::PartnerIntegration(item) => Some(item.payer_id.clone()),
        }
    }

    pub fn generate_authorization_value(&self) -> String {
        let auth_id = format!(
            "{}:{}",
            self.get_client_id().expose(),
            self.get_client_secret().expose(),
        );
        format!("Basic {}", consts::BASE64_ENGINE.encode(auth_id))
    }
}

#[derive(Debug)]
pub struct StandardFlowCredentials {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
}

#[derive(Debug)]
pub struct PartnerFlowCredentials {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
    pub(super) payer_id: Secret<String>,
}

impl PaypalAuthType {
    pub fn get_credentials(
        &self,
    ) -> CustomResult<&PaypalConnectorCredentials, errors::ConnectorError> {
        match self {
            Self::TemporaryAuth => Err(errors::ConnectorError::InvalidConnectorConfig {
                config: "TemporaryAuth found in connector_account_details",
            }
            .into()),
            Self::AuthWithDetails(credentials) => Ok(credentials),
        }
    }
}

impl TryFrom<&ConnectorAuthType> for PaypalAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self::AuthWithDetails(
                PaypalConnectorCredentials::StandardIntegration(StandardFlowCredentials {
                    client_id: key1.to_owned(),
                    client_secret: api_key.to_owned(),
                }),
            )),
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self::AuthWithDetails(
                PaypalConnectorCredentials::PartnerIntegration(PartnerFlowCredentials {
                    client_id: key1.to_owned(),
                    client_secret: api_key.to_owned(),
                    payer_id: api_secret.to_owned(),
                }),
            )),
            ConnectorAuthType::TemporaryAuth => Ok(Self::TemporaryAuth),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaypalOrderStatus {
    Pending,
    Completed,
    Voided,
    Created,
    Saved,
    PayerActionRequired,
    Approved,
}

impl ForeignFrom<(PaypalOrderStatus, PaypalPaymentIntent)> for storage_enums::AttemptStatus {
    fn foreign_from(item: (PaypalOrderStatus, PaypalPaymentIntent)) -> Self {
        match item.0 {
            PaypalOrderStatus::Completed => {
                if item.1 == PaypalPaymentIntent::Authorize {
                    Self::Authorized
                } else {
                    Self::Charged
                }
            }
            PaypalOrderStatus::Voided => Self::Voided,
            PaypalOrderStatus::Created | PaypalOrderStatus::Saved | PaypalOrderStatus::Pending => {
                Self::Pending
            }
            PaypalOrderStatus::Approved => Self::AuthenticationSuccessful,
            PaypalOrderStatus::PayerActionRequired => Self::AuthenticationPending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentsCollectionItem {
    amount: OrderAmount,
    expiration_time: Option<String>,
    id: String,
    final_capture: Option<bool>,
    status: PaypalPaymentStatus,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PaymentsCollection {
    authorizations: Option<Vec<PaymentsCollectionItem>>,
    captures: Option<Vec<PaymentsCollectionItem>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseUnitItem {
    pub reference_id: Option<String>,
    pub invoice_id: Option<String>,
    pub payments: PaymentsCollection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalThreeDsResponse {
    id: String,
    status: PaypalOrderStatus,
    links: Vec<PaypalLinks>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PaypalPreProcessingResponse {
    PaypalLiabilityResponse(PaypalLiabilityResponse),
    PaypalNonLiabilityResponse(PaypalNonLiabilityResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalLiabilityResponse {
    pub payment_source: CardParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalNonLiabilityResponse {
    payment_source: CardsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardParams {
    pub card: AuthResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub authentication_result: PaypalThreeDsParams,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalThreeDsParams {
    pub liability_shift: LiabilityShift,
    pub three_d_secure: ThreeDsCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeDsCheck {
    pub enrollment_status: Option<EnrollmentStatus>,
    pub authentication_status: Option<AuthenticationStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LiabilityShift {
    Possible,
    No,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnrollmentStatus {
    Null,
    #[serde(rename = "Y")]
    Ready,
    #[serde(rename = "N")]
    NotReady,
    #[serde(rename = "U")]
    Unavailable,
    #[serde(rename = "B")]
    Bypassed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationStatus {
    Null,
    #[serde(rename = "Y")]
    Success,
    #[serde(rename = "N")]
    Failed,
    #[serde(rename = "R")]
    Rejected,
    #[serde(rename = "A")]
    Attempted,
    #[serde(rename = "U")]
    Unable,
    #[serde(rename = "C")]
    ChallengeRequired,
    #[serde(rename = "I")]
    InfoOnly,
    #[serde(rename = "D")]
    Decoupled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalOrdersResponse {
    id: String,
    intent: PaypalPaymentIntent,
    status: PaypalOrderStatus,
    purchase_units: Vec<PurchaseUnitItem>,
    payment_source: Option<PaymentSourceItemResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalLinks {
    href: Option<Url>,
    rel: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RedirectPurchaseUnitItem {
    pub invoice_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalRedirectResponse {
    id: String,
    intent: PaypalPaymentIntent,
    status: PaypalOrderStatus,
    purchase_units: Vec<RedirectPurchaseUnitItem>,
    links: Vec<PaypalLinks>,
    payment_source: Option<PaymentSourceItemResponse>,
}

// Note: Don't change order of deserialization of variant, priority is in descending order
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PaypalAuthResponse {
    PaypalOrdersResponse(PaypalOrdersResponse),
    PaypalRedirectResponse(PaypalRedirectResponse),
    PaypalThreeDsResponse(PaypalThreeDsResponse),
}

// Note: Don't change order of deserialization of variant, priority is in descending order
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PaypalSyncResponse {
    PaypalOrdersSyncResponse(PaypalOrdersResponse),
    PaypalThreeDsSyncResponse(PaypalThreeDsSyncResponse),
    PaypalRedirectSyncResponse(PaypalRedirectResponse),
    PaypalPaymentsSyncResponse(PaypalPaymentsSyncResponse),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaypalPaymentsSyncResponse {
    id: String,
    status: PaypalPaymentStatus,
    amount: OrderAmount,
    invoice_id: Option<String>,
    supplementary_data: PaypalSupplementaryData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalThreeDsSyncResponse {
    id: String,
    status: PaypalOrderStatus,
    // provided to separated response of card's 3DS from other
    payment_source: CardsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardsData {
    card: CardDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDetails {
    last_digits: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaypalMeta {
    pub authorize_id: Option<String>,
    pub capture_id: Option<String>,
    pub psync_flow: PaypalPaymentIntent,
    pub next_action: Option<api_models::payments::NextActionCall>,
    pub order_id: Option<String>,
}

fn get_id_based_on_intent(
    intent: &PaypalPaymentIntent,
    purchase_unit: &PurchaseUnitItem,
) -> CustomResult<String, errors::ConnectorError> {
    || -> _ {
        match intent {
            PaypalPaymentIntent::Capture => Some(
                purchase_unit
                    .payments
                    .captures
                    .clone()?
                    .into_iter()
                    .next()?
                    .id,
            ),
            PaypalPaymentIntent::Authorize => Some(
                purchase_unit
                    .payments
                    .authorizations
                    .clone()?
                    .into_iter()
                    .next()?
                    .id,
            ),
            PaypalPaymentIntent::Authenticate => None,
        }
    }()
    .ok_or_else(|| errors::ConnectorError::MissingConnectorTransactionID.into())
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PaypalOrdersResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaypalOrdersResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let purchase_units = item
            .response
            .purchase_units
            .first()
            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?;

        let id = get_id_based_on_intent(&item.response.intent, purchase_units)?;
        let (connector_meta, order_id) = match item.response.intent.clone() {
            PaypalPaymentIntent::Capture => (
                serde_json::json!(PaypalMeta {
                    authorize_id: None,
                    capture_id: Some(id),
                    psync_flow: item.response.intent.clone(),
                    next_action: None,
                    order_id: None,
                }),
                types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
            ),

            PaypalPaymentIntent::Authorize => (
                serde_json::json!(PaypalMeta {
                    authorize_id: Some(id),
                    capture_id: None,
                    psync_flow: item.response.intent.clone(),
                    next_action: None,
                    order_id: None,
                }),
                types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
            ),

            PaypalPaymentIntent::Authenticate => {
                Err(errors::ConnectorError::ResponseDeserializationFailed)?
            }
        };
        //payment collection will always have only one element as we only make one transaction per order.
        let payment_collection = &item
            .response
            .purchase_units
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?
            .payments;
        //payment collection item will either have "authorizations" field or "capture" field, not both at a time.
        let payment_collection_item = match (
            &payment_collection.authorizations,
            &payment_collection.captures,
        ) {
            (Some(authorizations), None) => authorizations.first(),
            (None, Some(captures)) => captures.first(),
            (Some(_), Some(captures)) => captures.first(),
            _ => None,
        }
        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
        let status = payment_collection_item.status.clone();
        let status = storage_enums::AttemptStatus::from(status);
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: order_id,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(Some(MandateReference {
                    connector_mandate_id: match item.response.payment_source.clone() {
                        Some(paypal_source) => match paypal_source {
                            PaymentSourceItemResponse::Paypal(paypal_source) => {
                                paypal_source.attributes.map(|attr| attr.vault.id)
                            }
                            PaymentSourceItemResponse::Card(card) => {
                                card.attributes.map(|attr| attr.vault.id)
                            }
                            PaymentSourceItemResponse::Eps(_)
                            | PaymentSourceItemResponse::Ideal(_) => None,
                        },
                        None => None,
                    },
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: None,
                })),
                connector_metadata: Some(connector_meta),
                network_txn_id: None,
                connector_response_reference_id: purchase_units
                    .invoice_id
                    .clone()
                    .or(Some(item.response.id)),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

fn get_redirect_url(
    link_vec: Vec<PaypalLinks>,
) -> CustomResult<Option<Url>, errors::ConnectorError> {
    let mut link: Option<Url> = None;
    for item2 in link_vec.iter() {
        if item2.rel == "payer-action" {
            link.clone_from(&item2.href)
        }
    }
    Ok(link)
}

impl<F>
    ForeignTryFrom<(
        types::ResponseRouterData<
            F,
            PaypalSyncResponse,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
        Option<common_enums::PaymentExperience>,
    )> for types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (item, payment_experience): (
            types::ResponseRouterData<
                F,
                PaypalSyncResponse,
                types::PaymentsSyncData,
                types::PaymentsResponseData,
            >,
            Option<common_enums::PaymentExperience>,
        ),
    ) -> Result<Self, Self::Error> {
        match item.response {
            PaypalSyncResponse::PaypalOrdersSyncResponse(response) => {
                Self::try_from(types::ResponseRouterData {
                    response,
                    data: item.data,
                    http_code: item.http_code,
                })
            }
            PaypalSyncResponse::PaypalRedirectSyncResponse(response) => Self::foreign_try_from((
                types::ResponseRouterData {
                    response,
                    data: item.data,
                    http_code: item.http_code,
                },
                payment_experience,
            )),
            PaypalSyncResponse::PaypalPaymentsSyncResponse(response) => {
                Self::try_from(types::ResponseRouterData {
                    response,
                    data: item.data,
                    http_code: item.http_code,
                })
            }
            PaypalSyncResponse::PaypalThreeDsSyncResponse(response) => {
                Self::try_from(types::ResponseRouterData {
                    response,
                    data: item.data,
                    http_code: item.http_code,
                })
            }
        }
    }
}

impl<F, T>
    ForeignTryFrom<(
        types::ResponseRouterData<F, PaypalRedirectResponse, T, types::PaymentsResponseData>,
        Option<common_enums::PaymentExperience>,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (item, payment_experience): (
            types::ResponseRouterData<F, PaypalRedirectResponse, T, types::PaymentsResponseData>,
            Option<common_enums::PaymentExperience>,
        ),
    ) -> Result<Self, Self::Error> {
        let status = storage_enums::AttemptStatus::foreign_from((
            item.response.clone().status,
            item.response.intent.clone(),
        ));
        let link = get_redirect_url(item.response.links.clone())?;

        // For Paypal SDK flow, we need to trigger SDK client and then complete authorize
        let next_action =
            if let Some(common_enums::PaymentExperience::InvokeSdkClient) = payment_experience {
                Some(api_models::payments::NextActionCall::CompleteAuthorize)
            } else {
                None
            };
        let connector_meta = serde_json::json!(PaypalMeta {
            authorize_id: None,
            capture_id: None,
            psync_flow: item.response.intent,
            next_action,
            order_id: None,
        });
        let purchase_units = item.response.purchase_units.first();
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(Some(services::RedirectForm::from((
                    link.ok_or(errors::ConnectorError::ResponseDeserializationFailed)?,
                    services::Method::Get,
                )))),
                mandate_reference: Box::new(None),
                connector_metadata: Some(connector_meta),
                network_txn_id: None,
                connector_response_reference_id: Some(
                    purchase_units.map_or(item.response.id, |item| item.invoice_id.clone()),
                ),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::Authorize,
            PaypalRedirectResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Authorize,
            PaypalRedirectResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = storage_enums::AttemptStatus::foreign_from((
            item.response.clone().status,
            item.response.intent.clone(),
        ));
        let link = get_redirect_url(item.response.links.clone())?;

        let connector_meta = serde_json::json!(PaypalMeta {
            authorize_id: None,
            capture_id: None,
            psync_flow: item.response.intent,
            next_action: None,
            order_id: None,
        });
        let purchase_units = item.response.purchase_units.first();
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(Some(services::RedirectForm::from((
                    link.ok_or(errors::ConnectorError::ResponseDeserializationFailed)?,
                    services::Method::Get,
                )))),
                mandate_reference: Box::new(None),
                connector_metadata: Some(connector_meta),
                network_txn_id: None,
                connector_response_reference_id: Some(
                    purchase_units.map_or(item.response.id, |item| item.invoice_id.clone()),
                ),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::PostSessionTokens,
            PaypalRedirectResponse,
            types::PaymentsPostSessionTokensData,
            types::PaymentsResponseData,
        >,
    > for types::PaymentsPostSessionTokensRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: types::ResponseRouterData<
            api::PostSessionTokens,
            PaypalRedirectResponse,
            types::PaymentsPostSessionTokensData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = storage_enums::AttemptStatus::foreign_from((
            item.response.clone().status,
            item.response.intent.clone(),
        ));

        // For Paypal SDK flow, we need to trigger SDK client and then Confirm
        let next_action = Some(api_models::payments::NextActionCall::Confirm);

        let connector_meta = serde_json::json!(PaypalMeta {
            authorize_id: None,
            capture_id: None,
            psync_flow: item.response.intent,
            next_action,
            order_id: Some(item.response.id.clone()),
        });

        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::NoResponseId,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: Some(connector_meta),
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            PaypalThreeDsSyncResponse,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PaypalThreeDsSyncResponse,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            // status is hardcoded because this try_from will only be reached in card 3ds before the completion of complete authorize flow.
            // also force sync won't be hit in terminal status thus leaving us with only one status to get here.
            status: storage_enums::AttemptStatus::AuthenticationPending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            PaypalThreeDsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PaypalThreeDsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let connector_meta = serde_json::json!(PaypalMeta {
            authorize_id: None,
            capture_id: None,
            psync_flow: PaypalPaymentIntent::Authenticate, // when there is no capture or auth id present
            next_action: None,
            order_id: None,
        });

        let status = storage_enums::AttemptStatus::foreign_from((
            item.response.clone().status,
            PaypalPaymentIntent::Authenticate,
        ));
        let link = get_redirect_url(item.response.links.clone())?;

        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(Some(paypal_threeds_link((
                    link,
                    item.data.request.complete_authorize_url.clone(),
                ))?)),
                mandate_reference: Box::new(None),
                connector_metadata: Some(connector_meta),
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

fn paypal_threeds_link(
    (redirect_url, complete_auth_url): (Option<Url>, Option<String>),
) -> CustomResult<services::RedirectForm, errors::ConnectorError> {
    let mut redirect_url =
        redirect_url.ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
    let complete_auth_url =
        complete_auth_url.ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: "complete_authorize_url",
        })?;
    let mut form_fields = std::collections::HashMap::from_iter(
        redirect_url
            .query_pairs()
            .map(|(key, value)| (key.to_string(), value.to_string())),
    );

    // paypal requires return url to be passed as a field along with payer_action_url
    form_fields.insert(String::from("redirect_uri"), complete_auth_url);

    // Do not include query params in the endpoint
    redirect_url.set_query(None);

    Ok(services::RedirectForm::Form {
        endpoint: redirect_url.to_string(),
        method: services::Method::Get,
        form_fields,
    })
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, PaypalPaymentsSyncResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PaypalPaymentsSyncResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: storage_enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response
                        .supplementary_data
                        .related_ids
                        .order_id
                        .clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .invoice_id
                    .clone()
                    .or(Some(item.response.supplementary_data.related_ids.order_id)),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
pub struct PaypalFulfillRequest {
    sender_batch_header: PayoutBatchHeader,
    items: Vec<PaypalPayoutItem>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
pub struct PayoutBatchHeader {
    sender_batch_id: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
pub struct PaypalPayoutItem {
    amount: PayoutAmount,
    note: Option<String>,
    notification_language: String,
    #[serde(flatten)]
    payout_method_data: PaypalPayoutMethodData,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
pub struct PaypalPayoutMethodData {
    recipient_type: PayoutRecipientType,
    recipient_wallet: PayoutWalletType,
    receiver: PaypalPayoutDataType,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayoutRecipientType {
    Email,
    PaypalId,
    Phone,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayoutWalletType {
    Paypal,
    Venmo,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaypalPayoutDataType {
    EmailType(Email),
    OtherType(Secret<String>),
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
pub struct PayoutAmount {
    value: StringMajorUnit,
    currency: storage_enums::Currency,
}

#[cfg(feature = "payouts")]
impl TryFrom<&PaypalRouterData<&types::PayoutsRouterData<api::PoFulfill>>>
    for PaypalFulfillRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::PayoutsRouterData<api::PoFulfill>>,
    ) -> Result<Self, Self::Error> {
        let item_data = PaypalPayoutItem::try_from(item)?;
        Ok(Self {
            sender_batch_header: PayoutBatchHeader {
                sender_batch_id: item.router_data.request.payout_id.to_owned(),
            },
            items: vec![item_data],
        })
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<&PaypalRouterData<&types::PayoutsRouterData<api::PoFulfill>>> for PaypalPayoutItem {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::PayoutsRouterData<api::PoFulfill>>,
    ) -> Result<Self, Self::Error> {
        let amount = PayoutAmount {
            value: item.amount.clone(),
            currency: item.router_data.request.destination_currency,
        };

        let payout_method_data = match item.router_data.get_payout_method_data()? {
            api::PayoutMethodData::Wallet(wallet_data) => match wallet_data {
                api::WalletPayout::Paypal(data) => {
                    let (recipient_type, receiver) =
                        match (data.email, data.telephone_number, data.paypal_id) {
                            (Some(email), _, _) => (
                                PayoutRecipientType::Email,
                                PaypalPayoutDataType::EmailType(email),
                            ),
                            (_, Some(phone), _) => (
                                PayoutRecipientType::Phone,
                                PaypalPayoutDataType::OtherType(phone),
                            ),
                            (_, _, Some(paypal_id)) => (
                                PayoutRecipientType::PaypalId,
                                PaypalPayoutDataType::OtherType(paypal_id),
                            ),
                            _ => Err(errors::ConnectorError::MissingRequiredField {
                                field_name: "receiver_data",
                            })?,
                        };

                    PaypalPayoutMethodData {
                        recipient_type,
                        recipient_wallet: PayoutWalletType::Paypal,
                        receiver,
                    }
                }
                api::WalletPayout::Venmo(data) => {
                    let receiver = PaypalPayoutDataType::OtherType(data.telephone_number.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "telephone_number",
                        },
                    )?);
                    PaypalPayoutMethodData {
                        recipient_type: PayoutRecipientType::Phone,
                        recipient_wallet: PayoutWalletType::Venmo,
                        receiver,
                    }
                }
            },
            _ => Err(errors::ConnectorError::NotSupported {
                message: "PayoutMethodType is not supported".to_string(),
                connector: "Paypal",
            })?,
        };

        Ok(Self {
            amount,
            payout_method_data,
            note: item.router_data.description.to_owned(),
            notification_language: consts::DEFAULT_NOTIFICATION_SCRIPT_LANGUAGE.to_string(),
        })
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
pub struct PaypalFulfillResponse {
    batch_header: PaypalBatchResponse,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
pub struct PaypalBatchResponse {
    payout_batch_id: String,
    batch_status: PaypalFulfillStatus,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaypalFulfillStatus {
    Denied,
    Pending,
    Processing,
    Success,
    Cancelled,
}

#[cfg(feature = "payouts")]
impl ForeignFrom<PaypalFulfillStatus> for storage_enums::PayoutStatus {
    fn foreign_from(status: PaypalFulfillStatus) -> Self {
        match status {
            PaypalFulfillStatus::Success => Self::Success,
            PaypalFulfillStatus::Denied => Self::Failed,
            PaypalFulfillStatus::Cancelled => Self::Cancelled,
            PaypalFulfillStatus::Pending | PaypalFulfillStatus::Processing => Self::Pending,
        }
    }
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<types::PayoutsResponseRouterData<F, PaypalFulfillResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, PaypalFulfillResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(storage_enums::PayoutStatus::foreign_from(
                    item.response.batch_header.batch_status,
                )),
                connector_payout_id: Some(item.response.batch_header.payout_batch_id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct PaypalPaymentsCaptureRequest {
    amount: OrderAmount,
    final_capture: bool,
}

impl TryFrom<&PaypalRouterData<&types::PaymentsCaptureRouterData>>
    for PaypalPaymentsCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let amount = OrderAmount {
            currency_code: item.router_data.request.currency,
            value: item.amount.clone(),
        };
        Ok(Self {
            amount,
            final_capture: true,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaypalPaymentStatus {
    Created,
    Captured,
    Completed,
    Declined,
    Voided,
    Failed,
    Pending,
    Denied,
    Expired,
    PartiallyCaptured,
    Refunded,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaypalCaptureResponse {
    id: String,
    status: PaypalPaymentStatus,
    amount: Option<OrderAmount>,
    invoice_id: Option<String>,
    final_capture: bool,
    payment_source: Option<PaymentSourceItemResponse>,
}

impl From<PaypalPaymentStatus> for storage_enums::AttemptStatus {
    fn from(item: PaypalPaymentStatus) -> Self {
        match item {
            PaypalPaymentStatus::Created => Self::Authorized,
            PaypalPaymentStatus::Completed
            | PaypalPaymentStatus::Captured
            | PaypalPaymentStatus::Refunded => Self::Charged,
            PaypalPaymentStatus::Declined => Self::Failure,
            PaypalPaymentStatus::Failed => Self::CaptureFailed,
            PaypalPaymentStatus::Pending => Self::Pending,
            PaypalPaymentStatus::Denied | PaypalPaymentStatus::Expired => Self::Failure,
            PaypalPaymentStatus::PartiallyCaptured => Self::PartialCharged,
            PaypalPaymentStatus::Voided => Self::Voided,
        }
    }
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<PaypalCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<PaypalCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let status = storage_enums::AttemptStatus::from(item.response.status);
        let amount_captured = match status {
            storage_enums::AttemptStatus::Pending
            | storage_enums::AttemptStatus::Authorized
            | storage_enums::AttemptStatus::Failure
            | storage_enums::AttemptStatus::RouterDeclined
            | storage_enums::AttemptStatus::AuthenticationFailed
            | storage_enums::AttemptStatus::CaptureFailed
            | storage_enums::AttemptStatus::Started
            | storage_enums::AttemptStatus::AuthenticationPending
            | storage_enums::AttemptStatus::AuthenticationSuccessful
            | storage_enums::AttemptStatus::AuthorizationFailed
            | storage_enums::AttemptStatus::Authorizing
            | storage_enums::AttemptStatus::VoidInitiated
            | storage_enums::AttemptStatus::CodInitiated
            | storage_enums::AttemptStatus::CaptureInitiated
            | storage_enums::AttemptStatus::VoidFailed
            | storage_enums::AttemptStatus::AutoRefunded
            | storage_enums::AttemptStatus::Unresolved
            | storage_enums::AttemptStatus::PaymentMethodAwaited
            | storage_enums::AttemptStatus::ConfirmationAwaited
            | storage_enums::AttemptStatus::DeviceDataCollectionPending
            | storage_enums::AttemptStatus::Voided => 0,
            storage_enums::AttemptStatus::Charged
            | storage_enums::AttemptStatus::PartialCharged
            | storage_enums::AttemptStatus::PartialChargedAndChargeable => {
                item.data.request.amount_to_capture
            }
        };
        let connector_payment_id: PaypalMeta =
            to_connector_meta(item.data.request.connector_meta.clone())?;
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.data.request.connector_transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: Some(serde_json::json!(PaypalMeta {
                    authorize_id: connector_payment_id.authorize_id,
                    capture_id: Some(item.response.id.clone()),
                    psync_flow: PaypalPaymentIntent::Capture,
                    next_action: None,
                    order_id: None,
                })),
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .invoice_id
                    .or(Some(item.response.id)),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            amount_captured: Some(amount_captured),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaypalCancelStatus {
    Voided,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PaypalPaymentsCancelResponse {
    id: String,
    status: PaypalCancelStatus,
    amount: Option<OrderAmount>,
    invoice_id: Option<String>,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, PaypalPaymentsCancelResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PaypalPaymentsCancelResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = match item.response.status {
            PaypalCancelStatus::Voided => storage_enums::AttemptStatus::Voided,
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .invoice_id
                    .or(Some(item.response.id)),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct PaypalRefundRequest {
    pub amount: OrderAmount,
}

impl<F> TryFrom<&PaypalRouterData<&types::RefundsRouterData<F>>> for PaypalRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: OrderAmount {
                currency_code: item.router_data.request.currency,
                value: item.amount.clone(),
            },
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Completed,
    Failed,
    Cancelled,
    Pending,
}

impl From<RefundStatus> for storage_enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Completed => Self::Success,
            RefundStatus::Failed | RefundStatus::Cancelled => Self::Failure,
            RefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
    amount: Option<OrderAmount>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: storage_enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RefundSyncResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: storage_enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderErrorDetails {
    pub issue: String,
    pub description: String,
    pub value: Option<String>,
    pub field: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaypalOrderErrorResponse {
    pub name: Option<String>,
    pub message: String,
    pub debug_id: Option<String>,
    pub details: Option<Vec<OrderErrorDetails>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorDetails {
    pub issue: String,
    pub description: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaypalPaymentErrorResponse {
    pub name: Option<String>,
    pub message: String,
    pub debug_id: Option<String>,
    pub details: Option<Vec<ErrorDetails>>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalAccessTokenErrorResponse {
    pub error: String,
    pub error_description: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalWebhooksBody {
    pub event_type: PaypalWebhookEventType,
    pub resource: PaypalResource,
}

#[derive(Clone, Deserialize, Debug, strum::Display, Serialize)]
pub enum PaypalWebhookEventType {
    #[serde(rename = "PAYMENT.AUTHORIZATION.CREATED")]
    PaymentAuthorizationCreated,
    #[serde(rename = "PAYMENT.AUTHORIZATION.VOIDED")]
    PaymentAuthorizationVoided,
    #[serde(rename = "PAYMENT.CAPTURE.DECLINED")]
    PaymentCaptureDeclined,
    #[serde(rename = "PAYMENT.CAPTURE.COMPLETED")]
    PaymentCaptureCompleted,
    #[serde(rename = "PAYMENT.CAPTURE.PENDING")]
    PaymentCapturePending,
    #[serde(rename = "PAYMENT.CAPTURE.REFUNDED")]
    PaymentCaptureRefunded,
    #[serde(rename = "CHECKOUT.ORDER.APPROVED")]
    CheckoutOrderApproved,
    #[serde(rename = "CHECKOUT.ORDER.COMPLETED")]
    CheckoutOrderCompleted,
    #[serde(rename = "CHECKOUT.ORDER.PROCESSED")]
    CheckoutOrderProcessed,
    #[serde(rename = "CUSTOMER.DISPUTE.CREATED")]
    CustomerDisputeCreated,
    #[serde(rename = "CUSTOMER.DISPUTE.RESOLVED")]
    CustomerDisputeResolved,
    #[serde(rename = "CUSTOMER.DISPUTE.UPDATED")]
    CustomerDisputedUpdated,
    #[serde(rename = "RISK.DISPUTE.CREATED")]
    RiskDisputeCreated,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum PaypalResource {
    PaypalCardWebhooks(Box<PaypalCardWebhooks>),
    PaypalRedirectsWebhooks(Box<PaypalRedirectsWebhooks>),
    PaypalRefundWebhooks(Box<PaypalRefundWebhooks>),
    PaypalDisputeWebhooks(Box<PaypalDisputeWebhooks>),
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalDisputeWebhooks {
    pub dispute_id: String,
    pub disputed_transactions: Vec<DisputeTransaction>,
    pub dispute_amount: OrderAmount,
    pub dispute_outcome: Option<DisputeOutcome>,
    pub dispute_life_cycle_stage: DisputeLifeCycleStage,
    pub status: DisputeStatus,
    pub reason: Option<String>,
    pub external_reason_code: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub seller_response_due_date: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub update_time: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub create_time: Option<PrimitiveDateTime>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DisputeTransaction {
    pub seller_transaction_id: String,
}

#[derive(Clone, Deserialize, Debug, strum::Display, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DisputeLifeCycleStage {
    Inquiry,
    Chargeback,
    PreArbitration,
    Arbitration,
}

#[derive(Deserialize, Debug, strum::Display, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DisputeStatus {
    Open,
    WaitingForBuyerResponse,
    WaitingForSellerResponse,
    UnderReview,
    Resolved,
    Other,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DisputeOutcome {
    pub outcome_code: OutcomeCode,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutcomeCode {
    ResolvedBuyerFavour,
    ResolvedSellerFavour,
    ResolvedWithPayout,
    CanceledByBuyer,
    ACCEPTED,
    DENIED,
    NONE,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalRefundWebhooks {
    pub id: String,
    pub amount: OrderAmount,
    pub seller_payable_breakdown: PaypalSellerPayableBreakdown,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalSellerPayableBreakdown {
    pub total_refunded_amount: OrderAmount,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalCardWebhooks {
    pub supplementary_data: PaypalSupplementaryData,
    pub amount: OrderAmount,
    pub invoice_id: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalRedirectsWebhooks {
    pub purchase_units: Vec<PurchaseUnitItem>,
    pub links: Vec<PaypalLinks>,
    pub id: String,
    pub intent: PaypalPaymentIntent,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalWebhooksPurchaseUnits {
    pub reference_id: String,
    pub amount: OrderAmount,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalSupplementaryData {
    pub related_ids: PaypalRelatedIds,
}
#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalRelatedIds {
    pub order_id: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalWebooksEventType {
    pub event_type: PaypalWebhookEventType,
}

impl ForeignFrom<(PaypalWebhookEventType, Option<OutcomeCode>)> for api::IncomingWebhookEvent {
    fn foreign_from((event, outcome): (PaypalWebhookEventType, Option<OutcomeCode>)) -> Self {
        match event {
            PaypalWebhookEventType::PaymentCaptureCompleted
            | PaypalWebhookEventType::CheckoutOrderCompleted => Self::PaymentIntentSuccess,
            PaypalWebhookEventType::PaymentCapturePending
            | PaypalWebhookEventType::CheckoutOrderProcessed => Self::PaymentIntentProcessing,
            PaypalWebhookEventType::PaymentCaptureDeclined => Self::PaymentIntentFailure,
            PaypalWebhookEventType::PaymentCaptureRefunded => Self::RefundSuccess,
            PaypalWebhookEventType::CustomerDisputeCreated => Self::DisputeOpened,
            PaypalWebhookEventType::RiskDisputeCreated => Self::DisputeAccepted,
            PaypalWebhookEventType::CustomerDisputeResolved => {
                if let Some(outcome_code) = outcome {
                    Self::from(outcome_code)
                } else {
                    Self::EventNotSupported
                }
            }
            PaypalWebhookEventType::PaymentAuthorizationCreated
            | PaypalWebhookEventType::PaymentAuthorizationVoided
            | PaypalWebhookEventType::CheckoutOrderApproved
            | PaypalWebhookEventType::CustomerDisputedUpdated
            | PaypalWebhookEventType::Unknown => Self::EventNotSupported,
        }
    }
}

impl From<OutcomeCode> for api::IncomingWebhookEvent {
    fn from(outcome_code: OutcomeCode) -> Self {
        match outcome_code {
            OutcomeCode::ResolvedBuyerFavour => Self::DisputeLost,
            OutcomeCode::ResolvedSellerFavour => Self::DisputeWon,
            OutcomeCode::CanceledByBuyer => Self::DisputeCancelled,
            OutcomeCode::ACCEPTED => Self::DisputeAccepted,
            OutcomeCode::DENIED => Self::DisputeCancelled,
            OutcomeCode::NONE => Self::DisputeCancelled,
            OutcomeCode::ResolvedWithPayout => Self::EventNotSupported,
        }
    }
}

impl From<DisputeLifeCycleStage> for enums::DisputeStage {
    fn from(dispute_life_cycle_stage: DisputeLifeCycleStage) -> Self {
        match dispute_life_cycle_stage {
            DisputeLifeCycleStage::Inquiry => Self::PreDispute,
            DisputeLifeCycleStage::Chargeback => Self::Dispute,
            DisputeLifeCycleStage::PreArbitration => Self::PreArbitration,
            DisputeLifeCycleStage::Arbitration => Self::PreArbitration,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PaypalSourceVerificationRequest {
    pub transmission_id: String,
    pub transmission_time: String,
    pub cert_url: String,
    pub transmission_sig: String,
    pub auth_algo: String,
    pub webhook_id: String,
    pub webhook_event: serde_json::Value,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PaypalSourceVerificationResponse {
    pub verification_status: PaypalSourceVerificationStatus,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaypalSourceVerificationStatus {
    Success,
    Failure,
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::VerifyWebhookSource,
            PaypalSourceVerificationResponse,
            types::VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
    > for types::VerifyWebhookSourceRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::VerifyWebhookSource,
            PaypalSourceVerificationResponse,
            types::VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(VerifyWebhookSourceResponseData {
                verify_webhook_status: types::VerifyWebhookStatus::from(
                    item.response.verification_status,
                ),
            }),
            ..item.data
        })
    }
}

impl From<PaypalSourceVerificationStatus> for types::VerifyWebhookStatus {
    fn from(item: PaypalSourceVerificationStatus) -> Self {
        match item {
            PaypalSourceVerificationStatus::Success => Self::SourceVerified,
            PaypalSourceVerificationStatus::Failure => Self::SourceNotVerified,
        }
    }
}

impl TryFrom<(PaypalCardWebhooks, PaypalWebhookEventType)> for PaypalPaymentsSyncResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (webhook_body, webhook_event): (PaypalCardWebhooks, PaypalWebhookEventType),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id: webhook_body.supplementary_data.related_ids.order_id.clone(),
            status: PaypalPaymentStatus::try_from(webhook_event)?,
            amount: webhook_body.amount,
            supplementary_data: webhook_body.supplementary_data,
            invoice_id: webhook_body.invoice_id,
        })
    }
}

impl TryFrom<(PaypalRedirectsWebhooks, PaypalWebhookEventType)> for PaypalOrdersResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (webhook_body, webhook_event): (PaypalRedirectsWebhooks, PaypalWebhookEventType),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id: webhook_body.id,
            intent: webhook_body.intent,
            status: PaypalOrderStatus::try_from(webhook_event)?,
            purchase_units: webhook_body.purchase_units,
            payment_source: None,
        })
    }
}

impl TryFrom<(PaypalRefundWebhooks, PaypalWebhookEventType)> for RefundSyncResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (webhook_body, webhook_event): (PaypalRefundWebhooks, PaypalWebhookEventType),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id: webhook_body.id,
            status: RefundStatus::try_from(webhook_event)
                .attach_printable("Could not find suitable webhook event")?,
        })
    }
}

impl TryFrom<PaypalWebhookEventType> for PaypalPaymentStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(event: PaypalWebhookEventType) -> Result<Self, Self::Error> {
        match event {
            PaypalWebhookEventType::PaymentCaptureCompleted
            | PaypalWebhookEventType::CheckoutOrderCompleted => Ok(Self::Completed),
            PaypalWebhookEventType::PaymentAuthorizationVoided => Ok(Self::Voided),
            PaypalWebhookEventType::PaymentCaptureDeclined => Ok(Self::Declined),
            PaypalWebhookEventType::PaymentCapturePending
            | PaypalWebhookEventType::CheckoutOrderApproved
            | PaypalWebhookEventType::CheckoutOrderProcessed => Ok(Self::Pending),
            PaypalWebhookEventType::PaymentAuthorizationCreated => Ok(Self::Created),
            PaypalWebhookEventType::PaymentCaptureRefunded => Ok(Self::Refunded),
            PaypalWebhookEventType::CustomerDisputeCreated
            | PaypalWebhookEventType::CustomerDisputeResolved
            | PaypalWebhookEventType::CustomerDisputedUpdated
            | PaypalWebhookEventType::RiskDisputeCreated
            | PaypalWebhookEventType::Unknown => {
                Err(errors::ConnectorError::WebhookEventTypeNotFound.into())
            }
        }
    }
}

impl TryFrom<PaypalWebhookEventType> for RefundStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(event: PaypalWebhookEventType) -> Result<Self, Self::Error> {
        match event {
            PaypalWebhookEventType::PaymentCaptureRefunded => Ok(Self::Completed),
            PaypalWebhookEventType::PaymentAuthorizationCreated
            | PaypalWebhookEventType::PaymentAuthorizationVoided
            | PaypalWebhookEventType::PaymentCaptureDeclined
            | PaypalWebhookEventType::PaymentCaptureCompleted
            | PaypalWebhookEventType::PaymentCapturePending
            | PaypalWebhookEventType::CheckoutOrderApproved
            | PaypalWebhookEventType::CheckoutOrderCompleted
            | PaypalWebhookEventType::CheckoutOrderProcessed
            | PaypalWebhookEventType::CustomerDisputeCreated
            | PaypalWebhookEventType::CustomerDisputeResolved
            | PaypalWebhookEventType::CustomerDisputedUpdated
            | PaypalWebhookEventType::RiskDisputeCreated
            | PaypalWebhookEventType::Unknown => {
                Err(errors::ConnectorError::WebhookEventTypeNotFound.into())
            }
        }
    }
}

impl TryFrom<PaypalWebhookEventType> for PaypalOrderStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(event: PaypalWebhookEventType) -> Result<Self, Self::Error> {
        match event {
            PaypalWebhookEventType::PaymentCaptureCompleted
            | PaypalWebhookEventType::CheckoutOrderCompleted => Ok(Self::Completed),
            PaypalWebhookEventType::PaymentAuthorizationVoided => Ok(Self::Voided),
            PaypalWebhookEventType::PaymentCapturePending
            | PaypalWebhookEventType::CheckoutOrderProcessed => Ok(Self::Pending),
            PaypalWebhookEventType::PaymentAuthorizationCreated => Ok(Self::Created),
            PaypalWebhookEventType::CheckoutOrderApproved
            | PaypalWebhookEventType::PaymentCaptureDeclined
            | PaypalWebhookEventType::PaymentCaptureRefunded
            | PaypalWebhookEventType::CustomerDisputeCreated
            | PaypalWebhookEventType::CustomerDisputeResolved
            | PaypalWebhookEventType::CustomerDisputedUpdated
            | PaypalWebhookEventType::RiskDisputeCreated
            | PaypalWebhookEventType::Unknown => {
                Err(errors::ConnectorError::WebhookEventTypeNotFound.into())
            }
        }
    }
}

impl TryFrom<&types::VerifyWebhookSourceRequestData> for PaypalSourceVerificationRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(req: &types::VerifyWebhookSourceRequestData) -> Result<Self, Self::Error> {
        let req_body = serde_json::from_slice(&req.webhook_body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(Self {
            transmission_id: get_headers(
                &req.webhook_headers,
                webhook_headers::PAYPAL_TRANSMISSION_ID,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?,
            transmission_time: get_headers(
                &req.webhook_headers,
                webhook_headers::PAYPAL_TRANSMISSION_TIME,
            )?,
            cert_url: get_headers(&req.webhook_headers, webhook_headers::PAYPAL_CERT_URL)?,
            transmission_sig: get_headers(
                &req.webhook_headers,
                webhook_headers::PAYPAL_TRANSMISSION_SIG,
            )?,
            auth_algo: get_headers(&req.webhook_headers, webhook_headers::PAYPAL_AUTH_ALGO)?,
            webhook_id: String::from_utf8(req.merchant_secret.secret.to_vec())
                .change_context(errors::ConnectorError::WebhookVerificationSecretNotFound)
                .attach_printable("Could not convert secret to UTF-8")?,
            webhook_event: req_body,
        })
    }
}

fn get_headers(
    header: &actix_web::http::header::HeaderMap,
    key: &'static str,
) -> CustomResult<String, errors::ConnectorError> {
    let header_value = header
        .get(key)
        .map(|value| value.to_str())
        .ok_or(errors::ConnectorError::MissingRequiredField { field_name: key })?
        .change_context(errors::ConnectorError::InvalidDataFormat { field_name: key })?
        .to_owned();
    Ok(header_value)
}

impl From<OrderErrorDetails> for utils::ErrorCodeAndMessage {
    fn from(error: OrderErrorDetails) -> Self {
        Self {
            error_code: error.issue.to_string(),
            error_message: error.issue.to_string(),
        }
    }
}

impl From<ErrorDetails> for utils::ErrorCodeAndMessage {
    fn from(error: ErrorDetails) -> Self {
        Self {
            error_code: error.issue.to_string(),
            error_message: error.issue.to_string(),
        }
    }
}
