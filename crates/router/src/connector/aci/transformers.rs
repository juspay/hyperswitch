use std::str::FromStr;

use error_stack::report;
use masking::Secret;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::result_codes::{FAILURE_CODES, PENDING_CODES, SUCCESSFUL_CODES};
use crate::{
    core::errors,
    services,
    types::{self, api, storage::enums},
};

pub struct AciAuthType {
    pub api_key: String,
    pub entity_id: String,
}

impl TryFrom<&types::ConnectorAuthType> for AciAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = item {
            Ok(Self {
                api_key: api_key.to_string(),
                entity_id: key1.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentsRequest {
    pub entity_id: String,
    pub amount: i64,
    pub currency: String,
    pub payment_type: AciPaymentType,
    #[serde(flatten)]
    pub payment_method: PaymentDetails,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciCancelRequest {
    pub entity_id: String,
    pub payment_type: AciPaymentType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PaymentDetails {
    #[serde(rename = "card")]
    AciCard(CardDetails),
    Eps(BankRedirectionPMData),
    Ideal(BankRedirectionPMData),
    Giropay(BankRedirectionPMData),
    Sofort(BankRedirectionPMData),
    #[serde(rename = "bank")]
    Wallet,
    Klarna,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankRedirectionPMData {
    payment_brand: PaymentBrand,
    #[serde(rename = "bankAccount.country")]
    bank_account_country: Option<api_models::enums::CountryCode>,
    #[serde(rename = "bankAccount.bankName")]
    bank_account_bank_name: Option<String>,
    #[serde(rename = "bankAccount.bic")]
    bank_account_bic: Option<String>,
    #[serde(rename = "bankAccount.iban")]
    bank_account_iban: Option<String>,
    shopper_result_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaymentBrand {
    Eps,
    Ideal,
    Giropay,
    Sofortueberweisung,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CardDetails {
    #[serde(rename = "card.number")]
    pub card_number: Secret<String, common_utils::pii::CardNumber>,
    #[serde(rename = "card.holder")]
    pub card_holder: Secret<String>,
    #[serde(rename = "card.expiryMonth")]
    pub card_expiry_month: Secret<String>,
    #[serde(rename = "card.expiryYear")]
    pub card_expiry_year: Secret<String>,
    #[serde(rename = "card.cvv")]
    pub card_cvv: Secret<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct BankDetails {
    #[serde(rename = "bankAccount.holder")]
    pub account_holder: String,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize)]
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

impl TryFrom<&types::PaymentsAuthorizeRouterData> for AciPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let payment_details: PaymentDetails = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => PaymentDetails::AciCard(CardDetails {
                card_number: ccard.card_number,
                card_holder: ccard.card_holder_name,
                card_expiry_month: ccard.card_exp_month,
                card_expiry_year: ccard.card_exp_year,
                card_cvv: ccard.card_cvc,
            }),
            api::PaymentMethodData::PayLater(_) => PaymentDetails::Klarna,
            api::PaymentMethodData::Wallet(_) => PaymentDetails::Wallet,
            api::PaymentMethodData::BankRedirect(ref redirect_banking_data) => {
                match redirect_banking_data {
                    api_models::payments::BankRedirectData::Eps { .. } => {
                        PaymentDetails::Eps(BankRedirectionPMData {
                            payment_brand: PaymentBrand::Eps,
                            bank_account_country: Some(api_models::enums::CountryCode::AT),
                            bank_account_bank_name: None,
                            bank_account_bic: None,
                            bank_account_iban: None,
                            shopper_result_url: item.request.router_return_url.clone(),
                        })
                    }
                    api_models::payments::BankRedirectData::Giropay {
                        bank_account_bic,
                        bank_account_iban,
                        ..
                    } => PaymentDetails::Giropay(BankRedirectionPMData {
                        payment_brand: PaymentBrand::Giropay,
                        bank_account_country: Some(api_models::enums::CountryCode::DE),
                        bank_account_bank_name: None,
                        bank_account_bic: bank_account_bic.clone(),
                        bank_account_iban: bank_account_iban.clone(),
                        shopper_result_url: item.request.router_return_url.clone(),
                    }),
                    api_models::payments::BankRedirectData::Ideal { bank_name, .. } => {
                        PaymentDetails::Ideal(BankRedirectionPMData {
                            payment_brand: PaymentBrand::Ideal,
                            bank_account_country: Some(api_models::enums::CountryCode::NL),
                            bank_account_bank_name: Some(bank_name.to_string()),
                            bank_account_bic: None,
                            bank_account_iban: None,
                            shopper_result_url: item.request.router_return_url.clone(),
                        })
                    }
                    api_models::payments::BankRedirectData::Sofort { country, .. } => {
                        PaymentDetails::Sofort(BankRedirectionPMData {
                            payment_brand: PaymentBrand::Sofortueberweisung,
                            bank_account_country: Some(*country),
                            bank_account_bank_name: None,
                            bank_account_bic: None,
                            bank_account_iban: None,
                            shopper_result_url: item.request.router_return_url.clone(),
                        })
                    }
                }
            }
            api::PaymentMethodData::Crypto(_) => Err(errors::ConnectorError::NotSupported {
                payment_method: format!("{:?}", item.payment_method),
                connector: "Aci",
                payment_experience: api_models::enums::PaymentExperience::RedirectToUrl.to_string(),
            })?,
        };

        let auth = AciAuthType::try_from(&item.connector_auth_type)?;
        let aci_payment_request = Self {
            payment_method: payment_details,
            entity_id: auth.entity_id,
            amount: item.request.amount,
            currency: item.request.currency.to_string(),
            payment_type: AciPaymentType::Debit,
        };
        Ok(aci_payment_request)
    }
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
    // ndc is an internal unique identifier for the request.
    ndc: String,
    timestamp: String,
    build_number: String,
    pub(super) result: ResultCode,
    pub(super) redirect: Option<AciRedirectionData>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AciRedirectionData {
    method: Option<services::Method>,
    parameters: Vec<Parameters>,
    preconditions: Option<Vec<PreConditions>>,
    url: Url,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreConditions {
    origin: String,
    wait_until: String,
    description: String,
    method: services::Method,
    url: String,
    parameters: Parameters,
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
    pub(super) value: String,
    pub(super) message: String,
}

fn method_data(redirection_data: AciRedirectionData) -> services::Method {
    let mut three_ds: bool = false;
    // Check if method exists in 3DS
    if let Some(method_param) = redirection_data
        .parameters
        .iter()
        .find(|param| param.name == *"method")
    {
        three_ds = true;
        // Parse the parameter value as Method enum
        if let Ok(method) = &method_param.value.parse::<services::Method>() {
            return *method;
        }
    }
    if three_ds {
        redirection_data.method.unwrap_or(services::Method::Get)
    } else {
        redirection_data.method.unwrap_or(services::Method::Post)
    }
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
                    .map(|parameter| (parameter.clone().name, parameter.clone().value)),
            );

            let link = data.clone().url;

            services::RedirectForm {
                endpoint: link.to_string(),
                method: method_data(data),
                form_fields,
            }
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
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciRefundRequest {
    pub amount: i64,
    pub currency: String,
    pub payment_type: AciPaymentType,
    pub entity_id: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for AciRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let amount = item.request.refund_amount;
        let currency = item.request.currency;
        let payment_type = AciPaymentType::Refund;
        let auth = AciAuthType::try_from(&item.connector_auth_type)?;

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
