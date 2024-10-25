use common_enums::{enums, Currency, PayoutStatus};
use common_utils::{
    pii::Email,
    types::{StringMajorUnit, StringMinorUnit},
};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, PayoutsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PayoutsRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{PayoutsResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, RouterData as UtilsRouterData},
};

//TODO: Fill the struct with respective fields
pub struct NomupayRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for NomupayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct NomupayPaymentsRequest {
    amount: StringMinorUnit,
    card: NomupayCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]

pub struct NomupayCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub country: enums::CountryAlpha2,
    pub state_province: Secret<String>,
    pub street: Secret<String>,
    pub city: String,
    pub postal_code: Secret<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub profile_type: String,
    pub first_name: Secret<String>,
    pub last_name: Secret<String>,
    pub date_of_birth: String,
    pub gender: String,
    pub email_address: Email,
    pub phone_number_country_code: Option<String>,
    pub phone_number: Option<Secret<String>>,
    pub address: Address,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OnboardSubAccountRequest {
    //1
    pub account_id: Secret<String>,
    pub client_sub_account_id: String,
    pub profile: Profile,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BankAccount {
    pub bank_id: Option<Secret<String>>,
    pub account_id: Secret<String>,
    pub account_purpose: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VirtualAccountsType {
    pub country_code: String,
    pub currency_code: String,
    pub bank_id: String,
    pub bank_account_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OnboardTransferMethodRequest {
    //2
    pub country_code: enums::CountryAlpha2,
    pub currency_code: Currency,
    #[serde(rename = "type")]
    pub typee: String, // type giving error
    pub display_name: Secret<String>,
    pub bank_account: BankAccount,
    pub profile: Profile,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequest {
    //3
    pub source_id: Secret<String>,
    pub destination_id: Option<String>,
    pub payment_reference: Option<String>,
    pub amount: i64,
    pub currency_code: Currency,
    pub purpose: String,
    pub description: String,
    pub internal_memo: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    //4
    pub source_id: String,
    pub source_currency_code: Currency,
    pub destination_currency_code: Currency,
    pub amount: String,
    pub include_fee: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitRequest {
    //5
    pub source_id: String,
    pub id: String,
    pub destination_id: String,
    pub payment_reference: String,
    pub amount: String,
    pub currency_code: Currency,
    pub purpose: String,
    pub description: String,
    pub internal_memo: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OnboardSubAccountResponse {
    pub account_id: String,
    pub id: String,
    pub client_sub_account_id: String,
    pub profile: Profile,
    pub virtual_accounts: Vec<VirtualAccountsType>,
    pub status: String,
    pub created_on: String,
    pub last_updated: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OnboardTransferMethodResponse {
    pub parent_id: String,
    pub account_id: String,
    pub sub_account_id: String,
    pub id: String,
    pub status: String,
    pub created_on: String,
    pub last_updated: String,
    pub country_code: String,
    pub currency_code: Currency,
    pub display_name: String,
    #[serde(rename = "type")]
    pub typee: String,
    pub profile: Profile,
    pub bank_account: BankAccount,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResponse {
    pub id: String,
    pub status: NomupayPaymentStatus,
    pub created_on: String,
    pub last_updated: String,
    pub source_id: String,
    pub destination_id: String,
    pub payment_reference: String,
    pub amount: String,
    pub currency_code: String,
    pub purpose: String,
    pub description: String,
    pub internal_memo: String,
    pub release_on: String,
    pub expire_on: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FeesType {
    #[serde(rename = "type")]
    pub typee: String,
    pub fees: StringMajorUnit,
    pub currency_code: Currency,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PayoutQuoteResponse {
    pub source_id: String,
    pub destination_currency_code: Currency,
    pub amount: StringMajorUnit,
    pub source_currency_code: Currency,
    pub include_fee: bool,
    pub fees: Vec<FeesType>,
    pub payment_reference: String,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitResponse {
    pub id: String,
    pub status: String,
    pub created_on: String,
    pub last_updated: String,
    pub source_id: String,
    pub destination_id: String,
    pub payment_reference: String,
    pub amount: String,
    pub currency_code: Currency,
    pub purpose: String,
    pub description: String,
    pub internal_memo: String,
    pub release_on: String,
    pub expire_on: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub field: String,
    pub message: String,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NomupayError {
    pub error_code: String,
    pub error_description: Option<String>,
    pub validation_errors: Option<Vec<Error>>,
}

impl TryFrom<&NomupayRouterData<&PaymentsAuthorizeRouterData>> for NomupayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NomupayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = NomupayCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct NomupayAuthType {
    pub(super) kid: Secret<String>,
    pub(super) eid: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for NomupayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                kid: api_key.to_owned(),
                eid: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NomupayPaymentStatus {
    Pending,
    Processed,
    Failed,
    #[default]
    Processing,
    Scheduled,
    PendingAccountActivation,
    PendingTransferMethodCreation,
    PendingAccountKyc,
}

impl From<NomupayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: NomupayPaymentStatus) -> Self {
        match item {
            NomupayPaymentStatus::Processed => Self::Charged,
            NomupayPaymentStatus::Failed => Self::Failure,
            NomupayPaymentStatus::Processing => Self::Authorizing,
            _ => Self::Pending,
        }
    }
}

impl From<NomupayPaymentStatus> for PayoutStatus {
    fn from(item: NomupayPaymentStatus) -> Self {
        match item {
            NomupayPaymentStatus::Processed => Self::Success,
            NomupayPaymentStatus::Failed => Self::Failed,
            NomupayPaymentStatus::Processing => Self::Pending,
            _ => Self::Pending,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NomupayPaymentsResponse {
    status: NomupayPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, NomupayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, NomupayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct NomupayRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&NomupayRouterData<&RefundsRouterData<F>>> for NomupayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &NomupayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
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
pub struct NomupayErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

// PoRecipient Request
impl<F> TryFrom<&PayoutsRouterData<F>> for OnboardSubAccountRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let request = item.request.to_owned();
        let payout_type = request.payout_type;
        // let external_id = item.connector_request_reference_id.to_owned();

        // let country_iso2_code = item
        //     .get_billing_country()
        //     .unwrap_or(enums::CountryAlpha2::ER);

        // let auth  =  NomupayAuthType::try_from(&item.router_data.connector_auth_type)?;

        let my_address = Address {
            country: item
                .get_billing_country()
                .unwrap_or(enums::CountryAlpha2::ER),
            state_province: item
                .get_billing_state()
                .unwrap_or(Secret::new("default state".to_string())),
            street: item
                .get_billing_line1()
                .unwrap_or(Secret::new("default street".to_string())),
            city: item
                .get_billing_city()
                .unwrap_or("default city".to_string()),
            postal_code: item
                .get_billing_zip()
                .unwrap_or(Secret::new("123456".to_string())),
        };

        let profile = Profile {
            profile_type: "INDIVIDUAL".to_string(),
            first_name: item
                .get_billing_first_name()
                .unwrap_or(Secret::new("first name".to_string())),
            last_name: item
                .get_billing_last_name()
                .unwrap_or(Secret::new("last name".to_string())),
            date_of_birth: "unknown".to_string(),
            gender: "unknown".to_string(),
            email_address: item.get_billing_email().unwrap(),
            phone_number_country_code: item
                .get_billing_phone()
                .map(|phone| phone.country_code.clone())
                .unwrap(),
            phone_number: Some(
                item.get_billing_phone_number()
                    .unwrap_or(Secret::new("phone number".to_string())),
            ),
            address: my_address,
        };

        let source_id = match item.connector_auth_type.to_owned() {
            ConnectorAuthType::BodyKey { api_key: _, key1 } => Ok(key1),
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "source_id for PayoutRecipient creation",
            }),
        }?;

        match payout_type {
            Some(common_enums::PayoutType::Bank) => Ok(OnboardSubAccountRequest {
                account_id: source_id,                              //need help
                client_sub_account_id: "how to get it".to_string(), //need help
                profile,
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "This payment method is not implemented for Thunes".to_string(),
            )
            .into()),
        }
    }
}

// PoRecipient Response
#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, OnboardSubAccountResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, OnboardSubAccountResponse>,
    ) -> Result<Self, Self::Error> {
        let response: OnboardSubAccountResponse = item.response;

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::RequiresVendorAccountCreation),
                connector_payout_id: Some(response.id.to_string()),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: Some(String::from("error from: Payouts quote response transform")),
            }),
            ..item.data
        })
    }
}

// PoRecipientAccount Request
impl<F> TryFrom<&PayoutsRouterData<F>> for OnboardTransferMethodRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        // let country_iso2_code = item
        //     .get_billing_country()
        //     .unwrap_or(enums::CountryAlpha2::ER);
        let payout_method_data = item.get_payout_method_data()?;
        match payout_method_data {
            api_models::payouts::PayoutMethodData::Bank(bank) => match bank {
                api_models::payouts::Bank::Sepa(bank_details) => {
                    let bank_account = BankAccount {
                        bank_id: bank_details.bic,
                        account_id: bank_details.iban,
                        account_purpose: bank_details.bank_name, // savings or somthing else need help
                    };

                    let request = item.request.to_owned();
                    // let payout_type = request.payout_type;
                    // let external_id = item.connector_request_reference_id.to_owned();

                    let country_iso2_code = item
                        .get_billing_country()
                        .unwrap_or(enums::CountryAlpha2::CA);

                    // let auth  =  NomupayAuthType::try_from(&item.router_data.connector_auth_type)?;

                    let my_address = Address {
                        country: item
                            .get_billing_country()
                            .unwrap_or(enums::CountryAlpha2::ER),
                        state_province: item
                            .get_billing_state()
                            .unwrap_or(Secret::new("default state".to_string())),
                        street: item
                            .get_billing_line1()
                            .unwrap_or(Secret::new("default street".to_string())),
                        city: item
                            .get_billing_city()
                            .unwrap_or("default city".to_string()),
                        postal_code: item
                            .get_billing_zip()
                            .unwrap_or(Secret::new("123456".to_string())),
                    };

                    let profile = Profile {
                        profile_type: "INDIVIDUAL".to_string(),
                        first_name: item
                            .get_billing_first_name()
                            .unwrap_or(Secret::new("first name".to_string())),
                        last_name: item
                            .get_billing_last_name()
                            .unwrap_or(Secret::new("last name".to_string())),
                        date_of_birth: "unknown".to_string(),
                        gender: "unknown".to_string(),
                        email_address: item.get_billing_email().unwrap(),
                        phone_number_country_code: item
                            .get_billing_phone()
                            .map(|phone| phone.country_code.clone())
                            .unwrap(),
                        phone_number: Some(
                            item.get_billing_phone_number()
                                .unwrap_or(Secret::new("phone number".to_string())),
                        ),
                        address: my_address,
                    };

                    Ok(OnboardTransferMethodRequest {
                        country_code: country_iso2_code,
                        currency_code: item.request.destination_currency,
                        typee: "BANK_ACCOUNT".to_string(),
                        display_name: item.get_billing_full_name().unwrap(),
                        bank_account,
                        profile,
                    })
                }
                _ => Err(errors::ConnectorError::NotSupported {
                    message: "SEPA payouts are not supported".to_string(),
                    connector: "stripe",
                }
                .into()),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "This payment method is not implemented for Nomupay".to_string(),
            )
            .into()),
        }
    }
}

// PoRecipientAccount response
impl<F> TryFrom<PayoutsResponseRouterData<F, OnboardTransferMethodResponse>>
    for PayoutsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, OnboardTransferMethodResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::RequiresCreation),
                connector_payout_id: item.data.request.connector_payout_id.clone(),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
            }),
            ..item.data
        })
    }
}

// PoFulfill Request
impl<F> TryFrom<&PayoutsRouterData<F>> for PaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let source_id = match item.connector_auth_type.to_owned() {
            ConnectorAuthType::BodyKey { api_key: _, key1 } => Ok(key1),
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "source_id for PayoutRecipient creation",
            }),
        }?;
        Ok(Self {
            source_id,
            destination_id: item
                .response
                .clone()
                .map(|i| i.connector_payout_id)
                .unwrap(),
            payment_reference: item.request.clone().connector_payout_id,
            amount: item.request.amount,
            currency_code: item.request.destination_currency,
            purpose: "OTHER".to_string(),
            description: "This a test payment".to_string(),
            internal_memo: "This is an internal memo".to_string(),
        })
    }
}

// PoFulfill response
impl<F> TryFrom<PayoutsResponseRouterData<F, PaymentResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: PayoutsResponseRouterData<F, PaymentResponse>) -> Result<Self, Self::Error> {
        let response: PaymentResponse = item.response;

        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(PayoutStatus::from(response.status)),
                connector_payout_id: item.data.request.connector_payout_id.clone(),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
            }),
            ..item.data
        })
    }
}
