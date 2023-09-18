use api_models::{
    enums::{BankType, CountryAlpha2},
    payments::BankDebitData,
};
use common_utils::pii;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressDetailsData, BankDirectDebitBillingData, ConnectorCustomerData,
        PaymentsAuthorizeRequestData, PaymentsPreProcessingData, RouterData,
    },
    core::errors,
    types::{self, api, storage::enums, MandateReference},
};

pub struct GocardlessRouterData<T> {
    pub amount: i64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for GocardlessRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct GocardlessCustomerRequest {
    address_line1: Option<Secret<String>>,
    address_line2: Option<Secret<String>>,
    address_line3: Option<Secret<String>>,
    city: Option<Secret<String>>,
    country_code: Option<CountryAlpha2>,
    email: pii::Email,
    given_name: Secret<String>,
    family_name: Secret<String>,
    meta_data: CustomerMetaData,
    danish_identity_number: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    swedish_identity_number: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize)]
pub struct CustomerMetaData {
    crm_id: Option<Secret<String>>,
}

impl TryFrom<&types::ConnectorCustomerRouterData> for GocardlessCustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        let email = item.request.get_email()?;
        let billing_address = item.get_billing_address()?;
        let given_name = billing_address.get_first_name()?.to_owned();
        let family_name = billing_address.get_last_name()?.to_owned();
        let meta_data = CustomerMetaData {
            crm_id: item
                .customer_id
                .clone()
                .map(|customer_id| Secret::new(customer_id)),
        };
        Ok(Self {
            email,
            given_name,
            family_name,
            meta_data,
            address_line1: billing_address.line1.to_owned(),
            address_line2: billing_address.line2.to_owned(),
            address_line3: billing_address.line3.to_owned(),
            country_code: billing_address.country,
            // Should be populated based on the billing country
            danish_identity_number: None,
            postal_code: billing_address.zip.to_owned(),
            // Should be populated based on the billing country
            swedish_identity_number: None,
            city: billing_address.city.clone().map(|city| Secret::new(city)),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GocardlessCustomerResponse {
    id: Secret<String>,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            GocardlessCustomerResponse,
            types::ConnectorCustomerData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::ConnectorCustomerData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            GocardlessCustomerResponse,
            types::ConnectorCustomerData,
            types::PaymentsResponseData,
        >,
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
pub struct GocardlessBankAccountRequest {
    customer_bank_accounts: CustomerBankAccount,
    links: CustomerAccountLink,
}

#[derive(Debug, Serialize)]
pub struct CustomerAccountLink {
    customer: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CustomerBankAccount {
    InternationBankAccount(InternationalBankAccount),
    AUBankAccount(AUBankAccount),
    USBankAccount(USBankAccount),
}

#[derive(Debug, Serialize)]
pub struct InternationalBankAccount {
    iban: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct AUBankAccount {
    country_code: CountryAlpha2,
    account_number: Secret<String>,
    branch_code: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub struct USBankAccount {
    country_code: CountryAlpha2,
    account_number: Secret<String>,
    bank_code: Secret<String>,
    account_type: AccountType,
}

#[derive(Debug, Serialize)]
pub enum AccountType {
    Checking,
    Savings,
}

impl TryFrom<&types::TokenizationRouterData> for GocardlessBankAccountRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        let customer = item.get_customer_id()?;
        let customer_bank_accounts = CustomerBankAccount::try_from(item)?;
        Ok(Self {
            customer_bank_accounts,
            links: CustomerAccountLink {
                customer: Secret::new(customer),
            },
        })
    }
}

impl TryFrom<&types::TokenizationRouterData> for CustomerBankAccount {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        match &item.request.payment_method_data {
            api_models::payments::PaymentMethodData::BankDebit(bank_debit_data) => {
                CustomerBankAccount::try_from(bank_debit_data)
            }
            api_models::payments::PaymentMethodData::Card(_)
            | api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::Wallet(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Gocardless"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&BankDebitData> for CustomerBankAccount {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &BankDebitData) -> Result<Self, Self::Error> {
        match item {
            BankDebitData::AchBankDebit {
                billing_details,
                account_number,
                routing_number,
                bank_type,
                ..
            } => {
                let bank_type = bank_type.ok_or_else(utils::missing_field_err("bank_type"))?;
                let country_code = billing_details.get_billing_country()?;
                let us_bank_account = USBankAccount {
                    country_code,
                    account_number: account_number.clone(),
                    bank_code: routing_number.clone(),
                    account_type: AccountType::from(bank_type),
                };
                Ok(Self::USBankAccount(us_bank_account))
            }
            BankDebitData::BecsBankDebit {
                billing_details,
                account_number,
                bsb_number,
            } => {
                let country_code = billing_details.get_billing_country()?;
                let au_bank_account = AUBankAccount {
                    country_code,
                    account_number: account_number.clone(),
                    branch_code: bsb_number.clone(),
                };
                Ok(Self::AUBankAccount(au_bank_account))
            }
            BankDebitData::SepaBankDebit { iban, .. } => {
                let international_bank_account = InternationalBankAccount { iban: iban.clone() };
                Ok(Self::InternationBankAccount(international_bank_account))
            }
            BankDebitData::BacsBankDebit { .. } => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Gocardless"),
            )
            .into()),
        }
    }
}

impl From<BankType> for AccountType {
    fn from(item: BankType) -> Self {
        match item {
            BankType::Checking => AccountType::Checking,
            BankType::Savings => AccountType::Savings,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GocardlessBankAccountResponse {
    pub id: Secret<String>,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            GocardlessBankAccountResponse,
            types::PaymentMethodTokenizationData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentMethodTokenizationData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            GocardlessBankAccountResponse,
            types::PaymentMethodTokenizationData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TokenizationResponse {
                token: item.response.id.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct GocardlessMandateRequest {
    mandates: Mandate,
}

#[derive(Debug, Serialize)]
pub struct Mandate {
    scheme: GocardlessScheme,
    metadata: MandateMetaData,
    links: MandateLink,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GocardlessScheme {
    Becs,
    SepaCore,
    Ach,
    BecsNz,
}

#[derive(Debug, Serialize)]
pub struct MandateMetaData {
    payment_reference: String,
}

#[derive(Debug, Serialize)]
pub struct MandateLink {
    customer_bank_account: Secret<String>,
}

impl TryFrom<&types::PaymentsPreProcessingRouterData> for GocardlessMandateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsPreProcessingRouterData) -> Result<Self, Self::Error> {
        let scheme = match &item.request.payment_method_data {
            Some(payment_method_data) => match payment_method_data {
                api_models::payments::PaymentMethodData::BankDebit(bank_debit_data) => {
                    GocardlessScheme::try_from(bank_debit_data)
                }
                api_models::payments::PaymentMethodData::Card(_)
                | api_models::payments::PaymentMethodData::CardRedirect(_)
                | api_models::payments::PaymentMethodData::Wallet(_)
                | api_models::payments::PaymentMethodData::PayLater(_)
                | api_models::payments::PaymentMethodData::BankRedirect(_)
                | api_models::payments::PaymentMethodData::BankTransfer(_)
                | api_models::payments::PaymentMethodData::Crypto(_)
                | api_models::payments::PaymentMethodData::MandatePayment
                | api_models::payments::PaymentMethodData::Reward
                | api_models::payments::PaymentMethodData::Upi(_)
                | api_models::payments::PaymentMethodData::Voucher(_)
                | api_models::payments::PaymentMethodData::GiftCard(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        "Preprocessing flow for selected payment method through Gocardless"
                            .to_string(),
                    )
                    .into())
                }
            },
            None => todo!(),
        }?;
        let payment_method_token = item.get_payment_method_token()?;
        let customer_bank_account = match payment_method_token {
            types::PaymentMethodToken::Token(token) => Ok(token),
            types::PaymentMethodToken::ApplePayDecrypt(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    "Preprocessing flow for selected payment method through Gocardless".to_string(),
                ))
            }
        }?;
        Ok(Self {
            mandates: Mandate {
                scheme,
                metadata: MandateMetaData {
                    payment_reference: item.connector_request_reference_id.clone(),
                },
                links: MandateLink {
                    customer_bank_account: Secret::new(customer_bank_account),
                },
            },
        })
    }
}

impl TryFrom<&BankDebitData> for GocardlessScheme {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &BankDebitData) -> Result<Self, Self::Error> {
        match item {
            BankDebitData::AchBankDebit { .. } => Ok(Self::Ach),
            BankDebitData::SepaBankDebit { .. } => Ok(Self::SepaCore),
            BankDebitData::BecsBankDebit { .. } => Ok(Self::Becs),
            BankDebitData::BacsBankDebit { .. } => Err(errors::ConnectorError::NotImplemented(
                "Preprocessing flow for selected payment method through Gocardless".to_string(),
            )
            .into()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GocardlessMandateResponse {
    mandates: MandateResponse,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MandateResponse {
    id: String,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            GocardlessMandateResponse,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsPreProcessingData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            GocardlessMandateResponse,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response = if item.data.request.setup_mandate_details.is_some() {
            types::PaymentsResponseData::PreProcessingResponse {
                pre_processing_id: types::PreprocessingResponseId::PreProcessingId(
                    item.response.mandates.id,
                ),
                connector_metadata: None,
                session_token: None,
                connector_response_reference_id: None,
            }
        } else {
            let connector_mandate_id = item.data.request.get_connector_mandate_id()?;
            let mandate_reference = MandateReference {
                connector_mandate_id: Some(connector_mandate_id),
                payment_method_id: None,
            };
            types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::NoResponseId,
                redirection_data: None,
                mandate_reference: Some(mandate_reference),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }
        };
        Ok(Self {
            response: Ok(response),
            status: enums::AttemptStatus::Pending,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct GocardlessPaymentsRequest {
    amount: i64,
    currency: enums::Currency,
    description: Option<String>,
    metadata: PaymentMetaData,
    links: PaymentLink,
}

#[derive(Debug, Serialize)]
pub struct PaymentMetaData {
    payment_reference: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentLink {
    mandate: Secret<String>,
}

impl TryFrom<&GocardlessRouterData<&types::PaymentsAuthorizeRouterData>>
    for GocardlessPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &GocardlessRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let mandate_id = if item.router_data.request.is_mandate_payment() {
            if item.router_data.request.setup_future_usage.is_some() {
                item.router_data.get_preprocessing_id()
            } else {
                item.router_data
                    .request
                    .connector_mandate_id()
                    .ok_or_else(utils::missing_field_err("preprocessing_id"))
            }
        } else {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("gocardless"),
            )
            .into())
        }?;
        Ok(GocardlessPaymentsRequest {
            amount: item.router_data.request.amount,
            currency: item.router_data.request.currency,
            description: item.router_data.description.clone(),
            metadata: PaymentMetaData {
                payment_reference: item.router_data.connector_request_reference_id.clone(),
            },
            links: PaymentLink {
                mandate: Secret::new(mandate_id),
            },
        })
    }
}

// Auth Struct
pub struct GocardlessAuthType {
    pub(super) access_token: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for GocardlessAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                access_token: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GocardlessPaymentStatus {
    PendingCustomerApproval,
    PendingSubmission,
    Submitted,
    Confirmed,
    PaidOut,
    Cancelled,
    CustomerApprovalDenied,
    Failed,
}

impl From<GocardlessPaymentStatus> for enums::AttemptStatus {
    fn from(item: GocardlessPaymentStatus) -> Self {
        match item {
            GocardlessPaymentStatus::PendingCustomerApproval
            | GocardlessPaymentStatus::PendingSubmission
            | GocardlessPaymentStatus::Submitted => Self::Pending,
            GocardlessPaymentStatus::Confirmed | GocardlessPaymentStatus::PaidOut => Self::Charged,
            GocardlessPaymentStatus::Cancelled => Self::Voided,
            GocardlessPaymentStatus::CustomerApprovalDenied => Self::AuthenticationFailed,
            GocardlessPaymentStatus::Failed => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GocardlessPaymentsResponse {
    status: GocardlessPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, GocardlessPaymentsResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            GocardlessPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
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

// REFUND :
#[derive(Default, Debug, Serialize)]
pub struct GocardlessRefundRequest {
    refunds: GocardlessRefund,
}

#[derive(Default, Debug, Serialize)]
pub struct GocardlessRefund {
    amount: i64,
    metadata: RefundMetaData,
    links: RefundLink,
}

#[derive(Default, Debug, Serialize)]
pub struct RefundMetaData {
    refund_reference: String,
}

#[derive(Default, Debug, Serialize)]
pub struct RefundLink {
    payment: String,
}

impl<F> TryFrom<&GocardlessRouterData<&types::RefundsRouterData<F>>> for GocardlessRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &GocardlessRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            refunds: GocardlessRefund {
                amount: item.amount.to_owned(),
                metadata: RefundMetaData {
                    refund_reference: item.router_data.connector_request_reference_id.clone(),
                },
                links: RefundLink {
                    payment: item.router_data.request.connector_transaction_id.clone(),
                },
            },
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct RefundResponse {
    id: String,
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
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::Pending,
            }),
            ..item.data
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct GocardlessErrorResponse {
    pub error: GocardlessError,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GocardlessError {
    pub message: String,
    pub code: String,
    pub errors: Vec<Error>,
    #[serde(rename = "type")]
    pub error_type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Error {
    pub field: Option<String>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GocardlessWebhookEvent {
    pub events: Vec<WebhookEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub resource_type: WebhookResourceType,
    pub action: WebhookAction,
    pub links: WebhooksLink,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookResourceType {
    Payments,
    Refunds,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebhookAction {
    PaymentsAction(PaymentsAction),
    RefundsAction(RefundsAction),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentsAction {
    Created,
    CustomerApprovalGranted,
    CustomerApprovalDenied,
    Submitted,
    Confirmed,
    PaidOut,
    LateFailureSettled,
    SurchargeFeeDebited,
    Failed,
    Cancelled,
    ResubmissionRequired,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefundsAction {
    Created,
    Failed,
    Paid,
    // Payout statuses
    RefundSettled,
    FundsReturned,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebhooksLink {
    PaymentWebhooksLink(PaymentWebhooksLink),
    RefundWebhookLink(RefundWebhookLink),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefundWebhookLink {
    pub refund: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentWebhooksLink {
    pub payment: String,
}

impl TryFrom<&WebhookEvent> for GocardlessPaymentsResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &WebhookEvent) -> Result<Self, Self::Error> {
        let id = match &item.links {
            WebhooksLink::PaymentWebhooksLink(link) => link.payment.to_owned(),
            WebhooksLink::RefundWebhookLink(_) => {
                Err(errors::ConnectorError::WebhookEventTypeNotFound)?
            }
        };
        Ok(Self {
            status: GocardlessPaymentStatus::try_from(&item.action)?,
            id,
        })
    }
}

impl TryFrom<&WebhookAction> for GocardlessPaymentStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &WebhookAction) -> Result<Self, Self::Error> {
        match item {
            WebhookAction::PaymentsAction(action) => match action {
                PaymentsAction::CustomerApprovalGranted | PaymentsAction::Submitted => {
                    Ok(Self::Submitted)
                }
                PaymentsAction::CustomerApprovalDenied => Ok(Self::CustomerApprovalDenied),
                PaymentsAction::LateFailureSettled => Ok(Self::Failed),
                PaymentsAction::Failed => Ok(Self::Failed),
                PaymentsAction::Cancelled => Ok(Self::Cancelled),
                PaymentsAction::Confirmed => Ok(Self::Confirmed),
                PaymentsAction::PaidOut => Ok(Self::PaidOut),
                PaymentsAction::SurchargeFeeDebited
                | PaymentsAction::ResubmissionRequired
                | PaymentsAction::Created => Err(errors::ConnectorError::WebhookEventTypeNotFound)?,
            },
            WebhookAction::RefundsAction(_) => {
                Err(errors::ConnectorError::WebhookEventTypeNotFound)?
            }
        }
    }
}
