use common_utils::pii::Email;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, missing_field_err, CardData, PaymentsAuthorizeRequestData, RouterData,
    },
    core::errors,
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize)]
pub struct StaxPaymentsRequestMetaData {
    tax: i64,
}

#[derive(Debug, Serialize)]
pub struct StaxPaymentsRequest {
    payment_method_id: Secret<String>,
    total: f64,
    is_refundable: bool,
    pre_auth: bool,
    meta: StaxPaymentsRequestMetaData,
    idempotency_id: Option<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for StaxPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        if item.request.currency != enums::Currency::USD {
            Err(errors::ConnectorError::NotSupported {
                message: item.request.currency.to_string(),
                connector: "Stax",
            })?
        }
        let total = utils::to_currency_base_unit_asf64(item.request.amount, item.request.currency)?;

        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(_) => {
                let pm_token = item.get_payment_method_token()?;
                let pre_auth = !item.request.is_auto_capture()?;
                Ok(Self {
                    meta: StaxPaymentsRequestMetaData { tax: 0 },
                    total,
                    is_refundable: true,
                    pre_auth,
                    payment_method_id: Secret::new(match pm_token {
                        types::PaymentMethodToken::Token(token) => token,
                        types::PaymentMethodToken::ApplePayDecrypt(_) => {
                            Err(errors::ConnectorError::InvalidWalletToken)?
                        }
                    }),
                    idempotency_id: Some(item.connector_request_reference_id.clone()),
                })
            }
            api::PaymentMethodData::BankDebit(
                api_models::payments::BankDebitData::AchBankDebit { .. },
            ) => {
                let pm_token = item.get_payment_method_token()?;
                let pre_auth = !item.request.is_auto_capture()?;
                Ok(Self {
                    meta: StaxPaymentsRequestMetaData { tax: 0 },
                    total,
                    is_refundable: true,
                    pre_auth,
                    payment_method_id: Secret::new(match pm_token {
                        types::PaymentMethodToken::Token(token) => token,
                        types::PaymentMethodToken::ApplePayDecrypt(_) => {
                            Err(errors::ConnectorError::InvalidWalletToken)?
                        }
                    }),
                    idempotency_id: Some(item.connector_request_reference_id.clone()),
                })
            }
            api::PaymentMethodData::BankDebit(
                api_models::payments::BankDebitData::SepaBankDebit { .. },
            )
            | api::PaymentMethodData::BankDebit(
                api_models::payments::BankDebitData::BecsBankDebit { .. },
            )
            | api::PaymentMethodData::BankDebit(
                api_models::payments::BankDebitData::BacsBankDebit { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::AliPayQr {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::AliPayRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::AliPayHkRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::MomoRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::KakaoPayRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::GoPayRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::GcashRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::ApplePay {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::ApplePayRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::ApplePayThirdPartySdk { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::DanaRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::GooglePay {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::GooglePayRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::GooglePayThirdPartySdk { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::MbWayRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::MobilePayRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::PaypalRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::PaypalSdk {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::SamsungPay {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::TwintRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::VippsRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::TouchNGoRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::WeChatPayRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::WeChatPayQr {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::CashappQr {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::SwishQr {
                ..
            })
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::KlarnaRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(api_models::payments::PayLaterData::KlarnaSdk {
                ..
            })
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::AffirmRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::AfterpayClearpayRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::PayBrightRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::WalleyRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::AlmaRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::AtomeRedirect { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::BancontactCard { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Bizum { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Blik { .. },
            )
            | api::PaymentMethodData::BankRedirect(api_models::payments::BankRedirectData::Eps {
                ..
            })
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Giropay { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Ideal { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Interac { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingCzechRepublic { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingFinland { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingPoland { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingSlovakia { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OpenBankingUk { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Przelewy24 { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Sofort { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Trustly { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingFpx { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingThailand { .. },
            )
            | api::PaymentMethodData::BankTransfer(_) // TODO: how to match against boxed enum? box syntax is experimental. And nested match complains that variables need to be matched in all patterns. The only working solution I found would be to double match against a tuple:
            //match (&item.request.payment_method_data, &item.request.payment_method_data)

            //| api::PaymentMethodData::BankTransfer(box api_models::payments::BankTransferData::AchBankTransfer { .. })

            //| api::PaymentMethodData::BankTransfer(boxed_transfer_type) => {
            //    match *boxed_transfer_type {
            //                api_models::payments::BankTransferData::AchBankTransfer { .. }
            //                | api_models::payments::BankTransferData::SepaBankTransfer { .. }
            //                | api_models::payments::BankTransferData::BacsBankTransfer { .. }
            //                | api_models::payments::BankTransferData::MultibancoBankTransfer { .. }
            //                |
            //                api_models::payments::BankTransferData::PermataBankTransfer { .. }

            //                | api_models::payments::BankTransferData::BcaBankTransfer { .. }
            //                |
            //                api_models::payments::BankTransferData::BniVaBankTransfer { .. }
            //                | api_models::payments::BankTransferData::BriVaBankTransfer { .. }
            //                |
            //                api_models::payments::BankTransferData::CimbVaBankTransfer { .. }

            //                | api_models::payments::BankTransferData::DanamonVaBankTransfer { .. }
            //                |
            //                api_models::payments::BankTransferData::MandiriVaBankTransfer { .. }
            //                | api_models::payments::BankTransferData::Pix { .. }
            //                |
            //                api_models::payments::BankTransferData::Pse { .. } => {
            //                    Err(errors::ConnectorError::NotSupported {
            //                        message: "SELECTED_PAYMENT_METHOD".to_string(),
            //                        connector: "Stax",
            //                    })?
            //                }
            //    }
            //},
            //
            | api::PaymentMethodData::Crypto(api_models::payments::CryptoData { .. })
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::Boleto { .. },
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::Efecty,
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::PagoEfectivo,
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::RedCompra,
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::RedPagos,
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::Alfamart{ .. },
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::Indomaret{ .. },
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::Oxxo,
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::SevenEleven{ .. },
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::Lawson{ .. },
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::MiniStop{ .. },
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::FamilyMart{ .. },
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::Seicomart{ .. },
            )
            | api::PaymentMethodData::Voucher(
                api_models::payments::VoucherData::PayEasy{ .. },
            )
            //TODO: same problem as for BankTransfer
            | api::PaymentMethodData::GiftCard(_)
            | api::PaymentMethodData::CardRedirect(
                api_models::payments::CardRedirectData::Knet{ .. },
            )
            | api::PaymentMethodData::CardRedirect(
                api_models::payments::CardRedirectData::Benefit{ .. },
            )
            | api::PaymentMethodData::CardRedirect(
                api_models::payments::CardRedirectData::MomoAtm{ .. },
            )
            | api::PaymentMethodData::Upi( api_models::payments::UpiData{..}) => Err(errors::ConnectorError::NotSupported {
                message: "SELECTED_PAYMENT_METHOD".to_string(),
                connector: "Stax",
            })?,
        }
    }
}

// Auth Struct
pub struct StaxAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for StaxAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StaxCustomerRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    firstname: Option<String>,
}

impl TryFrom<&types::ConnectorCustomerRouterData> for StaxCustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        if item.request.email.is_none() && item.request.name.is_none() {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "email or name",
            })
            .into_report()
        } else {
            Ok(Self {
                email: item.request.email.to_owned(),
                firstname: item.request.name.to_owned(),
            })
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StaxCustomerResponse {
    id: Secret<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StaxCustomerResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StaxCustomerResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id: item.response.id.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct StaxTokenizeData {
    person_name: Secret<String>,
    card_number: cards::CardNumber,
    card_exp: Secret<String>,
    card_cvv: Secret<String>,
    customer_id: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct StaxBankTokenizeData {
    person_name: Secret<String>,
    bank_account: Secret<String>,
    bank_routing: Secret<String>,
    bank_name: api_models::enums::BankNames,
    bank_type: api_models::enums::BankType,
    bank_holder_type: api_models::enums::BankHolderType,
    customer_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
#[serde(rename_all = "lowercase")]
pub enum StaxTokenRequest {
    Card(StaxTokenizeData),
    Bank(StaxBankTokenizeData),
}

impl TryFrom<&types::TokenizationRouterData> for StaxTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        let customer_id = item.get_connector_customer_id()?;
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(card_data) => {
                let stax_card_data = StaxTokenizeData {
                    card_exp: card_data
                        .get_card_expiry_month_year_2_digit_with_delimiter("".to_string()),
                    person_name: card_data.card_holder_name,
                    card_number: card_data.card_number,
                    card_cvv: card_data.card_cvc,
                    customer_id: Secret::new(customer_id),
                };
                Ok(Self::Card(stax_card_data))
            }
            api_models::payments::PaymentMethodData::BankDebit(
                api_models::payments::BankDebitData::AchBankDebit {
                    billing_details,
                    account_number,
                    routing_number,
                    bank_name,
                    bank_type,
                    bank_holder_type,
                    ..
                },
            ) => {
                let stax_bank_data = StaxBankTokenizeData {
                    person_name: billing_details.name,
                    bank_account: account_number,
                    bank_routing: routing_number,
                    bank_name: bank_name.ok_or_else(missing_field_err("bank_name"))?,
                    bank_type: bank_type.ok_or_else(missing_field_err("bank_type"))?,
                    bank_holder_type: bank_holder_type
                        .ok_or_else(missing_field_err("bank_holder_type"))?,
                    customer_id: Secret::new(customer_id),
                };
                Ok(Self::Bank(stax_bank_data))
            }
            api::PaymentMethodData::BankDebit(
                api_models::payments::BankDebitData::SepaBankDebit { .. },
            )
            | api::PaymentMethodData::BankDebit(
                api_models::payments::BankDebitData::BecsBankDebit { .. },
            )
            | api::PaymentMethodData::BankDebit(
                api_models::payments::BankDebitData::BacsBankDebit { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::AliPayQr {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::AliPayRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::AliPayHkRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::MomoRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::KakaoPayRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::GoPayRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::GcashRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::ApplePay {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::ApplePayRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::ApplePayThirdPartySdk { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::DanaRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::GooglePay {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::GooglePayRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::GooglePayThirdPartySdk { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::MbWayRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::MobilePayRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::PaypalRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::PaypalSdk {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::SamsungPay {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::TwintRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::VippsRedirect {
                ..
            })
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::TouchNGoRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(
                api_models::payments::WalletData::WeChatPayRedirect { .. },
            )
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::WeChatPayQr {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::CashappQr {
                ..
            })
            | api::PaymentMethodData::Wallet(api_models::payments::WalletData::SwishQr {
                ..
            })
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::KlarnaRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(api_models::payments::PayLaterData::KlarnaSdk {
                ..
            })
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::AffirmRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::AfterpayClearpayRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::PayBrightRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::WalleyRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::AlmaRedirect { .. },
            )
            | api::PaymentMethodData::PayLater(
                api_models::payments::PayLaterData::AtomeRedirect { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::BancontactCard { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Bizum { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Blik { .. },
            )
            | api::PaymentMethodData::BankRedirect(api_models::payments::BankRedirectData::Eps {
                ..
            })
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Giropay { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Ideal { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Interac { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingCzechRepublic { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingFinland { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingPoland { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingSlovakia { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OpenBankingUk { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Przelewy24 { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Sofort { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::Trustly { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingFpx { .. },
            )
            | api::PaymentMethodData::BankRedirect(
                api_models::payments::BankRedirectData::OnlineBankingThailand { .. },
            )
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(api_models::payments::CryptoData { .. })
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::Boleto {
                ..
            })
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::Efecty)
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::PagoEfectivo)
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::RedCompra)
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::RedPagos)
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::Alfamart {
                ..
            })
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::Indomaret {
                ..
            })
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::Oxxo)
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::SevenEleven {
                ..
            })
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::Lawson {
                ..
            })
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::MiniStop {
                ..
            })
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::FamilyMart {
                ..
            })
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::Seicomart {
                ..
            })
            | api::PaymentMethodData::Voucher(api_models::payments::VoucherData::PayEasy {
                ..
            })
            // TODO: same problem as for BankTransfer
            | api::PaymentMethodData::GiftCard(_)
            | api::PaymentMethodData::CardRedirect(
                api_models::payments::CardRedirectData::Knet{ .. },
            )
            | api::PaymentMethodData::CardRedirect(
                api_models::payments::CardRedirectData::Benefit{ .. },
            )
            | api::PaymentMethodData::CardRedirect(
                api_models::payments::CardRedirectData::MomoAtm{ .. },
            )
            | api::PaymentMethodData::Upi( api_models::payments::UpiData{..}) => Err(errors::ConnectorError::NotSupported {
                message: "SELECTED_PAYMENT_METHOD".to_string(),
                connector: "Stax",
            })?,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StaxTokenResponse {
    id: Secret<String>,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, StaxTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StaxTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TokenizationResponse {
                token: item.response.id.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaxPaymentResponseTypes {
    Charge,
    PreAuth,
}

#[derive(Debug, Deserialize)]
pub struct StaxChildCapture {
    id: String,
}

#[derive(Debug, Deserialize)]
pub struct StaxPaymentsResponse {
    success: bool,
    id: String,
    is_captured: i8,
    is_voided: bool,
    child_captures: Vec<StaxChildCapture>,
    #[serde(rename = "type")]
    payment_response_type: StaxPaymentResponseTypes,
    idempotency_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StaxMetaData {
    pub capture_id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StaxPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StaxPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let mut connector_metadata = None;
        let mut status = match item.response.success {
            true => match item.response.payment_response_type {
                StaxPaymentResponseTypes::Charge => enums::AttemptStatus::Charged,
                StaxPaymentResponseTypes::PreAuth => match item.response.is_captured {
                    0 => enums::AttemptStatus::Authorized,
                    _ => {
                        connector_metadata =
                            item.response.child_captures.first().map(|child_captures| {
                                serde_json::json!(StaxMetaData {
                                    capture_id: child_captures.id.clone()
                                })
                            });
                        enums::AttemptStatus::Charged
                    }
                },
            },
            false => enums::AttemptStatus::Failure,
        };
        if item.response.is_voided {
            status = enums::AttemptStatus::Voided;
        }

        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    item.response.idempotency_id.unwrap_or(item.response.id),
                ),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaxCaptureRequest {
    total: Option<f64>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for StaxCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let total = utils::to_currency_base_unit_asf64(
            item.request.amount_to_capture,
            item.request.currency,
        )?;
        Ok(Self { total: Some(total) })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct StaxRefundRequest {
    pub total: f64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for StaxRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            total: utils::to_currency_base_unit_asf64(
                item.request.refund_amount,
                item.request.currency,
            )?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ChildTransactionsInResponse {
    id: String,
    success: bool,
    created_at: String,
    total: f64,
}
#[derive(Debug, Deserialize)]
pub struct RefundResponse {
    id: String,
    success: bool,
    child_transactions: Vec<ChildTransactionsInResponse>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_amount = utils::to_currency_base_unit_asf64(
            item.data.request.refund_amount,
            item.data.request.currency,
        )
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        let filtered_txn: Vec<&ChildTransactionsInResponse> = item
            .response
            .child_transactions
            .iter()
            .filter(|txn| txn.total == refund_amount)
            .collect();

        let mut refund_txn = filtered_txn
            .first()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

        for child in filtered_txn.iter() {
            if child.created_at > refund_txn.created_at {
                refund_txn = child;
            }
        }

        let refund_status = match refund_txn.success {
            true => enums::RefundStatus::Success,
            false => enums::RefundStatus::Failure,
        };

        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: refund_txn.id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.success {
            true => enums::RefundStatus::Success,
            false => enums::RefundStatus::Failure,
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaxWebhookEventType {
    PreAuth,
    Capture,
    Charge,
    Void,
    Refund,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct StaxWebhookBody {
    #[serde(rename = "type")]
    pub transaction_type: StaxWebhookEventType,
    pub id: String,
    pub auth_id: Option<String>,
    pub success: bool,
}
