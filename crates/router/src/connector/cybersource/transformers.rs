use api_models::payments;
use base64::Engine;
use common_enums::FutureUsage;
use common_utils::{ext_traits::ValueExt, pii};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    connector::utils::{
        self, AddressDetailsData, ApplePayDecrypt, CardData, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, PaymentsPreProcessingData,
        PaymentsSetupMandateRequestData, PaymentsSyncRequestData, RecurringMandateData, RouterData,
    },
    consts,
    core::errors,
    services,
    types::{
        self,
        api::{self, enums as api_enums},
        domain,
        storage::enums,
        transformers::ForeignFrom,
        ApplePayPredecryptData,
    },
};

#[derive(Debug, Serialize)]
pub struct CybersourceRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for CybersourceRouterData<T>
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
        // This conversion function is used at different places in the file, if updating this, keep a check for those
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceZeroMandateRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
}

impl TryFrom<&types::SetupMandateRouterData> for CybersourceZeroMandateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SetupMandateRouterData) -> Result<Self, Self::Error> {
        let email = item.request.get_email()?;
        let bill_to = build_bill_to(item.get_billing()?, email)?;

        let order_information = OrderInformationWithBill {
            amount_details: Amount {
                total_amount: "0".to_string(),
                currency: item.request.currency,
            },
            bill_to: Some(bill_to),
        };
        let (action_list, action_token_types, authorization_options) = (
            Some(vec![CybersourceActionsList::TokenCreate]),
            Some(vec![CybersourceActionsTokenType::PaymentInstrument]),
            Some(CybersourceAuthorizationOptions {
                initiator: Some(CybersourcePaymentInitiator {
                    initiator_type: Some(CybersourcePaymentInitiatorTypes::Customer),
                    credential_stored_on_file: Some(true),
                    stored_credential_used: None,
                }),
                merchant_intitiated_transaction: None,
            }),
        );

        let client_reference_information = ClientReferenceInformation {
            code: Some(item.connector_request_reference_id.clone()),
        };

        let (payment_information, solution) = match item.request.payment_method_data.clone() {
            domain::PaymentMethodData::Card(ccard) => {
                let card_issuer = ccard.get_card_issuer();
                let card_type = match card_issuer {
                    Ok(issuer) => Some(String::from(issuer)),
                    Err(_) => None,
                };
                (
                    PaymentInformation::Cards(CardPaymentInformation {
                        card: Card {
                            number: ccard.card_number,
                            expiration_month: ccard.card_exp_month,
                            expiration_year: ccard.card_exp_year,
                            security_code: ccard.card_cvc,
                            card_type,
                        },
                    }),
                    None,
                )
            }

            domain::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                domain::WalletData::ApplePay(apple_pay_data) => {
                    match item.payment_method_token.clone() {
                        Some(payment_method_token) => match payment_method_token {
                            types::PaymentMethodToken::ApplePayDecrypt(decrypt_data) => {
                                let expiration_month = decrypt_data.get_expiry_month()?;
                                let expiration_year = decrypt_data.get_four_digit_expiry_year()?;
                                (
                                    PaymentInformation::ApplePay(ApplePayPaymentInformation {
                                        tokenized_card: TokenizedCard {
                                            number: decrypt_data.application_primary_account_number,
                                            cryptogram: decrypt_data
                                                .payment_data
                                                .online_payment_cryptogram,
                                            transaction_type: TransactionType::ApplePay,
                                            expiration_year,
                                            expiration_month,
                                        },
                                    }),
                                    Some(PaymentSolution::ApplePay),
                                )
                            }
                            types::PaymentMethodToken::Token(_) => {
                                Err(errors::ConnectorError::InvalidWalletToken)?
                            }
                        },
                        None => (
                            PaymentInformation::ApplePayToken(ApplePayTokenPaymentInformation {
                                fluid_data: FluidData {
                                    value: Secret::from(apple_pay_data.payment_data),
                                },
                                tokenized_card: ApplePayTokenizedCard {
                                    transaction_type: TransactionType::ApplePay,
                                },
                            }),
                            Some(PaymentSolution::ApplePay),
                        ),
                    }
                }
                domain::WalletData::GooglePay(google_pay_data) => (
                    PaymentInformation::GooglePay(GooglePayPaymentInformation {
                        fluid_data: FluidData {
                            value: Secret::from(
                                consts::BASE64_ENGINE
                                    .encode(google_pay_data.tokenization_data.token),
                            ),
                        },
                    }),
                    Some(PaymentSolution::GooglePay),
                ),
                domain::WalletData::AliPayQr(_)
                | domain::WalletData::AliPayRedirect(_)
                | domain::WalletData::AliPayHkRedirect(_)
                | domain::WalletData::MomoRedirect(_)
                | domain::WalletData::KakaoPayRedirect(_)
                | domain::WalletData::GoPayRedirect(_)
                | domain::WalletData::GcashRedirect(_)
                | domain::WalletData::ApplePayRedirect(_)
                | domain::WalletData::ApplePayThirdPartySdk(_)
                | domain::WalletData::DanaRedirect {}
                | domain::WalletData::GooglePayRedirect(_)
                | domain::WalletData::GooglePayThirdPartySdk(_)
                | domain::WalletData::MbWayRedirect(_)
                | domain::WalletData::MobilePayRedirect(_)
                | domain::WalletData::PaypalRedirect(_)
                | domain::WalletData::PaypalSdk(_)
                | domain::WalletData::SamsungPay(_)
                | domain::WalletData::TwintRedirect {}
                | domain::WalletData::VippsRedirect {}
                | domain::WalletData::TouchNGoRedirect(_)
                | domain::WalletData::WeChatPayRedirect(_)
                | domain::WalletData::WeChatPayQr(_)
                | domain::WalletData::CashappQr(_)
                | domain::WalletData::SwishQr(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                ))?,
            },
            domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                ))?
            }
        };

        let processing_information = ProcessingInformation {
            capture: Some(false),
            capture_options: None,
            action_list,
            action_token_types,
            authorization_options,
            commerce_indicator: String::from("internet"),
            payment_solution: solution.map(String::from),
        };
        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    consumer_authentication_information: Option<CybersourceConsumerAuthInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInformation {
    action_list: Option<Vec<CybersourceActionsList>>,
    action_token_types: Option<Vec<CybersourceActionsTokenType>>,
    authorization_options: Option<CybersourceAuthorizationOptions>,
    commerce_indicator: String,
    capture: Option<bool>,
    capture_options: Option<CaptureOptions>,
    payment_solution: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthInformation {
    ucaf_collection_indicator: Option<String>,
    cavv: Option<String>,
    ucaf_authentication_data: Option<Secret<String>>,
    xid: Option<String>,
    directory_server_transaction_id: Option<Secret<String>>,
    specification_version: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantDefinedInformation {
    key: u8,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourceActionsList {
    TokenCreate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CybersourceActionsTokenType {
    PaymentInstrument,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceAuthorizationOptions {
    initiator: Option<CybersourcePaymentInitiator>,
    merchant_intitiated_transaction: Option<MerchantInitiatedTransaction>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantInitiatedTransaction {
    reason: Option<String>,
    //Required for recurring mandates payment
    original_authorized_amount: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentInitiator {
    #[serde(rename = "type")]
    initiator_type: Option<CybersourcePaymentInitiatorTypes>,
    credential_stored_on_file: Option<bool>,
    stored_credential_used: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CybersourcePaymentInitiatorTypes {
    Customer,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureOptions {
    capture_sequence_number: u32,
    total_capture_count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPaymentInformation {
    card: Card,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCard {
    number: Secret<String>,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    cryptogram: Secret<String>,
    transaction_type: TransactionType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayTokenizedCard {
    transaction_type: TransactionType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayTokenPaymentInformation {
    fluid_data: FluidData,
    tokenized_card: ApplePayTokenizedCard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayPaymentInformation {
    tokenized_card: TokenizedCard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandatePaymentInformation {
    payment_instrument: CybersoucrePaymentInstrument,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FluidData {
    value: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayPaymentInformation {
    fluid_data: FluidData,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentInformation {
    Cards(CardPaymentInformation),
    GooglePay(GooglePayPaymentInformation),
    ApplePay(ApplePayPaymentInformation),
    ApplePayToken(ApplePayTokenPaymentInformation),
    MandatePayment(MandatePaymentInformation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CybersoucrePaymentInstrument {
    id: Secret<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: cards::CardNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    security_code: Secret<String>,
    #[serde(rename = "type")]
    card_type: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformationWithBill {
    amount_details: Amount,
    bill_to: Option<BillTo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformationIncrementalAuthorization {
    amount_details: AdditionalAmount,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformation {
    amount_details: Amount,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    total_amount: String,
    currency: api_models::enums::Currency,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalAmount {
    additional_amount: String,
    currency: String,
}

#[derive(Debug, Serialize)]
pub enum PaymentSolution {
    ApplePay,
    GooglePay,
}

#[derive(Debug, Serialize)]
pub enum TransactionType {
    #[serde(rename = "1")]
    ApplePay,
}

impl From<PaymentSolution> for String {
    fn from(solution: PaymentSolution) -> Self {
        let payment_solution = match solution {
            PaymentSolution::ApplePay => "001",
            PaymentSolution::GooglePay => "012",
        };
        payment_solution.to_string()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillTo {
    first_name: Secret<String>,
    last_name: Secret<String>,
    address1: Secret<String>,
    locality: String,
    administrative_area: Secret<String>,
    postal_code: Secret<String>,
    country: api_enums::CountryAlpha2,
    email: pii::Email,
}

impl From<&CybersourceRouterData<&types::PaymentsAuthorizeRouterData>>
    for ClientReferenceInformation
{
    fn from(item: &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>) -> Self {
        Self {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        }
    }
}

impl From<&CybersourceRouterData<&types::PaymentsCompleteAuthorizeRouterData>>
    for ClientReferenceInformation
{
    fn from(item: &CybersourceRouterData<&types::PaymentsCompleteAuthorizeRouterData>) -> Self {
        Self {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        }
    }
}

impl
    TryFrom<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        Option<PaymentSolution>,
        Option<String>,
    )> for ProcessingInformation
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, solution, network): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            Option<PaymentSolution>,
            Option<String>,
        ),
    ) -> Result<Self, Self::Error> {
        let (action_list, action_token_types, authorization_options) = if item
            .router_data
            .request
            .setup_future_usage
            .map_or(false, |future_usage| {
                matches!(future_usage, FutureUsage::OffSession)
            })
            && (item.router_data.request.customer_acceptance.is_some()
                || item
                    .router_data
                    .request
                    .setup_mandate_details
                    .clone()
                    .map_or(false, |mandate_details| {
                        mandate_details.customer_acceptance.is_some()
                    })) {
            (
                Some(vec![CybersourceActionsList::TokenCreate]),
                Some(vec![CybersourceActionsTokenType::PaymentInstrument]),
                Some(CybersourceAuthorizationOptions {
                    initiator: Some(CybersourcePaymentInitiator {
                        initiator_type: Some(CybersourcePaymentInitiatorTypes::Customer),
                        credential_stored_on_file: Some(true),
                        stored_credential_used: None,
                    }),
                    merchant_intitiated_transaction: None,
                }),
            )
        } else if item.router_data.request.connector_mandate_id().is_some() {
            let original_amount = item
                .router_data
                .get_recurring_mandate_payment_data()?
                .get_original_payment_amount()?;
            let original_currency = item
                .router_data
                .get_recurring_mandate_payment_data()?
                .get_original_payment_currency()?;
            (
                None,
                None,
                Some(CybersourceAuthorizationOptions {
                    initiator: None,
                    merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                        reason: None,
                        original_authorized_amount: Some(utils::get_amount_as_string(
                            &types::api::CurrencyUnit::Base,
                            original_amount,
                            original_currency,
                        )?),
                    }),
                }),
            )
        } else {
            (None, None, None)
        };
        let commerce_indicator = match network {
            Some(card_network) => match card_network.to_lowercase().as_str() {
                "amex" => "aesk",
                "discover" => "dipb",
                "mastercard" => "spa",
                "visa" => "internet",
                _ => "internet",
            },
            None => "internet",
        }
        .to_string();
        Ok(Self {
            capture: Some(matches!(
                item.router_data.request.capture_method,
                Some(enums::CaptureMethod::Automatic) | None
            )),
            payment_solution: solution.map(String::from),
            action_list,
            action_token_types,
            authorization_options,
            capture_options: None,
            commerce_indicator,
        })
    }
}

impl
    From<(
        &CybersourceRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
        Option<PaymentSolution>,
        &CybersourceConsumerAuthValidateResponse,
    )> for ProcessingInformation
{
    fn from(
        (item, solution, three_ds_data): (
            &CybersourceRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
            Option<PaymentSolution>,
            &CybersourceConsumerAuthValidateResponse,
        ),
    ) -> Self {
        let (action_list, action_token_types, authorization_options) = if item
            .router_data
            .request
            .setup_future_usage
            .map_or(false, |future_usage| {
                matches!(future_usage, FutureUsage::OffSession)
            })
        //TODO check for customer acceptance also
        {
            (
                Some(vec![CybersourceActionsList::TokenCreate]),
                Some(vec![CybersourceActionsTokenType::PaymentInstrument]),
                Some(CybersourceAuthorizationOptions {
                    initiator: Some(CybersourcePaymentInitiator {
                        initiator_type: Some(CybersourcePaymentInitiatorTypes::Customer),
                        credential_stored_on_file: Some(true),
                        stored_credential_used: None,
                    }),
                    merchant_intitiated_transaction: None,
                }),
            )
        } else {
            (None, None, None)
        };
        Self {
            capture: Some(matches!(
                item.router_data.request.capture_method,
                Some(enums::CaptureMethod::Automatic) | None
            )),
            payment_solution: solution.map(String::from),
            action_list,
            action_token_types,
            authorization_options,
            capture_options: None,
            commerce_indicator: three_ds_data
                .indicator
                .to_owned()
                .unwrap_or(String::from("internet")),
        }
    }
}

impl
    From<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        BillTo,
    )> for OrderInformationWithBill
{
    fn from(
        (item, bill_to): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            BillTo,
        ),
    ) -> Self {
        Self {
            amount_details: Amount {
                total_amount: item.amount.to_owned(),
                currency: item.router_data.request.currency,
            },
            bill_to: Some(bill_to),
        }
    }
}

impl
    From<(
        &CybersourceRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
        BillTo,
    )> for OrderInformationWithBill
{
    fn from(
        (item, bill_to): (
            &CybersourceRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
            BillTo,
        ),
    ) -> Self {
        Self {
            amount_details: Amount {
                total_amount: item.amount.to_owned(),
                currency: item.router_data.request.currency,
            },
            bill_to: Some(bill_to),
        }
    }
}

// for cybersource each item in Billing is mandatory
fn build_bill_to(
    address_details: &payments::Address,
    email: pii::Email,
) -> Result<BillTo, error_stack::Report<errors::ConnectorError>> {
    let address = address_details
        .address
        .as_ref()
        .ok_or_else(utils::missing_field_err("billing.address"))?;
    let mut state = address.to_state_code()?.peek().clone();
    state.truncate(20);
    Ok(BillTo {
        first_name: address.get_first_name()?.to_owned(),
        last_name: address.get_last_name()?.to_owned(),
        address1: address.get_line1()?.to_owned(),
        locality: address.get_city()?.to_owned(),
        administrative_area: Secret::from(state),
        postal_code: address.get_zip()?.to_owned(),
        country: address.get_country()?.to_owned(),
        email,
    })
}

impl ForeignFrom<Value> for Vec<MerchantDefinedInformation> {
    fn foreign_from(metadata: Value) -> Self {
        let hashmap: std::collections::BTreeMap<String, Value> =
            serde_json::from_str(&metadata.to_string())
                .unwrap_or(std::collections::BTreeMap::new());
        let mut vector: Self = Self::new();
        let mut iter = 1;
        for (key, value) in hashmap {
            vector.push(MerchantDefinedInformation {
                key: iter,
                value: format!("{key}={value}"),
            });
            iter += 1;
        }
        vector
    }
}

impl
    TryFrom<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        domain::Card,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            domain::Card,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing()?, email)?;
        let order_information = OrderInformationWithBill::from((item, bill_to));

        let card_issuer = ccard.get_card_issuer();
        let card_type = match card_issuer {
            Ok(issuer) => Some(String::from(issuer)),
            Err(_) => None,
        };

        let payment_information = PaymentInformation::Cards(CardPaymentInformation {
            card: Card {
                number: ccard.card_number,
                expiration_month: ccard.card_exp_month,
                expiration_year: ccard.card_exp_year,
                security_code: ccard.card_cvc,
                card_type,
            },
        });

        let processing_information = ProcessingInformation::try_from((item, None, None))?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information =
            item.router_data.request.metadata.clone().map(|metadata| {
                Vec::<MerchantDefinedInformation>::foreign_from(metadata.peek().to_owned())
            });

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information: None,
            merchant_defined_information,
        })
    }
}

impl
    TryFrom<(
        &CybersourceRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
        domain::Card,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (
            &CybersourceRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
            domain::Card,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing()?, email)?;
        let order_information = OrderInformationWithBill::from((item, bill_to));

        let card_issuer = ccard.get_card_issuer();
        let card_type = match card_issuer {
            Ok(issuer) => Some(String::from(issuer)),
            Err(_) => None,
        };

        let payment_information = PaymentInformation::Cards(CardPaymentInformation {
            card: Card {
                number: ccard.card_number,
                expiration_month: ccard.card_exp_month,
                expiration_year: ccard.card_exp_year,
                security_code: ccard.card_cvc,
                card_type,
            },
        });
        let client_reference_information = ClientReferenceInformation::from(item);

        let three_ds_info: CybersourceThreeDSMetadata = item
            .router_data
            .request
            .connector_meta
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_meta",
            })?
            .parse_value("CybersourceThreeDSMetadata")
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;

        let processing_information =
            ProcessingInformation::from((item, None, &three_ds_info.three_ds_data));

        let consumer_authentication_information = Some(CybersourceConsumerAuthInformation {
            ucaf_collection_indicator: three_ds_info.three_ds_data.ucaf_collection_indicator,
            cavv: three_ds_info.three_ds_data.cavv,
            ucaf_authentication_data: three_ds_info.three_ds_data.ucaf_authentication_data,
            xid: three_ds_info.three_ds_data.xid,
            directory_server_transaction_id: three_ds_info
                .three_ds_data
                .directory_server_transaction_id,
            specification_version: three_ds_info.three_ds_data.specification_version,
        });

        let merchant_defined_information =
            item.router_data.request.metadata.clone().map(|metadata| {
                Vec::<MerchantDefinedInformation>::foreign_from(metadata.peek().to_owned())
            });

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information,
            merchant_defined_information,
        })
    }
}

impl
    TryFrom<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        Box<ApplePayPredecryptData>,
        domain::ApplePayWalletData,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, apple_pay_data, apple_pay_wallet_data): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            Box<ApplePayPredecryptData>,
            domain::ApplePayWalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing()?, email)?;
        let order_information = OrderInformationWithBill::from((item, bill_to));
        let processing_information = ProcessingInformation::try_from((
            item,
            Some(PaymentSolution::ApplePay),
            Some(apple_pay_wallet_data.payment_method.network.clone()),
        ))?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let expiration_month = apple_pay_data.get_expiry_month()?;
        let expiration_year = apple_pay_data.get_four_digit_expiry_year()?;
        let payment_information = PaymentInformation::ApplePay(ApplePayPaymentInformation {
            tokenized_card: TokenizedCard {
                number: apple_pay_data.application_primary_account_number,
                cryptogram: apple_pay_data.payment_data.online_payment_cryptogram,
                transaction_type: TransactionType::ApplePay,
                expiration_year,
                expiration_month,
            },
        });
        let merchant_defined_information =
            item.router_data.request.metadata.clone().map(|metadata| {
                Vec::<MerchantDefinedInformation>::foreign_from(metadata.peek().to_owned())
            });
        let ucaf_collection_indicator = match apple_pay_wallet_data
            .payment_method
            .network
            .to_lowercase()
            .as_str()
        {
            "mastercard" => Some("2".to_string()),
            _ => None,
        };
        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information: Some(CybersourceConsumerAuthInformation {
                ucaf_collection_indicator,
                cavv: None,
                ucaf_authentication_data: None,
                xid: None,
                directory_server_transaction_id: None,
                specification_version: None,
            }),
            merchant_defined_information,
        })
    }
}

impl
    TryFrom<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        domain::GooglePayWalletData,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, google_pay_data): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            domain::GooglePayWalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing()?, email)?;
        let order_information = OrderInformationWithBill::from((item, bill_to));

        let payment_information = PaymentInformation::GooglePay(GooglePayPaymentInformation {
            fluid_data: FluidData {
                value: Secret::from(
                    consts::BASE64_ENGINE.encode(google_pay_data.tokenization_data.token),
                ),
            },
        });
        let processing_information =
            ProcessingInformation::try_from((item, Some(PaymentSolution::GooglePay), None))?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information =
            item.router_data.request.metadata.clone().map(|metadata| {
                Vec::<MerchantDefinedInformation>::foreign_from(metadata.peek().to_owned())
            });

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information: None,
            merchant_defined_information,
        })
    }
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsAuthorizeRouterData>>
    for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.connector_mandate_id() {
            Some(connector_mandate_id) => Self::try_from((item, connector_mandate_id)),
            None => {
                match item.router_data.request.payment_method_data.clone() {
                    domain::PaymentMethodData::Card(ccard) => Self::try_from((item, ccard)),
                    domain::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                        domain::WalletData::ApplePay(apple_pay_data) => {
                            match item.router_data.payment_method_token.clone() {
                                Some(payment_method_token) => match payment_method_token {
                                    types::PaymentMethodToken::ApplePayDecrypt(decrypt_data) => {
                                        Self::try_from((item, decrypt_data, apple_pay_data))
                                    }
                                    types::PaymentMethodToken::Token(_) => {
                                        Err(errors::ConnectorError::InvalidWalletToken)?
                                    }
                                },
                                None => {
                                    let email = item.router_data.request.get_email()?;
                                    let bill_to =
                                        build_bill_to(item.router_data.get_billing()?, email)?;
                                    let order_information =
                                        OrderInformationWithBill::from((item, bill_to));
                                    let processing_information =
                                        ProcessingInformation::try_from((
                                            item,
                                            Some(PaymentSolution::ApplePay),
                                            Some(apple_pay_data.payment_method.network.clone()),
                                        ))?;
                                    let client_reference_information =
                                        ClientReferenceInformation::from(item);
                                    let payment_information = PaymentInformation::ApplePayToken(
                                        ApplePayTokenPaymentInformation {
                                            fluid_data: FluidData {
                                                value: Secret::from(apple_pay_data.payment_data),
                                            },
                                            tokenized_card: ApplePayTokenizedCard {
                                                transaction_type: TransactionType::ApplePay,
                                            },
                                        },
                                    );
                                    let merchant_defined_information =
                                        item.router_data.request.metadata.clone().map(|metadata| {
                                            Vec::<MerchantDefinedInformation>::foreign_from(
                                                metadata.peek().to_owned(),
                                            )
                                        });
                                    let ucaf_collection_indicator = match apple_pay_data
                                        .payment_method
                                        .network
                                        .to_lowercase()
                                        .as_str()
                                    {
                                        "mastercard" => Some("2".to_string()),
                                        _ => None,
                                    };
                                    Ok(Self {
                                        processing_information,
                                        payment_information,
                                        order_information,
                                        client_reference_information,
                                        merchant_defined_information,
                                        consumer_authentication_information: Some(
                                            CybersourceConsumerAuthInformation {
                                                ucaf_collection_indicator,
                                                cavv: None,
                                                ucaf_authentication_data: None,
                                                xid: None,
                                                directory_server_transaction_id: None,
                                                specification_version: None,
                                            },
                                        ),
                                    })
                                }
                            }
                        }
                        domain::WalletData::GooglePay(google_pay_data) => {
                            Self::try_from((item, google_pay_data))
                        }
                        domain::WalletData::AliPayQr(_)
                        | domain::WalletData::AliPayRedirect(_)
                        | domain::WalletData::AliPayHkRedirect(_)
                        | domain::WalletData::MomoRedirect(_)
                        | domain::WalletData::KakaoPayRedirect(_)
                        | domain::WalletData::GoPayRedirect(_)
                        | domain::WalletData::GcashRedirect(_)
                        | domain::WalletData::ApplePayRedirect(_)
                        | domain::WalletData::ApplePayThirdPartySdk(_)
                        | domain::WalletData::DanaRedirect {}
                        | domain::WalletData::GooglePayRedirect(_)
                        | domain::WalletData::GooglePayThirdPartySdk(_)
                        | domain::WalletData::MbWayRedirect(_)
                        | domain::WalletData::MobilePayRedirect(_)
                        | domain::WalletData::PaypalRedirect(_)
                        | domain::WalletData::PaypalSdk(_)
                        | domain::WalletData::SamsungPay(_)
                        | domain::WalletData::TwintRedirect {}
                        | domain::WalletData::VippsRedirect {}
                        | domain::WalletData::TouchNGoRedirect(_)
                        | domain::WalletData::WeChatPayRedirect(_)
                        | domain::WalletData::WeChatPayQr(_)
                        | domain::WalletData::CashappQr(_)
                        | domain::WalletData::SwishQr(_) => {
                            Err(errors::ConnectorError::NotImplemented(
                                utils::get_unimplemented_payment_method_error_message(
                                    "Cybersource",
                                ),
                            )
                            .into())
                        }
                    },
                    // If connector_mandate_id is present MandatePayment will be the PMD, the case will be handled in the first `if` clause.
                    // This is a fallback implementation in the event of catastrophe.
                    domain::PaymentMethodData::MandatePayment => {
                        let connector_mandate_id =
                            item.router_data.request.connector_mandate_id().ok_or(
                                errors::ConnectorError::MissingRequiredField {
                                    field_name: "connector_mandate_id",
                                },
                            )?;
                        Self::try_from((item, connector_mandate_id))
                    }
                    domain::PaymentMethodData::CardRedirect(_)
                    | domain::PaymentMethodData::PayLater(_)
                    | domain::PaymentMethodData::BankRedirect(_)
                    | domain::PaymentMethodData::BankDebit(_)
                    | domain::PaymentMethodData::BankTransfer(_)
                    | domain::PaymentMethodData::Crypto(_)
                    | domain::PaymentMethodData::Reward
                    | domain::PaymentMethodData::Upi(_)
                    | domain::PaymentMethodData::Voucher(_)
                    | domain::PaymentMethodData::GiftCard(_)
                    | domain::PaymentMethodData::CardToken(_) => {
                        Err(errors::ConnectorError::NotImplemented(
                            utils::get_unimplemented_payment_method_error_message("Cybersource"),
                        )
                        .into())
                    }
                }
            }
        }
    }
}

impl
    TryFrom<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        String,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, connector_mandate_id): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            String,
        ),
    ) -> Result<Self, Self::Error> {
        let processing_information = ProcessingInformation::try_from((item, None, None))?;
        let payment_instrument = CybersoucrePaymentInstrument {
            id: connector_mandate_id.into(),
        };
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing()?, email)?;
        let order_information = OrderInformationWithBill::from((item, bill_to));
        let payment_information =
            PaymentInformation::MandatePayment(MandatePaymentInformation { payment_instrument });
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information =
            item.router_data.request.metadata.clone().map(|metadata| {
                Vec::<MerchantDefinedInformation>::foreign_from(metadata.peek().to_owned())
            });
        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            merchant_defined_information,
            consumer_authentication_information: None,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceAuthSetupRequest {
    payment_information: PaymentInformation,
    client_reference_information: ClientReferenceInformation,
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsAuthorizeRouterData>>
    for CybersourceAuthSetupRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            domain::PaymentMethodData::Card(ccard) => {
                let card_issuer = ccard.get_card_issuer();
                let card_type = match card_issuer {
                    Ok(issuer) => Some(String::from(issuer)),
                    Err(_) => None,
                };
                let payment_information = PaymentInformation::Cards(CardPaymentInformation {
                    card: Card {
                        number: ccard.card_number,
                        expiration_month: ccard.card_exp_month,
                        expiration_year: ccard.card_exp_year,
                        security_code: ccard.card_cvc,
                        card_type,
                    },
                });
                let client_reference_information = ClientReferenceInformation::from(item);
                Ok(Self {
                    payment_information,
                    client_reference_information,
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
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsCaptureRequest {
    processing_information: ProcessingInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsIncrementalAuthorizationRequest {
    processing_information: ProcessingInformation,
    order_information: OrderInformationIncrementalAuthorization,
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsCaptureRouterData>>
    for CybersourcePaymentsCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let merchant_defined_information =
            item.router_data.request.metadata.clone().map(|metadata| {
                Vec::<MerchantDefinedInformation>::foreign_from(metadata.peek().to_owned())
            });
        Ok(Self {
            processing_information: ProcessingInformation {
                capture_options: Some(CaptureOptions {
                    capture_sequence_number: 1,
                    total_capture_count: 1,
                }),
                action_list: None,
                action_token_types: None,
                authorization_options: None,
                capture: None,
                commerce_indicator: String::from("internet"),
                payment_solution: None,
            },
            order_information: OrderInformationWithBill {
                amount_details: Amount {
                    total_amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                },
                bill_to: None,
            },
            client_reference_information: ClientReferenceInformation {
                code: Some(item.router_data.connector_request_reference_id.clone()),
            },
            merchant_defined_information,
        })
    }
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsIncrementalAuthorizationRouterData>>
    for CybersourcePaymentsIncrementalAuthorizationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::PaymentsIncrementalAuthorizationRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            processing_information: ProcessingInformation {
                action_list: None,
                action_token_types: None,
                authorization_options: Some(CybersourceAuthorizationOptions {
                    initiator: Some(CybersourcePaymentInitiator {
                        initiator_type: None,
                        credential_stored_on_file: None,
                        stored_credential_used: Some(true),
                    }),
                    merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                        reason: Some("5".to_owned()),
                        original_authorized_amount: None,
                    }),
                }),
                commerce_indicator: String::from("internet"),
                capture: None,
                capture_options: None,
                payment_solution: None,
            },
            order_information: OrderInformationIncrementalAuthorization {
                amount_details: AdditionalAmount {
                    additional_amount: item.amount.clone(),
                    currency: item.router_data.request.currency.to_string(),
                },
            },
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceVoidRequest {
    client_reference_information: ClientReferenceInformation,
    reversal_information: ReversalInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
    // The connector documentation does not mention the merchantDefinedInformation field for Void requests. But this has been still added because it works!
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReversalInformation {
    amount_details: Amount,
    reason: String,
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsCancelRouterData>> for CybersourceVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: &CybersourceRouterData<&types::PaymentsCancelRouterData>,
    ) -> Result<Self, Self::Error> {
        let merchant_defined_information =
            value.router_data.request.metadata.clone().map(|metadata| {
                Vec::<MerchantDefinedInformation>::foreign_from(metadata.peek().to_owned())
            });
        Ok(Self {
            client_reference_information: ClientReferenceInformation {
                code: Some(value.router_data.connector_request_reference_id.clone()),
            },
            reversal_information: ReversalInformation {
                amount_details: Amount {
                    total_amount: value.amount.to_owned(),
                    currency: value.router_data.request.currency.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "Currency",
                        },
                    )?,
                },
                reason: value
                    .router_data
                    .request
                    .cancellation_reason
                    .clone()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "Cancellation Reason",
                    })?,
            },
            merchant_defined_information,
        })
    }
}

pub struct CybersourceAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_account: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for CybersourceAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourcePaymentStatus {
    Authorized,
    Succeeded,
    Failed,
    Voided,
    Reversed,
    Pending,
    Declined,
    Rejected,
    Challenge,
    AuthorizedPendingReview,
    AuthorizedRiskDeclined,
    Transmitted,
    InvalidRequest,
    ServerError,
    PendingAuthentication,
    PendingReview,
    Accepted,
    Cancelled,
    //PartialAuthorized, not being consumed yet.
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourceIncrementalAuthorizationStatus {
    Authorized,
    Declined,
    AuthorizedPendingReview,
}

impl ForeignFrom<(CybersourcePaymentStatus, bool)> for enums::AttemptStatus {
    fn foreign_from((status, capture): (CybersourcePaymentStatus, bool)) -> Self {
        match status {
            CybersourcePaymentStatus::Authorized
            | CybersourcePaymentStatus::AuthorizedPendingReview => {
                if capture {
                    // Because Cybersource will return Payment Status as Authorized even in AutoCapture Payment
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            CybersourcePaymentStatus::Pending => {
                if capture {
                    Self::Charged
                } else {
                    Self::Pending
                }
            }
            CybersourcePaymentStatus::Succeeded | CybersourcePaymentStatus::Transmitted => {
                Self::Charged
            }
            CybersourcePaymentStatus::Voided
            | CybersourcePaymentStatus::Reversed
            | CybersourcePaymentStatus::Cancelled => Self::Voided,
            CybersourcePaymentStatus::Failed
            | CybersourcePaymentStatus::Declined
            | CybersourcePaymentStatus::AuthorizedRiskDeclined
            | CybersourcePaymentStatus::Rejected
            | CybersourcePaymentStatus::InvalidRequest
            | CybersourcePaymentStatus::ServerError => Self::Failure,
            CybersourcePaymentStatus::PendingAuthentication => Self::AuthenticationPending,
            CybersourcePaymentStatus::PendingReview
            | CybersourcePaymentStatus::Challenge
            | CybersourcePaymentStatus::Accepted => Self::Pending,
        }
    }
}

impl From<CybersourceIncrementalAuthorizationStatus> for common_enums::AuthorizationStatus {
    fn from(item: CybersourceIncrementalAuthorizationStatus) -> Self {
        match item {
            CybersourceIncrementalAuthorizationStatus::Authorized
            | CybersourceIncrementalAuthorizationStatus::AuthorizedPendingReview => Self::Success,
            CybersourceIncrementalAuthorizationStatus::Declined => Self::Failure,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CybersourcePaymentsResponse {
    ClientReferenceInformation(CybersourceClientReferenceResponse),
    ErrorInformation(CybersourceErrorInformationResponse),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceClientReferenceResponse {
    id: String,
    status: CybersourcePaymentStatus,
    client_reference_information: ClientReferenceInformation,
    processor_information: Option<ClientProcessorInformation>,
    risk_information: Option<ClientRiskInformation>,
    token_information: Option<CybersourceTokenInformation>,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceErrorInformationResponse {
    id: String,
    error_information: CybersourceErrorInformation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthInformationResponse {
    access_token: String,
    device_data_collection_url: String,
    reference_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientAuthSetupInfoResponse {
    id: String,
    client_reference_information: ClientReferenceInformation,
    consumer_authentication_information: CybersourceConsumerAuthInformationResponse,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CybersourceAuthSetupResponse {
    ClientAuthSetupInfo(ClientAuthSetupInfoResponse),
    ErrorInformation(CybersourceErrorInformationResponse),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsIncrementalAuthorizationResponse {
    status: CybersourceIncrementalAuthorizationStatus,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CybersourceSetupMandatesResponse {
    ClientReferenceInformation(CybersourceClientReferenceResponse),
    ErrorInformation(CybersourceErrorInformationResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReferenceInformation {
    code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientProcessorInformation {
    avs: Option<Avs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Avs {
    code: String,
    code_raw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRiskInformation {
    rules: Option<Vec<ClientRiskInformationRules>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientRiskInformationRules {
    name: Secret<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceTokenInformation {
    payment_instrument: CybersoucrePaymentInstrument,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CybersourceErrorInformation {
    reason: Option<String>,
    message: Option<String>,
}

impl<F, T>
    From<(
        &CybersourceErrorInformationResponse,
        types::ResponseRouterData<F, CybersourcePaymentsResponse, T, types::PaymentsResponseData>,
        Option<enums::AttemptStatus>,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    fn from(
        (error_response, item, transaction_status): (
            &CybersourceErrorInformationResponse,
            types::ResponseRouterData<
                F,
                CybersourcePaymentsResponse,
                T,
                types::PaymentsResponseData,
            >,
            Option<enums::AttemptStatus>,
        ),
    ) -> Self {
        let error_reason = error_response
            .error_information
            .message
            .to_owned()
            .unwrap_or(consts::NO_ERROR_MESSAGE.to_string());
        let error_message = error_response.error_information.reason.to_owned();
        let response = Err(types::ErrorResponse {
            code: error_message
                .clone()
                .unwrap_or(consts::NO_ERROR_CODE.to_string()),
            message: error_message.unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            reason: Some(error_reason),
            status_code: item.http_code,
            attempt_status: None,
            connector_transaction_id: Some(error_response.id.clone()),
        });
        match transaction_status {
            Some(status) => Self {
                response,
                status,
                ..item.data
            },
            None => Self {
                response,
                ..item.data
            },
        }
    }
}

fn get_error_response_if_failure(
    (info_response, status, http_code): (
        &CybersourceClientReferenceResponse,
        enums::AttemptStatus,
        u16,
    ),
) -> Option<types::ErrorResponse> {
    if utils::is_payment_failure(status) {
        Some(types::ErrorResponse::from((
            &info_response.error_information,
            &info_response.risk_information,
            http_code,
            info_response.id.clone(),
        )))
    } else {
        None
    }
}

fn get_payment_response(
    (info_response, status, http_code): (
        &CybersourceClientReferenceResponse,
        enums::AttemptStatus,
        u16,
    ),
) -> Result<types::PaymentsResponseData, types::ErrorResponse> {
    let error_response = get_error_response_if_failure((info_response, status, http_code));
    match error_response {
        Some(error) => Err(error),
        None => {
            let incremental_authorization_allowed =
                Some(status == enums::AttemptStatus::Authorized);
            let mandate_reference =
                info_response
                    .token_information
                    .clone()
                    .map(|token_info| types::MandateReference {
                        connector_mandate_id: Some(token_info.payment_instrument.id.expose()),
                        payment_method_id: None,
                    });
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(info_response.id.clone()),
                redirection_data: None,
                mandate_reference,
                connector_metadata: info_response
                    .processor_information
                    .as_ref()
                    .map(|processor_information| serde_json::json!({"avs_response": processor_information.avs})),
                network_txn_id: None,
                connector_response_reference_id: Some(
                    info_response
                        .client_reference_information
                        .code
                        .clone()
                        .unwrap_or(info_response.id.clone()),
                ),
                incremental_authorization_allowed,
            })
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourcePaymentsResponse::ClientReferenceInformation(info_response) => {
                let status = enums::AttemptStatus::foreign_from((
                    info_response.status.clone(),
                    item.data.request.is_auto_capture()?,
                ));
                let response = get_payment_response((&info_response, status, item.http_code));
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            CybersourcePaymentsResponse::ErrorInformation(ref error_response) => Ok(Self::from((
                &error_response.clone(),
                item,
                Some(enums::AttemptStatus::Failure),
            ))),
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourceAuthSetupResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourceAuthSetupResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourceAuthSetupResponse::ClientAuthSetupInfo(info_response) => Ok(Self {
                status: enums::AttemptStatus::AuthenticationPending,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::NoResponseId,
                    redirection_data: Some(services::RedirectForm::CybersourceAuthSetup {
                        access_token: info_response
                            .consumer_authentication_information
                            .access_token,
                        ddc_url: info_response
                            .consumer_authentication_information
                            .device_data_collection_url,
                        reference_id: info_response
                            .consumer_authentication_information
                            .reference_id,
                    }),
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(
                        info_response
                            .client_reference_information
                            .code
                            .unwrap_or(info_response.id.clone()),
                    ),
                    incremental_authorization_allowed: None,
                }),
                ..item.data
            }),
            CybersourceAuthSetupResponse::ErrorInformation(error_response) => {
                let error_reason = error_response
                    .error_information
                    .message
                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string());
                let error_message = error_response.error_information.reason;
                Ok(Self {
                    response: Err(types::ErrorResponse {
                        code: error_message
                            .clone()
                            .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                        message: error_message.unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                        reason: Some(error_reason),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(error_response.id.clone()),
                    }),
                    status: enums::AttemptStatus::AuthenticationFailed,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthInformationRequest {
    return_url: String,
    reference_id: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceAuthEnrollmentRequest {
    payment_information: PaymentInformation,
    client_reference_information: ClientReferenceInformation,
    consumer_authentication_information: CybersourceConsumerAuthInformationRequest,
    order_information: OrderInformationWithBill,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CybersourceRedirectionAuthResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthInformationValidateRequest {
    authentication_transaction_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceAuthValidateRequest {
    payment_information: PaymentInformation,
    client_reference_information: ClientReferenceInformation,
    consumer_authentication_information: CybersourceConsumerAuthInformationValidateRequest,
    order_information: OrderInformation,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CybersourcePreProcessingRequest {
    AuthEnrollment(CybersourceAuthEnrollmentRequest),
    AuthValidate(CybersourceAuthValidateRequest),
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsPreProcessingRouterData>>
    for CybersourcePreProcessingRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::PaymentsPreProcessingRouterData>,
    ) -> Result<Self, Self::Error> {
        let client_reference_information = ClientReferenceInformation {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        };
        let payment_method_data = item.router_data.request.payment_method_data.clone().ok_or(
            errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "payment_method_data",
            },
        )?;
        let payment_information = match payment_method_data {
            domain::PaymentMethodData::Card(ccard) => {
                let card_issuer = ccard.get_card_issuer();
                let card_type = match card_issuer {
                    Ok(issuer) => Some(String::from(issuer)),
                    Err(_) => None,
                };
                Ok(PaymentInformation::Cards(CardPaymentInformation {
                    card: Card {
                        number: ccard.card_number,
                        expiration_month: ccard.card_exp_month,
                        expiration_year: ccard.card_exp_year,
                        security_code: ccard.card_cvc,
                        card_type,
                    },
                }))
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
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                ))
            }
        }?;

        let redirect_response = item.router_data.request.redirect_response.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "redirect_response",
            },
        )?;

        let amount_details = Amount {
            total_amount: item.amount.clone(),
            currency: item.router_data.request.currency.ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                },
            )?,
        };

        match redirect_response.params {
            Some(param) if !param.clone().peek().is_empty() => {
                let reference_id = param
                    .clone()
                    .peek()
                    .split_once('=')
                    .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                        field_name: "request.redirect_response.params.reference_id",
                    })?
                    .1
                    .to_string();
                let email = item.router_data.request.get_email()?;
                let bill_to = build_bill_to(item.router_data.get_billing()?, email)?;
                let order_information = OrderInformationWithBill {
                    amount_details,
                    bill_to: Some(bill_to),
                };
                Ok(Self::AuthEnrollment(CybersourceAuthEnrollmentRequest {
                    payment_information,
                    client_reference_information,
                    consumer_authentication_information:
                        CybersourceConsumerAuthInformationRequest {
                            return_url: item.router_data.request.get_complete_authorize_url()?,
                            reference_id,
                        },
                    order_information,
                }))
            }
            Some(_) | None => {
                let redirect_payload: CybersourceRedirectionAuthResponse = redirect_response
                    .payload
                    .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                        field_name: "request.redirect_response.payload",
                    })?
                    .peek()
                    .clone()
                    .parse_value("CybersourceRedirectionAuthResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                let order_information = OrderInformation { amount_details };
                Ok(Self::AuthValidate(CybersourceAuthValidateRequest {
                    payment_information,
                    client_reference_information,
                    consumer_authentication_information:
                        CybersourceConsumerAuthInformationValidateRequest {
                            authentication_transaction_id: redirect_payload.transaction_id,
                        },
                    order_information,
                }))
            }
        }
    }
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsCompleteAuthorizeRouterData>>
    for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_method_data = item.router_data.request.payment_method_data.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_data",
            },
        )?;
        match payment_method_data {
            domain::PaymentMethodData::Card(ccard) => Self::try_from((item, ccard)),
            domain::PaymentMethodData::Wallet(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourceAuthEnrollmentStatus {
    PendingAuthentication,
    AuthenticationSuccessful,
    AuthenticationFailed,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthValidateResponse {
    ucaf_collection_indicator: Option<String>,
    cavv: Option<String>,
    ucaf_authentication_data: Option<Secret<String>>,
    xid: Option<String>,
    specification_version: Option<String>,
    directory_server_transaction_id: Option<Secret<String>>,
    indicator: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CybersourceThreeDSMetadata {
    three_ds_data: CybersourceConsumerAuthValidateResponse,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthInformationEnrollmentResponse {
    access_token: Option<Secret<String>>,
    step_up_url: Option<String>,
    //Added to segregate the three_ds_data in a separate struct
    #[serde(flatten)]
    validate_response: CybersourceConsumerAuthValidateResponse,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientAuthCheckInfoResponse {
    id: String,
    client_reference_information: ClientReferenceInformation,
    consumer_authentication_information: CybersourceConsumerAuthInformationEnrollmentResponse,
    status: CybersourceAuthEnrollmentStatus,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CybersourcePreProcessingResponse {
    ClientAuthCheckInfo(Box<ClientAuthCheckInfoResponse>),
    ErrorInformation(CybersourceErrorInformationResponse),
}

impl From<CybersourceAuthEnrollmentStatus> for enums::AttemptStatus {
    fn from(item: CybersourceAuthEnrollmentStatus) -> Self {
        match item {
            CybersourceAuthEnrollmentStatus::PendingAuthentication => Self::AuthenticationPending,
            CybersourceAuthEnrollmentStatus::AuthenticationSuccessful => {
                Self::AuthenticationSuccessful
            }
            CybersourceAuthEnrollmentStatus::AuthenticationFailed => Self::AuthenticationFailed,
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourcePreProcessingResponse,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsPreProcessingData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourcePreProcessingResponse,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourcePreProcessingResponse::ClientAuthCheckInfo(info_response) => {
                let status = enums::AttemptStatus::from(info_response.status);
                let risk_info: Option<ClientRiskInformation> = None;
                if utils::is_payment_failure(status) {
                    let response = Err(types::ErrorResponse::from((
                        &info_response.error_information,
                        &risk_info,
                        item.http_code,
                        info_response.id.clone(),
                    )));

                    Ok(Self {
                        status,
                        response,
                        ..item.data
                    })
                } else {
                    let connector_response_reference_id = Some(
                        info_response
                            .client_reference_information
                            .code
                            .unwrap_or(info_response.id.clone()),
                    );

                    let redirection_data = match (
                        info_response
                            .consumer_authentication_information
                            .access_token,
                        info_response
                            .consumer_authentication_information
                            .step_up_url,
                    ) {
                        (Some(token), Some(step_up_url)) => {
                            Some(services::RedirectForm::CybersourceConsumerAuth {
                                access_token: token.expose(),
                                step_up_url,
                            })
                        }
                        _ => None,
                    };
                    let three_ds_data = serde_json::to_value(
                        info_response
                            .consumer_authentication_information
                            .validate_response,
                    )
                    .into_report()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
                    Ok(Self {
                        status,
                        response: Ok(types::PaymentsResponseData::TransactionResponse {
                            resource_id: types::ResponseId::NoResponseId,
                            redirection_data,
                            mandate_reference: None,
                            connector_metadata: Some(serde_json::json!({
                                "three_ds_data": three_ds_data
                            })),
                            network_txn_id: None,
                            connector_response_reference_id,
                            incremental_authorization_allowed: None,
                        }),
                        ..item.data
                    })
                }
            }
            CybersourcePreProcessingResponse::ErrorInformation(ref error_response) => {
                let error_reason = error_response
                    .error_information
                    .message
                    .to_owned()
                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string());
                let error_message = error_response.error_information.reason.to_owned();
                let response = Err(types::ErrorResponse {
                    code: error_message
                        .clone()
                        .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                    message: error_message.unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    reason: Some(error_reason),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(error_response.id.clone()),
                });
                Ok(Self {
                    response,
                    status: enums::AttemptStatus::AuthenticationFailed,
                    ..item.data
                })
            }
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourcePaymentsResponse::ClientReferenceInformation(info_response) => {
                let status = enums::AttemptStatus::foreign_from((
                    info_response.status.clone(),
                    item.data.request.is_auto_capture()?,
                ));
                let response = get_payment_response((&info_response, status, item.http_code));
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            CybersourcePaymentsResponse::ErrorInformation(ref error_response) => Ok(Self::from((
                &error_response.clone(),
                item,
                Some(enums::AttemptStatus::Failure),
            ))),
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourcePaymentsResponse::ClientReferenceInformation(info_response) => {
                let status =
                    enums::AttemptStatus::foreign_from((info_response.status.clone(), true));
                let response = get_payment_response((&info_response, status, item.http_code));
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            CybersourcePaymentsResponse::ErrorInformation(ref error_response) => {
                Ok(Self::from((&error_response.clone(), item, None)))
            }
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsCancelData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsCancelData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsCancelData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourcePaymentsResponse::ClientReferenceInformation(info_response) => {
                let status =
                    enums::AttemptStatus::foreign_from((info_response.status.clone(), false));
                let response = get_payment_response((&info_response, status, item.http_code));
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            CybersourcePaymentsResponse::ErrorInformation(ref error_response) => {
                Ok(Self::from((&error_response.clone(), item, None)))
            }
        }
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourceSetupMandatesResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourceSetupMandatesResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourceSetupMandatesResponse::ClientReferenceInformation(info_response) => {
                let mandate_reference = info_response.token_information.clone().map(|token_info| {
                    types::MandateReference {
                        connector_mandate_id: Some(token_info.payment_instrument.id.expose()),
                        payment_method_id: None,
                    }
                });
                let mut mandate_status =
                    enums::AttemptStatus::foreign_from((info_response.status.clone(), false));
                if matches!(mandate_status, enums::AttemptStatus::Authorized) {
                    //In case of zero auth mandates we want to make the payment reach the terminal status so we are converting the authorized status to charged as well.
                    mandate_status = enums::AttemptStatus::Charged
                }
                let error_response =
                    get_error_response_if_failure((&info_response, mandate_status, item.http_code));

                Ok(Self {
                    status: mandate_status,
                    response: match error_response {
                        Some(error) => Err(error),
                        None => Ok(types::PaymentsResponseData::TransactionResponse {
                            resource_id: types::ResponseId::ConnectorTransactionId(
                                info_response.id.clone(),
                            ),
                            redirection_data: None,
                            mandate_reference,
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: Some(
                                info_response
                                    .client_reference_information
                                    .code
                                    .clone()
                                    .unwrap_or(info_response.id),
                            ),
                            incremental_authorization_allowed: Some(
                                mandate_status == enums::AttemptStatus::Authorized,
                            ),
                        }),
                    },
                    ..item.data
                })
            }
            CybersourceSetupMandatesResponse::ErrorInformation(ref error_response) => {
                let error_reason = error_response
                    .error_information
                    .message
                    .to_owned()
                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string());
                let error_message = error_response.error_information.reason.to_owned();
                let response = Err(types::ErrorResponse {
                    code: error_message
                        .clone()
                        .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                    message: error_message.unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    reason: Some(error_reason),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(error_response.id.clone()),
                });
                Ok(Self {
                    response,
                    status: enums::AttemptStatus::Failure,
                    ..item.data
                })
            }
        }
    }
}

impl<F, T>
    TryFrom<(
        types::ResponseRouterData<
            F,
            CybersourcePaymentsIncrementalAuthorizationResponse,
            T,
            types::PaymentsResponseData,
        >,
        bool,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            types::ResponseRouterData<
                F,
                CybersourcePaymentsIncrementalAuthorizationResponse,
                T,
                types::PaymentsResponseData,
            >,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        Ok(Self {
            response: match item.response.error_information {
                Some(error) => Ok(
                    types::PaymentsResponseData::IncrementalAuthorizationResponse {
                        status: common_enums::AuthorizationStatus::Failure,
                        error_code: error.reason,
                        error_message: error.message,
                        connector_authorization_id: None,
                    },
                ),
                _ => Ok(
                    types::PaymentsResponseData::IncrementalAuthorizationResponse {
                        status: item.response.status.into(),
                        error_code: None,
                        error_message: None,
                        connector_authorization_id: None,
                    },
                ),
            },
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CybersourceTransactionResponse {
    ApplicationInformation(CybersourceApplicationInfoResponse),
    ErrorInformation(CybersourceErrorInformationResponse),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceApplicationInfoResponse {
    id: String,
    application_information: ApplicationInformation,
    client_reference_information: Option<ClientReferenceInformation>,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInformation {
    status: CybersourcePaymentStatus,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourceTransactionResponse,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourceTransactionResponse,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourceTransactionResponse::ApplicationInformation(app_response) => {
                let status = enums::AttemptStatus::foreign_from((
                    app_response.application_information.status,
                    item.data.request.is_auto_capture()?,
                ));
                let incremental_authorization_allowed =
                    Some(status == enums::AttemptStatus::Authorized);
                let risk_info: Option<ClientRiskInformation> = None;
                if utils::is_payment_failure(status) {
                    Ok(Self {
                        response: Err(types::ErrorResponse::from((
                            &app_response.error_information,
                            &risk_info,
                            item.http_code,
                            app_response.id.clone(),
                        ))),
                        status: enums::AttemptStatus::Failure,
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        status,
                        response: Ok(types::PaymentsResponseData::TransactionResponse {
                            resource_id: types::ResponseId::ConnectorTransactionId(
                                app_response.id.clone(),
                            ),
                            redirection_data: None,
                            mandate_reference: None,
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: app_response
                                .client_reference_information
                                .map(|cref| cref.code)
                                .unwrap_or(Some(app_response.id)),
                            incremental_authorization_allowed,
                        }),
                        ..item.data
                    })
                }
            }
            CybersourceTransactionResponse::ErrorInformation(error_response) => Ok(Self {
                status: item.data.status,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        error_response.id.clone(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(error_response.id),
                    incremental_authorization_allowed: None,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceRefundRequest {
    order_information: OrderInformation,
    client_reference_information: ClientReferenceInformation,
}

impl<F> TryFrom<&CybersourceRouterData<&types::RefundsRouterData<F>>> for CybersourceRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            order_information: OrderInformation {
                amount_details: Amount {
                    total_amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                },
            },
            client_reference_information: ClientReferenceInformation {
                code: Some(item.router_data.request.refund_id.clone()),
            },
        })
    }
}

impl From<CybersourceRefundStatus> for enums::RefundStatus {
    fn from(item: CybersourceRefundStatus) -> Self {
        match item {
            CybersourceRefundStatus::Succeeded | CybersourceRefundStatus::Transmitted => {
                Self::Success
            }
            CybersourceRefundStatus::Failed | CybersourceRefundStatus::Voided => Self::Failure,
            CybersourceRefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourceRefundStatus {
    Succeeded,
    Transmitted,
    Failed,
    Pending,
    Voided,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceRefundResponse {
    id: String,
    status: CybersourceRefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, CybersourceRefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, CybersourceRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RsyncApplicationInformation {
    status: CybersourceRefundStatus,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceRsyncResponse {
    id: String,
    application_information: RsyncApplicationInformation,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, CybersourceRsyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, CybersourceRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(
                    item.response.application_information.status,
                ),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceStandardErrorResponse {
    pub error_information: Option<ErrorInformation>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceNotAvailableErrorResponse {
    pub errors: Vec<CybersourceNotAvailableErrorObject>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceNotAvailableErrorObject {
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceServerErrorResponse {
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<Reason>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Reason {
    SystemError,
    ServerTimeout,
    ServiceTimeout,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CybersourceAuthenticationErrorResponse {
    pub response: AuthenticationErrorInformation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CybersourceErrorResponse {
    AuthenticationError(CybersourceAuthenticationErrorResponse),
    //If the request resource is not available/exists in cybersource
    NotAvailableError(CybersourceNotAvailableErrorResponse),
    StandardError(CybersourceStandardErrorResponse),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub field: String,
    pub reason: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ErrorInformation {
    pub message: String,
    pub reason: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AuthenticationErrorInformation {
    pub rmsg: String,
}

impl
    From<(
        &Option<CybersourceErrorInformation>,
        &Option<ClientRiskInformation>,
        u16,
        String,
    )> for types::ErrorResponse
{
    fn from(
        (error_data, risk_information, status_code, transaction_id): (
            &Option<CybersourceErrorInformation>,
            &Option<ClientRiskInformation>,
            u16,
            String,
        ),
    ) -> Self {
        let avs_message = risk_information
            .clone()
            .map(|client_risk_information| {
                client_risk_information.rules.map(|rules| {
                    rules
                        .iter()
                        .map(|risk_info| format!(" , {}", risk_info.name.clone().expose()))
                        .collect::<Vec<String>>()
                        .join("")
                })
            })
            .unwrap_or(Some("".to_string()));
        let error_reason = error_data
            .clone()
            .map(|error_details| {
                error_details.message.unwrap_or("".to_string())
                    + &avs_message.unwrap_or("".to_string())
            })
            .unwrap_or(consts::NO_ERROR_MESSAGE.to_string());
        let error_message = error_data
            .clone()
            .and_then(|error_details| error_details.reason);

        Self {
            code: error_message
                .clone()
                .unwrap_or(consts::NO_ERROR_CODE.to_string()),
            message: error_message
                .clone()
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            reason: Some(error_reason.clone()),
            status_code,
            attempt_status: Some(enums::AttemptStatus::Failure),
            connector_transaction_id: Some(transaction_id.clone()),
        }
    }
}
