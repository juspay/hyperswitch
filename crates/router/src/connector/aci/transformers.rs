use std::str::FromStr;

use api_models::enums::BankNames;
use common_utils::pii::Email;
use error_stack::report;
use masking::Secret;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::result_codes::{FAILURE_CODES, PENDING_CODES, SUCCESSFUL_CODES};
use crate::{
    connector::utils::{self, RouterData},
    core::errors,
    services,
    types::{self, api, storage::enums},
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
pub struct AciRouterData<T> {
    amount: String,
    router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for AciRouterData<T>
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

pub struct AciAuthType {
    pub api_key: Secret<String>,
    pub entity_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for AciAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = item {
            Ok(Self {
                api_key: api_key.to_owned(),
                entity_id: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentsRequest {
    #[serde(flatten)]
    pub txn_details: TransactionDetails,
    #[serde(flatten)]
    pub payment_method: PaymentDetails,
    #[serde(flatten)]
    pub instruction: Option<Instruction>,
    pub shopper_result_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    pub entity_id: Secret<String>,
    pub amount: String,
    pub currency: String,
    pub payment_type: AciPaymentType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciCancelRequest {
    pub entity_id: Secret<String>,
    pub payment_type: AciPaymentType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PaymentDetails {
    #[serde(rename = "card")]
    AciCard(Box<CardDetails>),
    BankRedirect(Box<BankRedirectionPMData>),
    Wallet(Box<WalletPMData>),
    Klarna,
    Mandate,
}

impl TryFrom<&api_models::payments::WalletData> for PaymentDetails {
    type Error = Error;
    fn try_from(wallet_data: &api_models::payments::WalletData) -> Result<Self, Self::Error> {
        let payment_data = match wallet_data {
            api_models::payments::WalletData::MbWayRedirect(data) => {
                Self::Wallet(Box::new(WalletPMData {
                    payment_brand: PaymentBrand::Mbway,
                    account_id: Some(data.telephone_number.clone()),
                }))
            }
            api_models::payments::WalletData::AliPayRedirect { .. } => {
                Self::Wallet(Box::new(WalletPMData {
                    payment_brand: PaymentBrand::AliPay,
                    account_id: None,
                }))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method".to_string(),
            ))?,
        };
        Ok(payment_data)
    }
}

impl
    TryFrom<(
        &AciRouterData<&types::PaymentsAuthorizeRouterData>,
        &api_models::payments::BankRedirectData,
    )> for PaymentDetails
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&types::PaymentsAuthorizeRouterData>,
            &api_models::payments::BankRedirectData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_redirect_data) = value;
        let payment_data = match bank_redirect_data {
            api_models::payments::BankRedirectData::Eps { .. } => {
                Self::BankRedirect(Box::new(BankRedirectionPMData {
                    payment_brand: PaymentBrand::Eps,
                    bank_account_country: Some(api_models::enums::CountryAlpha2::AT),
                    bank_account_bank_name: None,
                    bank_account_bic: None,
                    bank_account_iban: None,
                    billing_country: None,
                    merchant_customer_id: None,
                    merchant_transaction_id: None,
                    customer_email: None,
                }))
            }
            api_models::payments::BankRedirectData::Giropay {
                bank_account_bic,
                bank_account_iban,
                ..
            } => Self::BankRedirect(Box::new(BankRedirectionPMData {
                payment_brand: PaymentBrand::Giropay,
                bank_account_country: Some(api_models::enums::CountryAlpha2::DE),
                bank_account_bank_name: None,
                bank_account_bic: bank_account_bic.clone(),
                bank_account_iban: bank_account_iban.clone(),
                billing_country: None,
                merchant_customer_id: None,
                merchant_transaction_id: None,
                customer_email: None,
            })),
            api_models::payments::BankRedirectData::Ideal { bank_name, .. } => {
                Self::BankRedirect(Box::new(BankRedirectionPMData {
                    payment_brand: PaymentBrand::Ideal,
                    bank_account_country: Some(api_models::enums::CountryAlpha2::NL),
                    bank_account_bank_name: bank_name.to_owned(),
                    bank_account_bic: None,
                    bank_account_iban: None,
                    billing_country: None,
                    merchant_customer_id: None,
                    merchant_transaction_id: None,
                    customer_email: None,
                }))
            }
            api_models::payments::BankRedirectData::Sofort { country, .. } => {
                Self::BankRedirect(Box::new(BankRedirectionPMData {
                    payment_brand: PaymentBrand::Sofortueberweisung,
                    bank_account_country: Some(country.to_owned()),
                    bank_account_bank_name: None,
                    bank_account_bic: None,
                    bank_account_iban: None,
                    billing_country: None,
                    merchant_customer_id: None,
                    merchant_transaction_id: None,
                    customer_email: None,
                }))
            }
            api_models::payments::BankRedirectData::Przelewy24 {
                billing_details, ..
            } => Self::BankRedirect(Box::new(BankRedirectionPMData {
                payment_brand: PaymentBrand::Przelewy,
                bank_account_country: None,
                bank_account_bank_name: None,
                bank_account_bic: None,
                bank_account_iban: None,
                billing_country: None,
                merchant_customer_id: None,
                merchant_transaction_id: None,
                customer_email: billing_details.email.to_owned(),
            })),
            api_models::payments::BankRedirectData::Interac { email, country } => {
                Self::BankRedirect(Box::new(BankRedirectionPMData {
                    payment_brand: PaymentBrand::InteracOnline,
                    bank_account_country: Some(country.to_owned()),
                    bank_account_bank_name: None,
                    bank_account_bic: None,
                    bank_account_iban: None,
                    billing_country: None,
                    merchant_customer_id: None,
                    merchant_transaction_id: None,
                    customer_email: Some(email.to_owned()),
                }))
            }
            api_models::payments::BankRedirectData::Trustly { country } => {
                Self::BankRedirect(Box::new(BankRedirectionPMData {
                    payment_brand: PaymentBrand::Trustly,
                    bank_account_country: None,
                    bank_account_bank_name: None,
                    bank_account_bic: None,
                    bank_account_iban: None,
                    billing_country: Some(country.to_owned()),
                    merchant_customer_id: Some(Secret::new(item.router_data.get_customer_id()?)),
                    merchant_transaction_id: Some(Secret::new(
                        item.router_data.connector_request_reference_id.clone(),
                    )),
                    customer_email: None,
                }))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method".to_string(),
            ))?,
        };
        Ok(payment_data)
    }
}

impl TryFrom<api_models::payments::Card> for PaymentDetails {
    type Error = Error;
    fn try_from(card_data: api_models::payments::Card) -> Result<Self, Self::Error> {
        Ok(Self::AciCard(Box::new(CardDetails {
            card_number: card_data.card_number,
            card_holder: card_data.card_holder_name,
            card_expiry_month: card_data.card_exp_month,
            card_expiry_year: card_data.card_exp_year,
            card_cvv: card_data.card_cvc,
        })))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankRedirectionPMData {
    payment_brand: PaymentBrand,
    #[serde(rename = "bankAccount.country")]
    bank_account_country: Option<api_models::enums::CountryAlpha2>,
    #[serde(rename = "bankAccount.bankName")]
    bank_account_bank_name: Option<BankNames>,
    #[serde(rename = "bankAccount.bic")]
    bank_account_bic: Option<Secret<String>>,
    #[serde(rename = "bankAccount.iban")]
    bank_account_iban: Option<Secret<String>>,
    #[serde(rename = "billing.country")]
    billing_country: Option<api_models::enums::CountryAlpha2>,
    #[serde(rename = "customer.email")]
    customer_email: Option<Email>,
    #[serde(rename = "customer.merchantCustomerId")]
    merchant_customer_id: Option<Secret<String>>,
    merchant_transaction_id: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletPMData {
    payment_brand: PaymentBrand,
    #[serde(rename = "virtualAccount.accountId")]
    account_id: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentBrand {
    Eps,
    Ideal,
    Giropay,
    Sofortueberweisung,
    InteracOnline,
    Przelewy,
    Trustly,
    Mbway,
    #[serde(rename = "ALIPAY")]
    AliPay,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CardDetails {
    #[serde(rename = "card.number")]
    pub card_number: cards::CardNumber,
    #[serde(rename = "card.holder")]
    pub card_holder: Secret<String>,
    #[serde(rename = "card.expiryMonth")]
    pub card_expiry_month: Secret<String>,
    #[serde(rename = "card.expiryYear")]
    pub card_expiry_year: Secret<String>,
    #[serde(rename = "card.cvv")]
    pub card_cvv: Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum InstructionMode {
    Initial,
    Repeated,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum InstructionType {
    Unscheduled,
}

#[derive(Debug, Clone, Serialize)]
pub enum InstructionSource {
    #[serde(rename = "CIT")]
    CardholderInitiatedTransaction,
    #[serde(rename = "MIT")]
    MerchantInitiatedTransaction,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    #[serde(rename = "standingInstruction.mode")]
    mode: InstructionMode,

    #[serde(rename = "standingInstruction.type")]
    transaction_type: InstructionType,

    #[serde(rename = "standingInstruction.source")]
    source: InstructionSource,

    create_registration: Option<bool>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct BankDetails {
    #[serde(rename = "bankAccount.holder")]
    pub account_holder: String,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum AciPaymentType {
    #[serde(rename = "PA")]
    Preauthorization,
    #[default]
    #[serde(rename = "DB")]
    Debit,
    #[serde(rename = "CD")]
    Credit,
    #[serde(rename = "CP")]
    Capture,
    #[serde(rename = "RV")]
    Reversal,
    #[serde(rename = "RF")]
    Refund,
}

impl TryFrom<&AciRouterData<&types::PaymentsAuthorizeRouterData>> for AciPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AciRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ref card_data) => Self::try_from((item, card_data)),
            api::PaymentMethodData::Wallet(ref wallet_data) => Self::try_from((item, wallet_data)),
            api::PaymentMethodData::PayLater(ref pay_later_data) => {
                Self::try_from((item, pay_later_data))
            }
            api::PaymentMethodData::BankRedirect(ref bank_redirect_data) => {
                Self::try_from((item, bank_redirect_data))
            }
            api::PaymentMethodData::MandatePayment => {
                let mandate_id = item.router_data.request.mandate_id.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "mandate_id",
                    },
                )?;
                Self::try_from((item, mandate_id))
            }
            api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::GiftCard(_)
            | api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_) => Err(errors::ConnectorError::NotSupported {
                message: format!("{:?}", item.router_data.payment_method),
                connector: "Aci",
            })?,
        }
    }
}

impl
    TryFrom<(
        &AciRouterData<&types::PaymentsAuthorizeRouterData>,
        &api_models::payments::WalletData,
    )> for AciPaymentsRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&types::PaymentsAuthorizeRouterData>,
            &api_models::payments::WalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, wallet_data) = value;
        let txn_details = get_transaction_details(item)?;
        let payment_method = PaymentDetails::try_from(wallet_data)?;

        Ok(Self {
            txn_details,
            payment_method,
            instruction: None,
            shopper_result_url: item.router_data.request.router_return_url.clone(),
        })
    }
}

impl
    TryFrom<(
        &AciRouterData<&types::PaymentsAuthorizeRouterData>,
        &api_models::payments::BankRedirectData,
    )> for AciPaymentsRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&types::PaymentsAuthorizeRouterData>,
            &api_models::payments::BankRedirectData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_redirect_data) = value;
        let txn_details = get_transaction_details(item)?;
        let payment_method = PaymentDetails::try_from((item, bank_redirect_data))?;

        Ok(Self {
            txn_details,
            payment_method,
            instruction: None,
            shopper_result_url: item.router_data.request.router_return_url.clone(),
        })
    }
}

impl
    TryFrom<(
        &AciRouterData<&types::PaymentsAuthorizeRouterData>,
        &api_models::payments::PayLaterData,
    )> for AciPaymentsRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&types::PaymentsAuthorizeRouterData>,
            &api_models::payments::PayLaterData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, _pay_later_data) = value;
        let txn_details = get_transaction_details(item)?;
        let payment_method = PaymentDetails::Klarna;

        Ok(Self {
            txn_details,
            payment_method,
            instruction: None,
            shopper_result_url: item.router_data.request.router_return_url.clone(),
        })
    }
}

impl
    TryFrom<(
        &AciRouterData<&types::PaymentsAuthorizeRouterData>,
        &api::Card,
    )> for AciPaymentsRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&types::PaymentsAuthorizeRouterData>,
            &api::Card,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, card_data) = value;
        let txn_details = get_transaction_details(item)?;
        let payment_method = PaymentDetails::try_from(card_data.clone())?;
        let instruction = get_instruction_details(item);

        Ok(Self {
            txn_details,
            payment_method,
            instruction,
            shopper_result_url: None,
        })
    }
}

impl
    TryFrom<(
        &AciRouterData<&types::PaymentsAuthorizeRouterData>,
        api_models::payments::MandateIds,
    )> for AciPaymentsRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&types::PaymentsAuthorizeRouterData>,
            api_models::payments::MandateIds,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, _mandate_data) = value;
        let instruction = get_instruction_details(item);
        let txn_details = get_transaction_details(item)?;

        Ok(Self {
            txn_details,
            payment_method: PaymentDetails::Mandate,
            instruction,
            shopper_result_url: item.router_data.request.router_return_url.clone(),
        })
    }
}

fn get_transaction_details(
    item: &AciRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<TransactionDetails, error_stack::Report<errors::ConnectorError>> {
    let auth = AciAuthType::try_from(&item.router_data.connector_auth_type)?;
    Ok(TransactionDetails {
        entity_id: auth.entity_id,
        amount: item.amount.to_owned(),
        currency: item.router_data.request.currency.to_string(),
        payment_type: AciPaymentType::Debit,
    })
}

fn get_instruction_details(
    item: &AciRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Option<Instruction> {
    if item.router_data.request.setup_mandate_details.is_some() {
        return Some(Instruction {
            mode: InstructionMode::Initial,
            transaction_type: InstructionType::Unscheduled,
            source: InstructionSource::CardholderInitiatedTransaction,
            create_registration: Some(true),
        });
    } else if item.router_data.request.mandate_id.is_some() {
        return Some(Instruction {
            mode: InstructionMode::Repeated,
            transaction_type: InstructionType::Unscheduled,
            source: InstructionSource::MerchantInitiatedTransaction,
            create_registration: None,
        });
    }
    None
}

impl TryFrom<&types::PaymentsCancelRouterData> for AciCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = AciAuthType::try_from(&item.connector_auth_type)?;
        let aci_payment_request = Self {
            entity_id: auth.entity_id,
            payment_type: AciPaymentType::Reversal,
        };
        Ok(aci_payment_request)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AciPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
    RedirectShopper,
}

impl From<AciPaymentStatus> for enums::AttemptStatus {
    fn from(item: AciPaymentStatus) -> Self {
        match item {
            AciPaymentStatus::Succeeded => Self::Charged,
            AciPaymentStatus::Failed => Self::Failure,
            AciPaymentStatus::Pending => Self::Authorizing,
            AciPaymentStatus::RedirectShopper => Self::AuthenticationPending,
        }
    }
}
impl FromStr for AciPaymentStatus {
    type Err = error_stack::Report<errors::ConnectorError>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if FAILURE_CODES.contains(&s) {
            Ok(Self::Failed)
        } else if PENDING_CODES.contains(&s) {
            Ok(Self::Pending)
        } else if SUCCESSFUL_CODES.contains(&s) {
            Ok(Self::Succeeded)
        } else {
            Err(report!(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(s.to_owned())
            )))
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentsResponse {
    id: String,
    registration_id: Option<String>,
    // ndc is an internal unique identifier for the request.
    ndc: String,
    timestamp: String,
    build_number: String,
    pub(super) result: ResultCode,
    pub(super) redirect: Option<AciRedirectionData>,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AciErrorResponse {
    ndc: String,
    timestamp: String,
    build_number: String,
    pub(super) result: ResultCode,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AciRedirectionData {
    method: Option<services::Method>,
    parameters: Vec<Parameters>,
    url: Url,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Parameters {
    name: String,
    value: String,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResultCode {
    pub(super) code: String,
    pub(super) description: String,
    pub(super) parameter_errors: Option<Vec<ErrorParameters>>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ErrorParameters {
    pub(super) name: String,
    pub(super) value: Option<String>,
    pub(super) message: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, AciPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, AciPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item.response.redirect.map(|data| {
            let form_fields = std::collections::HashMap::<_, _>::from_iter(
                data.parameters
                    .iter()
                    .map(|parameter| (parameter.name.clone(), parameter.value.clone())),
            );

            // If method is Get, parameters are appended to URL
            // If method is post, we http Post the method to URL
            services::RedirectForm::Form {
                endpoint: data.url.to_string(),
                // Handles method for Bank redirects currently.
                // 3DS response have method within preconditions. That would require replacing below line with a function.
                method: data.method.unwrap_or(services::Method::Post),
                form_fields,
            }
        });

        let mandate_reference = item
            .response
            .registration_id
            .map(|id| types::MandateReference {
                connector_mandate_id: Some(id),
                payment_method_id: None,
            });

        Ok(Self {
            status: {
                if redirection_data.is_some() {
                    enums::AttemptStatus::from(AciPaymentStatus::RedirectShopper)
                } else {
                    enums::AttemptStatus::from(AciPaymentStatus::from_str(
                        &item.response.result.code,
                    )?)
                }
            },
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data,
                mandate_reference,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciRefundRequest {
    pub amount: String,
    pub currency: String,
    pub payment_type: AciPaymentType,
    pub entity_id: Secret<String>,
}

impl<F> TryFrom<&AciRouterData<&types::RefundsRouterData<F>>> for AciRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AciRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let amount = item.amount.to_owned();
        let currency = item.router_data.request.currency;
        let payment_type = AciPaymentType::Refund;
        let auth = AciAuthType::try_from(&item.router_data.connector_auth_type)?;

        Ok(Self {
            amount,
            currency: currency.to_string(),
            payment_type,
            entity_id: auth.entity_id,
        })
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub enum AciRefundStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
}

impl FromStr for AciRefundStatus {
    type Err = error_stack::Report<errors::ConnectorError>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if FAILURE_CODES.contains(&s) {
            Ok(Self::Failed)
        } else if PENDING_CODES.contains(&s) {
            Ok(Self::Pending)
        } else if SUCCESSFUL_CODES.contains(&s) {
            Ok(Self::Succeeded)
        } else {
            Err(report!(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(s.to_owned())
            )))
        }
    }
}

impl From<AciRefundStatus> for enums::RefundStatus {
    fn from(item: AciRefundStatus) -> Self {
        match item {
            AciRefundStatus::Succeeded => Self::Success,
            AciRefundStatus::Failed => Self::Failure,
            AciRefundStatus::Pending => Self::Pending,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AciRefundResponse {
    id: String,
    //ndc is an internal unique identifier for the request.
    ndc: String,
    timestamp: String,
    build_number: String,
    pub(super) result: ResultCode,
}

impl<F> TryFrom<types::RefundsResponseRouterData<F, AciRefundResponse>>
    for types::RefundsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<F, AciRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(AciRefundStatus::from_str(
                    &item.response.result.code,
                )?),
            }),
            ..item.data
        })
    }
}
