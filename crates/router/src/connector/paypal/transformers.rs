use api_models::payments::BankRedirectData;
use common_utils::errors::CustomResult;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::{
        self, to_connector_meta, AccessTokenRequestInfo, AddressDetailsData,
        BankRedirectBillingData, CardData, PaymentsAuthorizeRequestData,
    },
    core::errors,
    services,
    types::{self, api, storage::enums as storage_enums, transformers::ForeignFrom},
};

#[derive(Debug, Serialize)]
pub struct PaypalRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for PaypalRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaypalPaymentIntent {
    Capture,
    Authorize,
}

#[derive(Default, Debug, Clone, Serialize, Eq, PartialEq, Deserialize)]
pub struct OrderAmount {
    currency_code: storage_enums::Currency,
    value: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PurchaseUnitRequest {
    reference_id: String,
    amount: OrderAmount,
}

#[derive(Debug, Serialize)]
pub struct Address {
    address_line_1: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    country_code: api_models::enums::CountryAlpha2,
}

#[derive(Debug, Serialize)]
pub struct CardRequest {
    billing_address: Option<Address>,
    expiry: Option<Secret<String>>,
    name: Secret<String>,
    number: Option<cards::CardNumber>,
    security_code: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub struct RedirectRequest {
    name: Secret<String>,
    country_code: api_models::enums::CountryAlpha2,
    experience_context: ContextStruct,
}

#[derive(Debug, Serialize)]
pub struct ContextStruct {
    return_url: Option<String>,
    cancel_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaypalRedirectionRequest {
    experience_context: ContextStruct,
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

#[derive(Debug, Serialize)]
pub struct PaypalPaymentsRequest {
    intent: PaypalPaymentIntent,
    purchase_units: Vec<PurchaseUnitRequest>,
    payment_source: Option<PaymentSourceItem>,
}

fn get_address_info(
    payment_address: Option<&api_models::payments::Address>,
) -> Result<Option<Address>, error_stack::Report<errors::ConnectorError>> {
    let address = payment_address.and_then(|payment_address| payment_address.address.as_ref());
    let address = match address {
        Some(address) => Some(Address {
            country_code: address.get_country()?.to_owned(),
            address_line_1: address.line1.clone(),
            postal_code: address.zip.clone(),
        }),
        None => None,
    };
    Ok(address)
}
fn get_payment_source(
    item: &types::PaymentsAuthorizeRouterData,
    bank_redirection_data: &BankRedirectData,
) -> Result<PaymentSourceItem, error_stack::Report<errors::ConnectorError>> {
    match bank_redirection_data {
        BankRedirectData::Eps {
            billing_details,
            bank_name: _,
            country,
        } => Ok(PaymentSourceItem::Eps(RedirectRequest {
            name: billing_details.get_billing_name()?,
            country_code: country.ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "eps.country",
            })?,
            experience_context: ContextStruct {
                return_url: item.request.complete_authorize_url.clone(),
                cancel_url: item.request.complete_authorize_url.clone(),
            },
        })),
        BankRedirectData::Giropay {
            billing_details,
            country,
            ..
        } => Ok(PaymentSourceItem::Giropay(RedirectRequest {
            name: billing_details.get_billing_name()?,
            country_code: country.ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "giropay.country",
            })?,
            experience_context: ContextStruct {
                return_url: item.request.complete_authorize_url.clone(),
                cancel_url: item.request.complete_authorize_url.clone(),
            },
        })),
        BankRedirectData::Ideal {
            billing_details,
            bank_name: _,
            country,
        } => Ok(PaymentSourceItem::IDeal(RedirectRequest {
            name: billing_details.get_billing_name()?,
            country_code: country.ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "ideal.country",
            })?,
            experience_context: ContextStruct {
                return_url: item.request.complete_authorize_url.clone(),
                cancel_url: item.request.complete_authorize_url.clone(),
            },
        })),
        BankRedirectData::Sofort {
            country,
            preferred_language: _,
            billing_details,
        } => Ok(PaymentSourceItem::Sofort(RedirectRequest {
            name: billing_details.get_billing_name()?,
            country_code: *country,
            experience_context: ContextStruct {
                return_url: item.request.complete_authorize_url.clone(),
                cancel_url: item.request.complete_authorize_url.clone(),
            },
        })),
        BankRedirectData::BancontactCard { .. }
        | BankRedirectData::Blik { .. }
        | BankRedirectData::Przelewy24 { .. } => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("Paypal"),
        )
        .into()),
        BankRedirectData::Bizum {}
        | BankRedirectData::Interac { .. }
        | BankRedirectData::OnlineBankingCzechRepublic { .. }
        | BankRedirectData::OnlineBankingFinland { .. }
        | BankRedirectData::OnlineBankingPoland { .. }
        | BankRedirectData::OnlineBankingSlovakia { .. }
        | BankRedirectData::OpenBankingUk { .. }
        | BankRedirectData::Trustly { .. }
        | BankRedirectData::OnlineBankingFpx { .. }
        | BankRedirectData::OnlineBankingThailand { .. } => {
            Err(errors::ConnectorError::NotSupported {
                message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "Paypal",
            }
            .into())
        }
    }
}

impl TryFrom<&PaypalRouterData<&types::PaymentsAuthorizeRouterData>> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaypalRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data {
            api_models::payments::PaymentMethodData::Card(ref ccard) => {
                let intent = if item.router_data.request.is_auto_capture()? {
                    PaypalPaymentIntent::Capture
                } else {
                    PaypalPaymentIntent::Authorize
                };
                let amount = OrderAmount {
                    currency_code: item.router_data.request.currency,
                    value: item.amount.to_owned(),
                };
                let reference_id = item.router_data.attempt_id.clone();

                let purchase_units = vec![PurchaseUnitRequest {
                    reference_id,
                    amount,
                }];
                let card = item.router_data.request.get_card()?;
                let expiry = Some(card.get_expiry_date_as_yyyymm("-"));

                let payment_source = Some(PaymentSourceItem::Card(CardRequest {
                    billing_address: get_address_info(item.router_data.address.billing.as_ref())?,
                    expiry,
                    name: ccard.card_holder_name.clone(),
                    number: Some(ccard.card_number.clone()),
                    security_code: Some(ccard.card_cvc.clone()),
                }));

                Ok(Self {
                    intent,
                    purchase_units,
                    payment_source,
                })
            }
            api::PaymentMethodData::Wallet(ref wallet_data) => match wallet_data {
                api_models::payments::WalletData::PaypalRedirect(_) => {
                    let intent = if item.router_data.request.is_auto_capture()? {
                        PaypalPaymentIntent::Capture
                    } else {
                        PaypalPaymentIntent::Authorize
                    };
                    let amount = OrderAmount {
                        currency_code: item.router_data.request.currency,
                        value: item.amount.to_owned(),
                    };
                    let reference_id = item.router_data.attempt_id.clone();
                    let purchase_units = vec![PurchaseUnitRequest {
                        reference_id,
                        amount,
                    }];
                    let payment_source =
                        Some(PaymentSourceItem::Paypal(PaypalRedirectionRequest {
                            experience_context: ContextStruct {
                                return_url: item.router_data.request.complete_authorize_url.clone(),
                                cancel_url: item.router_data.request.complete_authorize_url.clone(),
                            },
                        }));

                    Ok(Self {
                        intent,
                        purchase_units,
                        payment_source,
                    })
                }
                api_models::payments::WalletData::AliPayQr(_)
                | api_models::payments::WalletData::AliPayRedirect(_)
                | api_models::payments::WalletData::AliPayHkRedirect(_)
                | api_models::payments::WalletData::MomoRedirect(_)
                | api_models::payments::WalletData::KakaoPayRedirect(_)
                | api_models::payments::WalletData::GoPayRedirect(_)
                | api_models::payments::WalletData::GcashRedirect(_)
                | api_models::payments::WalletData::ApplePay(_)
                | api_models::payments::WalletData::ApplePayRedirect(_)
                | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
                | api_models::payments::WalletData::DanaRedirect {}
                | api_models::payments::WalletData::GooglePay(_)
                | api_models::payments::WalletData::GooglePayRedirect(_)
                | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
                | api_models::payments::WalletData::MbWayRedirect(_)
                | api_models::payments::WalletData::MobilePayRedirect(_)
                | api_models::payments::WalletData::PaypalSdk(_)
                | api_models::payments::WalletData::SamsungPay(_)
                | api_models::payments::WalletData::TwintRedirect {}
                | api_models::payments::WalletData::VippsRedirect {}
                | api_models::payments::WalletData::TouchNGoRedirect(_)
                | api_models::payments::WalletData::WeChatPayRedirect(_)
                | api_models::payments::WalletData::WeChatPayQr(_)
                | api_models::payments::WalletData::CashappQr(_)
                | api_models::payments::WalletData::SwishQr(_) => {
                    Err(errors::ConnectorError::NotSupported {
                        message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                        connector: "Paypal",
                    })?
                }
            },
            api::PaymentMethodData::BankRedirect(ref bank_redirection_data) => {
                let intent = if item.router_data.request.is_auto_capture()? {
                    PaypalPaymentIntent::Capture
                } else {
                    Err(errors::ConnectorError::FlowNotSupported {
                        flow: "Manual capture method for Bank Redirect".to_string(),
                        connector: "Paypal".to_string(),
                    })?
                };
                let amount = OrderAmount {
                    currency_code: item.router_data.request.currency,
                    value: item.amount.to_owned(),
                };
                let reference_id = item.router_data.attempt_id.clone();
                let purchase_units = vec![PurchaseUnitRequest {
                    reference_id,
                    amount,
                }];
                let payment_source =
                    Some(get_payment_source(item.router_data, bank_redirection_data)?);

                Ok(Self {
                    intent,
                    purchase_units,
                    payment_source,
                })
            }
            api_models::payments::PaymentMethodData::CardRedirect(ref card_redirect_data) => {
                Self::try_from(card_redirect_data)
            }
            api_models::payments::PaymentMethodData::PayLater(ref paylater_data) => {
                Self::try_from(paylater_data)
            }
            api_models::payments::PaymentMethodData::BankDebit(ref bank_debit_data) => {
                Self::try_from(bank_debit_data)
            }
            api_models::payments::PaymentMethodData::BankTransfer(ref bank_transfer_data) => {
                Self::try_from(bank_transfer_data.as_ref())
            }
            api_models::payments::PaymentMethodData::Voucher(ref voucher_data) => {
                Self::try_from(voucher_data)
            }
            api_models::payments::PaymentMethodData::GiftCard(ref giftcard_data) => {
                Self::try_from(giftcard_data.as_ref())
            }
            api_models::payments::PaymentMethodData::MandatePayment => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Paypal"),
                )
                .into())
            }
            api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::Upi(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Paypal",
                }
                .into())
            }
        }
    }
}

impl TryFrom<&api_models::payments::CardRedirectData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &api_models::payments::CardRedirectData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::CardRedirectData::Knet {}
            | api_models::payments::CardRedirectData::Benefit {}
            | api_models::payments::CardRedirectData::MomoAtm {} => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Paypal",
                }
                .into())
            }
        }
    }
}

impl TryFrom<&api_models::payments::PayLaterData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &api_models::payments::PayLaterData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::PayLaterData::KlarnaRedirect { .. }
            | api_models::payments::PayLaterData::KlarnaSdk { .. }
            | api_models::payments::PayLaterData::AffirmRedirect {}
            | api_models::payments::PayLaterData::AfterpayClearpayRedirect { .. }
            | api_models::payments::PayLaterData::PayBrightRedirect {}
            | api_models::payments::PayLaterData::WalleyRedirect {}
            | api_models::payments::PayLaterData::AlmaRedirect {}
            | api_models::payments::PayLaterData::AtomeRedirect {} => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Paypal",
                }
                .into())
            }
        }
    }
}

impl TryFrom<&api_models::payments::BankDebitData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &api_models::payments::BankDebitData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::BankDebitData::AchBankDebit { .. }
            | api_models::payments::BankDebitData::SepaBankDebit { .. }
            | api_models::payments::BankDebitData::BecsBankDebit { .. }
            | api_models::payments::BankDebitData::BacsBankDebit { .. } => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Paypal",
                }
                .into())
            }
        }
    }
}

impl TryFrom<&api_models::payments::BankTransferData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &api_models::payments::BankTransferData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::BankTransferData::AchBankTransfer { .. }
            | api_models::payments::BankTransferData::SepaBankTransfer { .. }
            | api_models::payments::BankTransferData::BacsBankTransfer { .. }
            | api_models::payments::BankTransferData::MultibancoBankTransfer { .. }
            | api_models::payments::BankTransferData::PermataBankTransfer { .. }
            | api_models::payments::BankTransferData::BcaBankTransfer { .. }
            | api_models::payments::BankTransferData::BniVaBankTransfer { .. }
            | api_models::payments::BankTransferData::BriVaBankTransfer { .. }
            | api_models::payments::BankTransferData::CimbVaBankTransfer { .. }
            | api_models::payments::BankTransferData::DanamonVaBankTransfer { .. }
            | api_models::payments::BankTransferData::MandiriVaBankTransfer { .. }
            | api_models::payments::BankTransferData::Pix {}
            | api_models::payments::BankTransferData::Pse {} => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Paypal",
                }
                .into())
            }
        }
    }
}

impl TryFrom<&api_models::payments::VoucherData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &api_models::payments::VoucherData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::VoucherData::Boleto(_)
            | api_models::payments::VoucherData::Efecty
            | api_models::payments::VoucherData::PagoEfectivo
            | api_models::payments::VoucherData::RedCompra
            | api_models::payments::VoucherData::RedPagos
            | api_models::payments::VoucherData::Alfamart(_)
            | api_models::payments::VoucherData::Indomaret(_)
            | api_models::payments::VoucherData::Oxxo
            | api_models::payments::VoucherData::SevenEleven(_)
            | api_models::payments::VoucherData::Lawson(_)
            | api_models::payments::VoucherData::MiniStop(_)
            | api_models::payments::VoucherData::FamilyMart(_)
            | api_models::payments::VoucherData::Seicomart(_)
            | api_models::payments::VoucherData::PayEasy(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Paypal",
                }
                .into())
            }
        }
    }
}

impl TryFrom<&api_models::payments::GiftCardData> for PaypalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &api_models::payments::GiftCardData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::GiftCardData::Givex(_)
            | api_models::payments::GiftCardData::PaySafeCard {} => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Paypal",
                }
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

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
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
pub struct PaypalAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) key1: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for PaypalAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                key1: key1.to_owned(),
            }),
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
    reference_id: String,
    payments: PaymentsCollection,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaypalOrdersResponse {
    id: String,
    intent: PaypalPaymentIntent,
    status: PaypalOrderStatus,
    purchase_units: Vec<PurchaseUnitItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalLinks {
    href: Option<Url>,
    rel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalRedirectResponse {
    id: String,
    intent: PaypalPaymentIntent,
    status: PaypalOrderStatus,
    links: Vec<PaypalLinks>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PaypalSyncResponse {
    PaypalOrdersSyncResponse(PaypalOrdersResponse),
    PaypalRedirectSyncResponse(PaypalRedirectResponse),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaypalPaymentsSyncResponse {
    id: String,
    status: PaypalPaymentStatus,
    amount: OrderAmount,
    supplementary_data: PaypalSupplementaryData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaypalMeta {
    pub authorize_id: Option<String>,
    pub capture_id: Option<String>,
    pub psync_flow: PaypalPaymentIntent,
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
                    psync_flow: item.response.intent.clone()
                }),
                types::ResponseId::ConnectorTransactionId(item.response.id),
            ),

            PaypalPaymentIntent::Authorize => (
                serde_json::json!(PaypalMeta {
                    authorize_id: Some(id),
                    capture_id: None,
                    psync_flow: item.response.intent.clone()
                }),
                types::ResponseId::ConnectorTransactionId(item.response.id),
            ),
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
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(connector_meta),
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

fn get_redirect_url(
    item: PaypalRedirectResponse,
) -> CustomResult<Option<Url>, errors::ConnectorError> {
    let mut link: Option<Url> = None;
    let link_vec = item.links;
    for item2 in link_vec.iter() {
        if item2.rel == "payer-action" {
            link = item2.href.clone();
        }
    }
    Ok(link)
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PaypalSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaypalSyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            PaypalSyncResponse::PaypalOrdersSyncResponse(response) => {
                Self::try_from(types::ResponseRouterData {
                    response,
                    data: item.data,
                    http_code: item.http_code,
                })
            }
            PaypalSyncResponse::PaypalRedirectSyncResponse(response) => {
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
    TryFrom<types::ResponseRouterData<F, PaypalRedirectResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaypalRedirectResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = storage_enums::AttemptStatus::foreign_from((
            item.response.clone().status,
            item.response.intent.clone(),
        ));
        let link = get_redirect_url(item.response.clone())?;
        let connector_meta = serde_json::json!(PaypalMeta {
            authorize_id: None,
            capture_id: None,
            psync_flow: item.response.intent
        });

        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Some(services::RedirectForm::from((
                    link.ok_or(errors::ConnectorError::ResponseDeserializationFailed)?,
                    services::Method::Get,
                ))),
                mandate_reference: None,
                connector_metadata: Some(connector_meta),
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
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
                    item.response.supplementary_data.related_ids.order_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
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
            value: item.amount.to_owned(),
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
    final_capture: bool,
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
        let amount_captured = item.data.request.amount_to_capture;
        let status = storage_enums::AttemptStatus::from(item.response.status);
        let connector_payment_id: PaypalMeta =
            to_connector_meta(item.data.request.connector_meta.clone())?;
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.data.request.connector_transaction_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(serde_json::json!(PaypalMeta {
                    authorize_id: connector_payment_id.authorize_id,
                    capture_id: Some(item.response.id),
                    psync_flow: PaypalPaymentIntent::Capture
                })),
                network_txn_id: None,
                connector_response_reference_id: None,
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
#[serde(rename_all = "camelCase")]
pub struct PaypalPaymentsCancelResponse {
    id: String,
    status: PaypalCancelStatus,
    amount: Option<OrderAmount>,
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
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
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
                value: item.amount.to_owned(),
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

#[derive(Debug, Clone, Deserialize)]
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
    pub name: String,
    pub message: String,
    pub debug_id: Option<String>,
    pub details: Option<Vec<OrderErrorDetails>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorDetails {
    pub issue: String,
    pub description: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaypalPaymentErrorResponse {
    pub name: String,
    pub message: String,
    pub debug_id: Option<String>,
    pub details: Option<Vec<ErrorDetails>>,
}

#[derive(Deserialize, Debug)]
pub struct PaypalAccessTokenErrorResponse {
    pub error: String,
    pub error_description: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalWebhooksBody {
    pub event_type: PaypalWebhookEventType,
    pub resource: PaypalResource,
}

#[derive(Deserialize, Debug, Serialize)]
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
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum PaypalResource {
    PaypalCardWebhooks(Box<PaypalCardWebhooks>),
    PaypalRedirectsWebhooks(Box<PaypalRedirectsWebhooks>),
    PaypalRefundWebhooks(Box<PaypalRefundWebhooks>),
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
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaypalRedirectsWebhooks {
    pub purchase_units: Vec<PaypalWebhooksPurchaseUnits>,
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

impl From<PaypalWebhookEventType> for api::IncomingWebhookEvent {
    fn from(event: PaypalWebhookEventType) -> Self {
        match event {
            PaypalWebhookEventType::PaymentCaptureCompleted
            | PaypalWebhookEventType::CheckoutOrderCompleted => Self::PaymentIntentSuccess,
            PaypalWebhookEventType::PaymentCapturePending
            | PaypalWebhookEventType::CheckoutOrderApproved
            | PaypalWebhookEventType::CheckoutOrderProcessed => Self::PaymentIntentProcessing,
            PaypalWebhookEventType::PaymentCaptureDeclined => Self::PaymentIntentFailure,
            PaypalWebhookEventType::PaymentCaptureRefunded => Self::RefundSuccess,
            PaypalWebhookEventType::Unknown
            | PaypalWebhookEventType::PaymentAuthorizationCreated
            | PaypalWebhookEventType::PaymentAuthorizationVoided => Self::EventNotSupported,
        }
    }
}
