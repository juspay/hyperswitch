use api_models::payouts::{
    self, AchBankTransfer, BacsBankTransfer, PayoutMethodData, SepaBankTransfer,
};
use common_enums::{enums, CountryAlpha2, Currency};
use common_utils::{
    ext_traits::OptionExt,
    id_type::PayoutId,
    pii::Email,
    types::{FloatMajorUnit, StringMinorUnit},
};
use error_stack::ResultExt;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::router_flow_types::PoFulfill;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{CustomerDetails, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PayoutsResponseData, PayoutsRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

#[cfg(feature = "payouts")]
use crate::types::PayoutsResponseRouterData;
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::RouterData as _,
};

pub struct EnvoyRouterData<T> {
    pub amount: FloatMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for EnvoyRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct EnvoyPaymentsRequest {
    amount: StringMinorUnit,
    card: EnvoyCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct EnvoyCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&EnvoyRouterData<&PaymentsAuthorizeRouterData>> for EnvoyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &EnvoyRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(_) => Err(errors::ConnectorError::NotImplemented(
                "Card payment method not implemented".to_string(),
            )
            .into()),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvoyAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for EnvoyAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                username: key1.to_owned(),
                password: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EnvoyPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<EnvoyPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: EnvoyPaymentStatus) -> Self {
        match item {
            EnvoyPaymentStatus::Succeeded => Self::Charged,
            EnvoyPaymentStatus::Failed => Self::Failure,
            EnvoyPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnvoyPaymentsResponse {
    status: EnvoyPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, EnvoyPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, EnvoyPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct EnvoyRefundRequest {
    pub amount: FloatMajorUnit,
}

impl<F> TryFrom<&EnvoyRouterData<&RefundsRouterData<F>>> for EnvoyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &EnvoyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnvoyErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}

//--------------------- SEPA PAYOUTS ---------------------
// https://docs.worldpay.com/apis/pushtoaccountglobal/reference/paytobankaccountv3#request-schema
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "soap:Envelope")]
pub struct SoapEnvelope {
    #[serde(rename = "@xmlns:xsi")]
    pub xsi: String,
    #[serde(rename = "@xmlns:xsd")]
    pub xsd: String,
    #[serde(rename = "@xmlns:soap")]
    pub soap: String,
    #[serde(rename = "soap:Body")]
    pub body: SoapBody,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoapBody {
    #[serde(rename = "payToBankAccountV3")]
    pub request: PayToBankAccountV3,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayToBankAccountV3 {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    pub auth: EnvoyAuthType,
    pub request_reference: PayoutId,
    pub payment_instructions: PaymentInstructions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentInstructions {
    #[serde(rename = "paymentInstructionV3")]
    pub instructions: Vec<PaymentInstructionV3>,
}

// --- 2. Payment Instruction & Details ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInstructionV3 {
    pub payment_details: PaymentDetails,
    pub payment_template: PaymentTemplate, // Required per your request to skip ItemID
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDetails {
    pub country_code: CountryAlpha2,
    pub source_currency: Currency,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_amount: Option<FloatMajorUnit>,
    pub target_currency: Currency,
    pub target_amount: FloatMajorUnit,
    pub source_or_target: SourceOrTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_reference: Option<String>,
    pub payment_reference: PayoutId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<Secret<String>>,
    pub fast_payment: YesNo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SourceOrTarget {
    #[serde(rename = "S")]
    Source,
    #[serde(rename = "T")]
    Target,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum YesNo {
    #[serde(rename = "Y")]
    Yes,
    #[serde(rename = "N")]
    No,
}

// --- 4. Bank Template (Replacement for PaymentItemID) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PaymentTemplate {
    #[serde(rename = "Row")]
    pub rows: Vec<TemplateRow>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateRow {
    #[serde(rename = "@Id")]
    pub id: BankField,
    #[serde(rename = "@Value")]
    pub value: Option<Secret<String>>,
}

/// Ref: https://docs.worldpay.com/apis/pushtoaccountglobal/reference/paymenttemplatefields
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BankField {
    #[serde(rename = "IBAN")]
    Iban,
    #[serde(rename = "SWIFT")]
    Swift,
    #[serde(rename = "BankAccountNumber")]
    BankAccountNumber,
    #[serde(rename = "BranchAddress")]
    BranchAddress,
    #[serde(rename = "BankCode")]
    BankCode,
    #[serde(rename = "BranchCode")]
    BranchCode,
    #[serde(rename = "BankName")]
    BankName,
    // Customer fields mandatory
    // String up to 50 characters Mandatory
    #[serde(rename = "CustomerName")]
    CustomerName,
}

fn get_template_for_ach(
    bank: AchBankTransfer,
    customer_details: Option<&CustomerDetails>,
) -> Result<Vec<TemplateRow>, error_stack::Report<errors::ConnectorError>> {
    let customer_name = customer_details
        .and_then(|c| c.name.clone())
        .get_required_value("customer_name")
        .change_context(errors::ConnectorError::MissingRequiredField {
            field_name: "customer_name",
        })?;

    Ok(vec![
        TemplateRow {
            id: BankField::BranchAddress,
            value: bank.bank_city.map(Secret::new),
        },
        TemplateRow {
            id: BankField::CustomerName,
            value: Some(customer_name.clone()),
        },
        TemplateRow {
            id: BankField::BankName,
            value: bank.bank_name.map(Secret::new),
        },
        TemplateRow {
            id: BankField::BankCode,
            value: Some(bank.bank_routing_number.clone()),
        },
        TemplateRow {
            id: BankField::BankAccountNumber,
            value: Some(bank.bank_account_number.clone()),
        },
    ])
}

fn get_template_for_bacs(
    bank: BacsBankTransfer,
    customer_details: Option<&CustomerDetails>,
) -> Result<Vec<TemplateRow>, error_stack::Report<errors::ConnectorError>> {
    let customer_name = customer_details
        .and_then(|c| c.name.clone())
        .get_required_value("customer_name")
        .change_context(errors::ConnectorError::MissingRequiredField {
            field_name: "customer_name",
        })?;

    Ok(vec![
        TemplateRow {
            id: BankField::BranchAddress,
            value: bank.bank_city.map(Secret::new),
        },
        TemplateRow {
            id: BankField::CustomerName,
            value: Some(customer_name.clone()),
        },
        TemplateRow {
            id: BankField::BankName,
            value: bank.bank_name.map(Secret::new),
        },
        TemplateRow {
            id: BankField::BankCode,
            value: Some(bank.bank_sort_code.clone()),
        },
        TemplateRow {
            id: BankField::BankAccountNumber,
            value: Some(bank.bank_account_number.clone()),
        },
    ])
}
fn get_template_for_sepa(
    bank: SepaBankTransfer,
    customer_details: Option<&CustomerDetails>,
) -> Result<Vec<TemplateRow>, error_stack::Report<errors::ConnectorError>> {
    let customer_name = customer_details
        .and_then(|c| c.name.clone())
        .get_required_value("customer_name")
        .change_context(errors::ConnectorError::MissingRequiredField {
            field_name: "customer_name",
        })?;

    Ok(vec![
        TemplateRow {
            id: BankField::BranchAddress,
            value: bank.bank_city.map(Secret::new),
        },
        TemplateRow {
            id: BankField::Iban,
            value: Some(bank.iban.clone()),
        },
        TemplateRow {
            id: BankField::CustomerName,
            value: Some(customer_name.clone()),
        },
        TemplateRow {
            id: BankField::BankName,
            value: bank.bank_name.map(Secret::new),
        },
        TemplateRow {
            id: BankField::Swift,
            value: bank.bic.clone(),
        },
    ])
}

// --- Helper implementation for initialization ---

impl SoapEnvelope {
    pub fn new(request: PayToBankAccountV3) -> Self {
        Self {
            xsi: "http://www.w3.org/2001/XMLSchema-instance".to_string(),
            xsd: "http://www.w3.org/2001/XMLSchema".to_string(),
            soap: "http://schemas.xmlsoap.org/soap/envelope/".to_string(),
            body: SoapBody { request },
        }
    }
}

// --- Payout Request Implementation ---

impl<F> TryFrom<&EnvoyRouterData<&PayoutsRouterData<F>>> for PayToBankAccountV3 {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &EnvoyRouterData<&PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let payout_data = &item.router_data.request;
        let customer_details = item.router_data.request.customer_details.to_owned();
        let payment_template = match item.router_data.get_payout_method_data()? {
            PayoutMethodData::Bank(payouts::Bank::Ach(bank)) => PaymentTemplate {
                rows: get_template_for_ach(bank, customer_details.as_ref())?,
            },
            PayoutMethodData::Bank(payouts::Bank::Sepa(bank)) => PaymentTemplate {
                rows: get_template_for_sepa(bank, customer_details.as_ref())?,
            },
            PayoutMethodData::Bank(payouts::Bank::Bacs(bank)) => PaymentTemplate {
                rows: get_template_for_bacs(bank, customer_details.as_ref())?,
            },
            _ => Err(errors::ConnectorError::NotSupported {
                message: "payout creation is not supported".to_string(),
                connector: "Envoy",
            })?,
        };

        // Create payment template with bank fields
        let country_code = item.router_data.get_billing_country()?;

        Ok(Self {
            xmlns: "http://merchantapi.envoyservices.com".to_string(),
            auth: EnvoyAuthType::try_from(&item.router_data.connector_auth_type)?,
            request_reference: payout_data.payout_id.clone(),
            payment_instructions: PaymentInstructions {
                instructions: vec![PaymentInstructionV3 {
                    payment_details: PaymentDetails {
                        country_code,
                        source_currency: payout_data.source_currency,
                        source_amount: None,
                        target_currency: payout_data.destination_currency,
                        target_amount: item.amount.to_owned(),
                        source_or_target: SourceOrTarget::Target,
                        merchant_reference: None,
                        payment_reference: payout_data.payout_id.clone(),
                        email: customer_details.as_ref().and_then(|c| c.email.clone()),
                        mobile: customer_details.as_ref().and_then(|c| c.phone.clone()),
                        fast_payment: YesNo::No,
                        payment_description: item.router_data.description.clone(),
                    },
                    payment_template,
                }],
            },
        })
    }
}

// --- Payout Response ---
// Ref: https://docs.worldpay.com/apis/pushtoaccountglobal/reference/paytobankaccountv3#response-schema

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Envelope")]
pub struct EnvoyPayoutSoapResponse {
    #[serde(rename = "Body")]
    pub body: PayoutSoapBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutSoapBody {
    #[serde(rename = "payToBankAccountV3Response")]
    pub response: PayToBankAccountV3Response,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayToBankAccountV3Response {
    #[serde(rename = "payToBankAccountV3Result")]
    pub result: PayToBankAccountV3Result,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayToBankAccountV3Result {
    pub request_reference: String,
    pub received_date: Option<String>,
    pub status_code: i32,
    pub status_message: Option<String>,

    #[serde(rename = "paymentInstructions")]
    pub payment_instructions_results: Option<PaymentInstructionsResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInstructionsResults {
    #[serde(rename = "paymentResultV3")]
    pub payment_result: PaymentInstructionResponseV3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInstructionResponseV3 {
    pub epacs_reference: Option<String>,
    pub payment_reference: Option<String>,
    pub payment_item_id: Option<Secret<String>>,
    pub payment_status_code: i32,
    pub payment_status_message: String,

    pub bank_details: Option<BankDetails>,
    pub payment_template: Option<PaymentTemplate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BankDetails {
    pub payee: Option<Secret<String>>,
    pub account_number: Option<Secret<String>>,
    pub bank_name: Option<Secret<String>>,
    pub iban: Option<Secret<String>>,
    pub swift: Option<Secret<String>>,
    pub bank_account_currency: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseStatus {
    Success,
    SuccessNoResults,
    Processing,
    Failed(i32),
}

impl From<i32> for ResponseStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Success,
            1 => Self::SuccessNoResults,
            2 => Self::Processing,
            other => Self::Failed(other),
        }
    }
}

impl From<ResponseStatus> for enums::PayoutStatus {
    fn from(status: ResponseStatus) -> Self {
        match status {
            ResponseStatus::Success => Self::Success,
            ResponseStatus::SuccessNoResults => Self::Pending,
            ResponseStatus::Processing => Self::Pending,
            ResponseStatus::Failed(_) => Self::Failed,
        }
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<PayoutsResponseRouterData<PoFulfill, EnvoyPayoutSoapResponse>>
    for PayoutsRouterData<PoFulfill>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<PoFulfill, EnvoyPayoutSoapResponse>,
    ) -> Result<Self, Self::Error> {
        let payment_account_v3_result = &item.response.body.response.result;
        let response = &payment_account_v3_result
            .payment_instructions_results
            .as_ref();
        if let Some(instructions_results) = response {
            let payout_status_result =
                ResponseStatus::from(instructions_results.payment_result.payment_status_code);

            let (error_code, error_message) =
                if let ResponseStatus::Failed(_) = payout_status_result {
                    (
                        Some(
                            instructions_results
                                .payment_result
                                .payment_status_code
                                .to_string(),
                        ),
                        Some(
                            instructions_results
                                .payment_result
                                .payment_status_message
                                .clone(),
                        ),
                    )
                } else {
                    (None, None)
                };
            Ok(Self {
                response: Ok(PayoutsResponseData {
                    status: Some(payout_status_result.into()),
                    connector_payout_id: Some(payment_account_v3_result.request_reference.clone()),
                    payout_eligible: None,
                    should_add_next_step_to_process_tracker: false,
                    error_code,
                    error_message,
                    payout_connector_metadata: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                response: Ok(PayoutsResponseData {
                    status: Some(enums::PayoutStatus::Failed),
                    connector_payout_id: Some(payment_account_v3_result.request_reference.clone()),
                    payout_eligible: None,
                    should_add_next_step_to_process_tracker: false,
                    error_code: Some(payment_account_v3_result.status_code.clone().to_string()),
                    error_message: payment_account_v3_result.status_message.clone(),
                    payout_connector_metadata: None,
                }),
                ..item.data
            })
        }
    }
}
