use std::collections::BTreeMap;

#[cfg(feature = "payouts")]
use api_models::payouts::{Bank, PayoutMethodData};
use base64::{engine::general_purpose, Engine as _};
use common_enums::enums;
#[cfg(feature = "payouts")]
use common_enums::{CountryAlpha2, PayoutStatus};
use common_utils::{id_type::CustomerId, pii, types::StringMajorUnit};
use error_stack::ResultExt;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::types::{PayoutsResponseData, PayoutsRouterData};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors::ConnectorError;
use hyperswitch_masking::{ExposeInterface, Secret};
use openssl::{hash::MessageDigest, pkey::PKey, rsa::Rsa, sign::Signer};
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};
#[cfg(feature = "payouts")]
use crate::{
    types::PayoutsResponseRouterData,
    utils::{
        self, get_unimplemented_payment_method_error_message, AddressData,
        PayoutFulfillRequestData, PayoutsData, RouterData as _,
    },
};

//TODO: Fill the struct with respective fields
pub struct TrustlyRouterData<T> {
    pub amount: StringMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for TrustlyRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct TrustlyPaymentsRequest {
    amount: StringMajorUnit,
    card: TrustlyCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct TrustlyCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&TrustlyRouterData<&PaymentsAuthorizeRouterData>> for TrustlyPaymentsRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &TrustlyRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(_) => Err(ConnectorError::NotImplemented(
                "Card payment method not implemented".to_string(),
            )
            .into()),
            _ => Err(ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustlyAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for TrustlyAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                username: api_key.to_owned(),
                password: key1.to_owned(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustlyMetadata {
    private_key: Secret<String>,
}

#[cfg(feature = "payouts")]
impl TryFrom<&Option<pii::SecretSerdeValue>> for TrustlyMetadata {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(ConnectorError::InvalidConnectorConfig { config: "metadata" })?;
        Ok(metadata)
    }
}

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrustlyPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<TrustlyPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: TrustlyPaymentStatus) -> Self {
        match item {
            TrustlyPaymentStatus::Succeeded => Self::Charged,
            TrustlyPaymentStatus::Failed => Self::Failure,
            TrustlyPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustlyPaymentsResponse {
    status: TrustlyPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, TrustlyPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, TrustlyPaymentsResponse, T, PaymentsResponseData>,
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
pub struct TrustlyRefundRequest {
    pub amount: StringMajorUnit,
}

impl<F> TryFrom<&TrustlyRouterData<&RefundsRouterData<F>>> for TrustlyRefundRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &TrustlyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
    type Error = error_stack::Report<ConnectorError>;
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
    type Error = error_stack::Report<ConnectorError>;
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustlyErrorResponse {
    pub version: String,
    pub error: TrustlyErrorResponseError,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustlyErrorResponseError {
    pub name: String,
    pub code: i64,
    pub message: String,
    pub error: TrustlyErrorResponseErrorDetails,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustlyErrorResponseErrorDetails {
    pub uuid: String,
}

fn process_error_response(error_response: TrustlyErrorResponse, http_code: u16) -> ErrorResponse {
    ErrorResponse {
        code: error_response.error.code.to_string(),
        message: error_response.error.message.clone(),
        reason: Some(error_response.error.message.clone()),
        status_code: http_code,
        attempt_status: None,
        connector_transaction_id: None,
        connector_response_reference_id: Some(error_response.error.error.uuid),
        network_advice_code: None,
        network_decline_code: None,
        network_error_message: None,
        connector_metadata: None,
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum TrustlyMethod {
    RegisterAccount,
    AccountPayout,
    GetWithdrawals,
}

impl TrustlyMethod {
    fn as_str(&self) -> &'static str {
        match self {
            Self::RegisterAccount => "RegisterAccount",
            Self::AccountPayout => "AccountPayout",
            Self::GetWithdrawals => "GetWithdrawals",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RegisterAccountRequest {
    method: TrustlyMethod,
    params: RegisterAccountParams,
    version: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RegisterAccountParams {
    data: RegisterAccountData,
    signature: Secret<String>,
    #[serde(rename = "UUID")]
    uuid: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "PascalCase")]
pub struct RegisterAccountData {
    account_number: Secret<String>,
    bank_number: Secret<String>,
    clearing_house: String,
    end_user_i_d: CustomerId,
    firstname: Secret<String>,
    lastname: Secret<String>,
    username: Secret<String>,
    password: Secret<String>,
    attributes: Option<RegisterAccountAttributes>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
#[serde_with::skip_serializing_none]
pub struct RegisterAccountAttributes {
    address_country: Option<CountryAlpha2>,
    address_line1: Option<Secret<String>>,
    address_line2: Option<Secret<String>>,
    address_city: Option<String>,
    address_postal_code: Option<Secret<String>>,
    mobile_phone: Option<Secret<String>>,
    email: Option<pii::Email>,
}

#[cfg(feature = "payouts")]
fn trustly_serialize<T: Serialize>(data: &T) -> String {
    let value = serde_json::to_value(data).unwrap_or_default();
    serialize_value(&value)
}

enum Algorithm {
    SHA256,
}

impl Algorithm {
    fn message_digest(&self) -> MessageDigest {
        match self {
            Self::SHA256 => MessageDigest::sha256(),
        }
    }

    fn prefix(&self) -> &'static str {
        // Trustly expects the signature header like "alg=RS256;"
        "alg=RS256;"
    }
}

fn serialize_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<_, _> = map.iter().collect();
            sorted.iter().filter(|(_, v)| !v.is_null()).fold(
                String::new(),
                |mut output, (key, value)| {
                    output.push_str(key);
                    output.push_str(&serialize_data(value));
                    output
                },
            )
        }
        serde_json::Value::Array(arr) => arr.iter().map(serialize_data).collect(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
    }
}

fn generate_trustly_signature<T: Serialize>(
    method: &TrustlyMethod,
    uuid: &str,
    data: &T,
    private_key: &str,
) -> Result<String, ConnectorError> {
    let algorithm = Algorithm::SHA256;
    let pem = utils::base64_decode(private_key.to_string())
        .map_err(|_| ConnectorError::RequestEncodingFailed)?;
    let rsa = Rsa::private_key_from_pem(&pem).map_err(|_| ConnectorError::RequestEncodingFailed)?;
    let private_key = PKey::from_rsa(rsa).map_err(|_| ConnectorError::RequestEncodingFailed)?;

    let plaintext = format!("{}{}{}", method.as_str(), uuid, trustly_serialize(data));

    let mut signer = Signer::new(algorithm.message_digest(), &private_key)
        .map_err(|_| ConnectorError::RequestEncodingFailed)?;
    signer
        .update(plaintext.as_bytes())
        .map_err(|_| ConnectorError::RequestEncodingFailed)?;
    let signature = signer
        .sign_to_vec()
        .map_err(|_| ConnectorError::RequestEncodingFailed)?;

    Ok(format!(
        "{}{}",
        algorithm.prefix(),
        general_purpose::STANDARD.encode(&signature)
    ))
}

fn serialize_data(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            // BTreeMap keeps keys sorted (matches PHP's ksort)
            let sorted: BTreeMap<_, _> = map.iter().collect();
            sorted
                .iter()
                .fold(String::new(), |mut output, (key, value)| {
                    output.push_str(key);
                    output.push_str(&serialize_data(value));
                    output
                })
        }
        serde_json::Value::Array(arr) => arr.iter().map(serialize_data).collect(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
    }
}

#[cfg(feature = "payouts")]
fn get_customer_details(
    customer_details: Option<&hyperswitch_domain_models::router_request_types::CustomerDetails>,
    billing_details: Option<&hyperswitch_domain_models::address::Address>,
) -> Result<(String, String), ConnectorError> {
    if let Some(customer) = customer_details {
        if let Some(name) = &customer.name {
            let n = name.clone().expose();
            let parts: Vec<&str> = n.splitn(2, ' ').collect();
            if let [first, second] = parts.as_slice() {
                return Ok((first.to_string(), second.to_string()));
            }
        }
    }

    if let Some(billing) = billing_details {
        if let Some(address) = &billing.address {
            let first_name = address
                .first_name
                .clone()
                .ok_or(ConnectorError::MissingRequiredField {
                    field_name: "first_name",
                })?
                .expose();
            let last_name = address
                .last_name
                .clone()
                .ok_or(ConnectorError::MissingRequiredField {
                    field_name: "last_name",
                })?
                .expose();

            return Ok((first_name, last_name));
        }
    }

    Err(ConnectorError::MissingRequiredField {
        field_name: "customer's first name / last name",
    })
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<&TrustlyRouterData<&PayoutsRouterData<F>>> for RegisterAccountRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &TrustlyRouterData<&PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let payout_method_data = item.router_data.get_payout_method_data()?;
        match payout_method_data {
            PayoutMethodData::Bank(Bank::Trustly(trustly_data)) => {
                let (account_number, bank_number) = if let Some(iban) = trustly_data.iban {
                    (iban, Secret::new(String::new()))
                } else {
                    (
                        trustly_data.account_number.ok_or(
                            ConnectorError::MissingRequiredField {
                                field_name: "account_number",
                            },
                        )?,
                        trustly_data
                            .bank_number
                            .ok_or(ConnectorError::MissingRequiredField {
                                field_name: "bank_number",
                            })?,
                    )
                };

                let customer_details = item.router_data.request.customer_details.clone();
                let billing_details = item.router_data.get_optional_billing();
                let (first_name, last_name) =
                    get_customer_details(customer_details.as_ref(), billing_details)?;

                let attributes = if billing_details.is_some()
                    || customer_details.and_then(|details| details.email).is_some()
                {
                    Some(RegisterAccountAttributes {
                        address_city: item.router_data.get_optional_billing_city(),
                        address_country: item.router_data.get_optional_billing_country(),
                        address_line1: item.router_data.get_optional_billing_line1(),
                        address_line2: item.router_data.get_optional_billing_line2(),
                        address_postal_code: item.router_data.get_optional_billing_zip(),
                        email: item.router_data.get_optional_billing_email(),
                        mobile_phone: billing_details.and_then(|details| hyperswitch_domain_models::address::Address::get_phone_with_country_code(details).ok()),
                    })
                } else {
                    None
                };

                let uuid = uuid::Uuid::new_v4().to_string();
                let private_key =
                    TrustlyMetadata::try_from(&item.router_data.connector_meta_data)?.private_key;
                let auth_details =
                    TrustlyAuthType::try_from(&item.router_data.connector_auth_type)?;
                let register_account_data = RegisterAccountData {
                    account_number,
                    bank_number,
                    clearing_house: common_enums::Country::from_alpha2(trustly_data.country_code)
                        .to_string()
                        .to_uppercase(),
                    end_user_i_d: item.router_data.get_customer_id()?,
                    firstname: Secret::new(first_name),
                    lastname: Secret::new(last_name),
                    username: auth_details.username,
                    password: auth_details.password,
                    attributes,
                };

                let signature = generate_trustly_signature(
                    &TrustlyMethod::RegisterAccount,
                    uuid.as_str(),
                    &register_account_data,
                    &private_key.expose(),
                )?;

                Ok(Self {
                    method: TrustlyMethod::RegisterAccount,
                    params: RegisterAccountParams {
                        data: register_account_data,
                        signature: Secret::new(signature),
                        uuid,
                    },
                    version: "1.1".to_string(),
                })
            }
            PayoutMethodData::Card(_)
            | PayoutMethodData::Wallet(_)
            | PayoutMethodData::Bank(_)
            | PayoutMethodData::BankRedirect(_)
            | PayoutMethodData::Passthrough(_) => Err(ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Trustly"),
            ))?,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RegisterAccountResponse {
    Success(RegisterAccountResponseSuccess),
    Error(TrustlyErrorResponse),
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegisterAccountResponseSuccess {
    pub result: RegisterAccountResponseResult,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegisterAccountResponseResult {
    data: RegisterAccountResponseResultData,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegisterAccountResponseResultData {
    accountid: Secret<String>,
    clearinghouse: String,
    bank: String,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, RegisterAccountResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, RegisterAccountResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            RegisterAccountResponse::Success(response) => {
                let account_id = response.result.data.accountid;
                let payout_connector_metadata = Some(Secret::new(serde_json::json!({
                    "account_id": account_id,
                })));

                Ok(Self {
                    response: Ok(PayoutsResponseData {
                        status: Some(PayoutStatus::RequiresCreation),
                        connector_payout_id: None,
                        payout_eligible: None,
                        should_add_next_step_to_process_tracker: false,
                        error_code: None,
                        error_message: None,
                        payout_connector_metadata,
                    }),
                    ..item.data
                })
            }
            RegisterAccountResponse::Error(error_response) => {
                let response = Err(process_error_response(error_response, item.http_code));
                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountPayoutRequest {
    method: TrustlyMethod,
    params: AccountPayoutParams,
    version: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct AccountPayoutParams {
    signature: Secret<String>,
    #[serde(rename = "UUID")]
    uuid: String,
    data: AccountPayoutData,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct AccountPayoutData {
    account_i_d: Secret<String>,
    amount: StringMajorUnit,
    attributes: Option<AccountPayoutAttributes>,
    currency: common_enums::Currency,
    end_user_i_d: CustomerId,
    message_i_d: String,
    notification_u_r_l: String,
    password: Secret<String>,
    username: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct AccountPayoutAttributes {
    shopper_statement: String,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustlyAccountId {
    account_id: Secret<String>,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<&TrustlyRouterData<&PayoutsRouterData<F>>> for AccountPayoutRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &TrustlyRouterData<&PayoutsRouterData<F>>) -> Result<Self, Self::Error> {
        let payout_method_data = item.router_data.get_payout_method_data()?;
        match payout_method_data {
            PayoutMethodData::Bank(Bank::Trustly(_trustly_data)) => {
                let notification_url = item.router_data.request.get_webhook_url()?;

                let metadata = item
                    .router_data
                    .request
                    .payout_connector_metadata
                    .clone()
                    .map(|secret| secret.expose().clone());
                let account_id: TrustlyAccountId =
                    utils::to_payout_connector_meta(metadata.clone())?;

                let auth_details =
                    TrustlyAuthType::try_from(&item.router_data.connector_auth_type)?;

                let private_key =
                    TrustlyMetadata::try_from(&item.router_data.connector_meta_data)?.private_key;
                let uuid = uuid::Uuid::new_v4().to_string();
                let account_payout_data = AccountPayoutData {
                    account_i_d: account_id.account_id,
                    amount: item.amount.clone(),
                    attributes: Some(AccountPayoutAttributes {
                        shopper_statement: item.router_data.description.clone().ok_or(
                            ConnectorError::MissingRequiredField {
                                field_name: "description",
                            },
                        )?,
                    }),
                    currency: item.router_data.request.destination_currency,
                    end_user_i_d: item.router_data.get_customer_id()?,
                    message_i_d: item
                        .router_data
                        .request
                        .payout_id
                        .get_string_repr()
                        .to_string(),
                    notification_u_r_l: notification_url,
                    password: auth_details.password.clone(),
                    username: auth_details.username.clone(),
                };

                let signature = generate_trustly_signature(
                    &TrustlyMethod::AccountPayout,
                    uuid.as_str(),
                    &account_payout_data,
                    &private_key.expose(),
                )?;

                Ok(Self {
                    method: TrustlyMethod::AccountPayout,
                    params: AccountPayoutParams {
                        data: account_payout_data,
                        signature: Secret::new(signature),
                        uuid,
                    },
                    version: "1.1".to_string(),
                })
            }
            PayoutMethodData::Card(_)
            | PayoutMethodData::Wallet(_)
            | PayoutMethodData::Bank(_)
            | PayoutMethodData::BankRedirect(_)
            | PayoutMethodData::Passthrough(_) => Err(ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Trustly"),
            ))?,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum PayoutResult {
    #[serde(rename = "0")]
    Failed,
    #[serde(rename = "1")]
    Pending,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AccountPayoutResponse {
    Success(AccountPayoutResponseSuccess),
    Error(TrustlyErrorResponse),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountPayoutResponseSuccess {
    version: String,
    result: AccountPayoutResponseResult,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountPayoutResponseResult {
    data: AccountPayoutResponseData,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountPayoutResponseData {
    orderid: String,
    result: PayoutResult,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, AccountPayoutResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, AccountPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            AccountPayoutResponse::Success(success_response) => {
                let response = success_response.result.data;
                let payout_status = match response.result {
                    PayoutResult::Failed => PayoutStatus::Failed,
                    PayoutResult::Pending => PayoutStatus::Initiated,
                };

                Ok(Self {
                    response: Ok(PayoutsResponseData {
                        status: Some(payout_status),
                        connector_payout_id: Some(response.orderid),
                        payout_eligible: None,
                        should_add_next_step_to_process_tracker: false,
                        error_code: None,
                        error_message: None,
                        payout_connector_metadata: None,
                    }),
                    ..item.data
                })
            }
            AccountPayoutResponse::Error(error_response) => {
                let response = Err(process_error_response(error_response, item.http_code));
                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustlyPayoutSyncRequest {
    method: TrustlyMethod,
    params: PayoutSyncRequestParams,
    version: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PayoutSyncRequestParams {
    #[serde(rename = "UUID")]
    uuid: String,
    data: PayoutSyncRequestData,
    signature: Secret<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PayoutSyncRequestData {
    order_id: Secret<String>,
    password: Secret<String>,
    username: Secret<String>,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<&PayoutsRouterData<F>> for TrustlyPayoutSyncRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_details = TrustlyAuthType::try_from(&item.connector_auth_type)?;
        let data = PayoutSyncRequestData {
            order_id: Secret::new(item.request.get_connector_payout_id()?),
            password: auth_details.password.clone(),
            username: auth_details.username.clone(),
        };
        let private_key = TrustlyMetadata::try_from(&item.connector_meta_data)?.private_key;

        let uuid = uuid::Uuid::new_v4().to_string();
        let signature = generate_trustly_signature(
            &TrustlyMethod::GetWithdrawals,
            uuid.as_str(),
            &data,
            &private_key.expose(),
        )?;

        Ok(Self {
            method: TrustlyMethod::GetWithdrawals,
            params: PayoutSyncRequestParams {
                uuid,
                data,
                signature: Secret::new(signature),
            },
            version: "1.1".to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TrustlyPayoutSyncResponse {
    Success(TrustlyPayoutSyncResponseSuccess),
    Error(TrustlyErrorResponse),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustlyPayoutSyncResponseSuccess {
    result: TrustlyPayoutSyncResponseResult,
    version: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustlyPayoutSyncResponseResult {
    uuid: String,
    method: String,
    data: Vec<TrustlyPayoutSyncResponseData>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustlyPayoutSyncResponseData {
    reference: String,
    orderid: String,
    transferstate: TrustlyPayoutStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum TrustlyPayoutStatus {
    Confirmed,
    Executing,
    Executed,
    Pending,
    Queued,
    Preparing,
    Prepared,
    Bounced,
    Error,
    Failed,
    Returned,
}

impl From<TrustlyPayoutStatus> for PayoutStatus {
    fn from(item: TrustlyPayoutStatus) -> Self {
        match item {
            TrustlyPayoutStatus::Confirmed => Self::Success,
            TrustlyPayoutStatus::Failed
            | TrustlyPayoutStatus::Error
            | TrustlyPayoutStatus::Bounced
            | TrustlyPayoutStatus::Returned => Self::Failed,
            TrustlyPayoutStatus::Executing | TrustlyPayoutStatus::Executed => Self::Pending,
            TrustlyPayoutStatus::Pending
            | TrustlyPayoutStatus::Queued
            | TrustlyPayoutStatus::Preparing
            | TrustlyPayoutStatus::Prepared => Self::Initiated,
        }
    }
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, TrustlyPayoutSyncResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, TrustlyPayoutSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            TrustlyPayoutSyncResponse::Success(response) => {
                if let Some(first) = response.result.data.first() {
                    Ok(Self {
                        response: Ok(PayoutsResponseData {
                            status: Some(PayoutStatus::from(first.transferstate.clone())),
                            connector_payout_id: Some(first.orderid.clone()),
                            payout_eligible: None,
                            should_add_next_step_to_process_tracker: false,
                            error_code: None,
                            error_message: None,
                            payout_connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        response: Ok(PayoutsResponseData {
                            status: None,
                            connector_payout_id: None,
                            payout_eligible: None,
                            should_add_next_step_to_process_tracker: false,
                            error_code: None,
                            error_message: None,
                            payout_connector_metadata: None,
                        }),
                        ..item.data
                    })
                }
            }
            TrustlyPayoutSyncResponse::Error(error_response) => {
                let response = Err(process_error_response(error_response, item.http_code));
                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}
