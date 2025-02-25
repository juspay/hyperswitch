use std::collections::BTreeMap;

use common_utils::{
    errors::CustomResult,
    ext_traits::{Encode, ValueExt},
};
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface, Secret, StrongSecret};
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    connector::utils::{
        self, CardData, PaymentsSyncRequestData, RefundsRequestData, RouterData, WalletData,
    },
    core::errors,
    services,
    types::{
        self,
        api::{self, enums as api_enums},
        domain,
        storage::enums,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils::OptionExt,
};

#[derive(Debug, Serialize)]
pub enum TransactionType {
    #[serde(rename = "authCaptureTransaction")]
    Payment,
    #[serde(rename = "authOnlyTransaction")]
    Authorization,
    #[serde(rename = "priorAuthCaptureTransaction")]
    Capture,
    #[serde(rename = "refundTransaction")]
    Refund,
    #[serde(rename = "voidTransaction")]
    Void,
    #[serde(rename = "authOnlyContinueTransaction")]
    ContinueAuthorization,
    #[serde(rename = "authCaptureContinueTransaction")]
    ContinueCapture,
}

#[derive(Debug, Serialize)]
pub struct AuthorizedotnetRouterData<T> {
    pub amount: f64,
    pub router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for AuthorizedotnetRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (&api::CurrencyUnit, enums::Currency, i64, T),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_f64(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetAuthType {
    name: Secret<String>,
    transaction_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for AuthorizedotnetAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                name: api_key.to_owned(),
                transaction_key: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CreditCardDetails {
    card_number: StrongSecret<String, cards::CardNumberStrategy>,
    expiration_date: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    card_code: Option<Secret<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BankAccountDetails {
    account_number: Secret<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum PaymentDetails {
    CreditCard(CreditCardDetails),
    OpaqueData(WalletDetails),
    PayPal(PayPalDetails),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PayPalDetails {
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletDetails {
    pub data_descriptor: WalletMethod,
    pub data_value: Secret<String>,
}

#[derive(Serialize, Debug, Deserialize)]
pub enum WalletMethod {
    #[serde(rename = "COMMON.GOOGLE.INAPP.PAYMENT")]
    Googlepay,
    #[serde(rename = "COMMON.APPLE.INAPP.PAYMENT")]
    Applepay,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionRequest {
    transaction_type: TransactionType,
    amount: f64,
    currency_code: common_enums::Currency,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment: Option<PaymentDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<ProfileDetails>,
    order: Order,
    #[serde(skip_serializing_if = "Option::is_none")]
    customer: Option<CustomerDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bill_to: Option<BillTo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_fields: Option<UserFields>,
    #[serde(skip_serializing_if = "Option::is_none")]
    processing_options: Option<ProcessingOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subsequent_auth_information: Option<SubsequentAuthInformation>,
    authorization_indicator_type: Option<AuthorizationIndicator>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFields {
    user_field: Vec<UserField>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserField {
    name: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum ProfileDetails {
    CreateProfileDetails(CreateProfileDetails),
    CustomerProfileDetails(CustomerProfileDetails),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateProfileDetails {
    create_profile: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CustomerProfileDetails {
    customer_profile_id: Secret<String>,
    payment_profile: PaymentProfileDetails,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PaymentProfileDetails {
    payment_profile_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerDetails {
    id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingOptions {
    is_subsequent_auth: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillTo {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address: Option<Secret<String>>,
    city: Option<String>,
    state: Option<Secret<String>>,
    zip: Option<Secret<String>>,
    country: Option<api_enums::CountryAlpha2>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    description: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsequentAuthInformation {
    original_network_trans_id: Secret<String>,
    // original_auth_amount: String, Required for Discover, Diners Club, JCB, and China Union Pay transactions.
    reason: Reason,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Reason {
    Resubmission,
    #[serde(rename = "delayedCharge")]
    DelayedCharge,
    Reauthorization,
    #[serde(rename = "noShow")]
    NoShow,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizationIndicator {
    authorization_indicator: AuthorizationType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionVoidOrCaptureRequest {
    transaction_type: TransactionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    amount: Option<f64>,
    ref_trans_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentsRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    ref_id: Option<String>,
    transaction_request: TransactionRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentCancelOrCaptureRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    transaction_request: TransactionVoidOrCaptureRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
// The connector enforces field ordering, it expects fields to be in the same order as in their API documentation
pub struct CreateCustomerProfileRequest {
    create_customer_profile_request: AuthorizedotnetZeroMandateRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetZeroMandateRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    profile: Profile,
    validation_mode: ValidationMode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    description: String,
    payment_profiles: PaymentProfiles,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentProfiles {
    customer_type: CustomerType,
    payment: PaymentDetails,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CustomerType {
    Individual,
    Business,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationMode {
    // testMode performs a Luhn mod-10 check on the card number, without further validation at connector.
    TestMode,
    // liveMode submits a zero-dollar or one-cent transaction (depending on card type and processor support) to confirm that the card number belongs to an active credit or debit account.
    LiveMode,
}

impl ForeignTryFrom<Value> for Vec<UserField> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(metadata: Value) -> Result<Self, Self::Error> {
        let hashmap: BTreeMap<String, Value> = serde_json::from_str(&metadata.to_string())
            .change_context(errors::ConnectorError::RequestEncodingFailedWithReason(
                "Failed to serialize request metadata".to_owned(),
            ))
            .attach_printable("")?;
        let mut vector: Self = Self::new();
        for (key, value) in hashmap {
            vector.push(UserField {
                name: key,
                value: value.to_string(),
            });
        }
        Ok(vector)
    }
}

impl TryFrom<&types::SetupMandateRouterData> for CreateCustomerProfileRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SetupMandateRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            domain::PaymentMethodData::Card(ccard) => {
                let merchant_authentication =
                    AuthorizedotnetAuthType::try_from(&item.connector_auth_type)?;
                let validation_mode = match item.test_mode {
                    Some(true) | None => ValidationMode::TestMode,
                    Some(false) => ValidationMode::LiveMode,
                };
                Ok(Self {
                    create_customer_profile_request: AuthorizedotnetZeroMandateRequest {
                        merchant_authentication,
                        profile: Profile {
                            // The payment ID is included in the description because the connector requires unique description when creating a mandate.
                            description: item.payment_id.clone(),
                            payment_profiles: PaymentProfiles {
                                customer_type: CustomerType::Individual,
                                payment: PaymentDetails::CreditCard(CreditCardDetails {
                                    card_number: (*ccard.card_number).clone(),
                                    expiration_date: ccard.get_expiry_date_as_yyyymm("-"),
                                    card_code: Some(ccard.card_cvc.clone()),
                                }),
                            },
                        },
                        validation_mode,
                    },
                })
            }
            domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::Wallet(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::RealTimePayment(_)
            | domain::PaymentMethodData::MobilePayment(_)
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::OpenBanking(_)
            | domain::PaymentMethodData::CardToken(_)
            | domain::PaymentMethodData::NetworkToken(_)
            | domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("authorizedotnet"),
                ))?
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetSetupMandateResponse {
    customer_profile_id: Option<String>,
    customer_payment_profile_id_list: Vec<String>,
    validation_direct_response_list: Option<Vec<Secret<String>>>,
    pub messages: ResponseMessages,
}

// zero dollar response
impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            AuthorizedotnetSetupMandateResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            AuthorizedotnetSetupMandateResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.messages.result_code {
            ResultCode::Ok => Ok(Self {
                status: enums::AttemptStatus::Charged,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::NoResponseId,
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(item.response.customer_profile_id.map(
                        |customer_profile_id| types::MandateReference {
                            connector_mandate_id:
                                item.response.customer_payment_profile_id_list.first().map(
                                    |payment_profile_id| {
                                        format!("{customer_profile_id}-{payment_profile_id}")
                                    },
                                ),
                            payment_method_id: None,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: None,
                        },
                    )),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            ResultCode::Error => {
                let error_code = match item.response.messages.message.first() {
                    Some(first_error_message) => first_error_message.code.clone(),
                    None => crate::consts::NO_ERROR_CODE.to_string(),
                };
                let error_reason = item
                    .response
                    .messages
                    .message
                    .iter()
                    .map(|error: &ResponseMessage| error.text.clone())
                    .collect::<Vec<String>>()
                    .join(" ");
                let response = Err(types::ErrorResponse {
                    code: error_code,
                    message: item.response.messages.result_code.to_string(),
                    reason: Some(error_reason),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
// The connector enforces field ordering, it expects fields to be in the same order as in their API documentation
pub struct CreateTransactionRequest {
    create_transaction_request: AuthorizedotnetPaymentsRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrCaptureTransactionRequest {
    create_transaction_request: AuthorizedotnetPaymentCancelOrCaptureRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthorizationType {
    Final,
    Pre,
}

impl TryFrom<enums::CaptureMethod> for AuthorizationType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(capture_method: enums::CaptureMethod) -> Result<Self, Self::Error> {
        match capture_method {
            enums::CaptureMethod::Manual => Ok(Self::Pre),
            enums::CaptureMethod::SequentialAutomatic | enums::CaptureMethod::Automatic => {
                Ok(Self::Final)
            }
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                utils::construct_not_supported_error_report(capture_method, "authorizedotnet"),
            )?,
        }
    }
}

impl TryFrom<&AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>>
    for CreateTransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        let ref_id = if item.router_data.connector_request_reference_id.len() <= 20 {
            Some(item.router_data.connector_request_reference_id.clone())
        } else {
            None
        };

        let transaction_request = match item
            .router_data
            .request
            .mandate_id
            .clone()
            .and_then(|mandate_ids| mandate_ids.mandate_reference_id)
        {
            Some(api_models::payments::MandateReferenceId::NetworkMandateId(network_trans_id)) => {
                TransactionRequest::try_from((item, network_trans_id))?
            }
            Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                connector_mandate_id,
            )) => TransactionRequest::try_from((item, connector_mandate_id))?,
            Some(api_models::payments::MandateReferenceId::NetworkTokenWithNTI(_)) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("authorizedotnet"),
                ))?
            }
            None => {
                match &item.router_data.request.payment_method_data {
                    domain::PaymentMethodData::Card(ccard) => {
                        TransactionRequest::try_from((item, ccard))
                    }
                    domain::PaymentMethodData::Wallet(wallet_data) => {
                        TransactionRequest::try_from((item, wallet_data))
                    }
                    domain::PaymentMethodData::CardRedirect(_)
                    | domain::PaymentMethodData::PayLater(_)
                    | domain::PaymentMethodData::BankRedirect(_)
                    | domain::PaymentMethodData::BankDebit(_)
                    | domain::PaymentMethodData::BankTransfer(_)
                    | domain::PaymentMethodData::Crypto(_)
                    | domain::PaymentMethodData::MandatePayment
                    | domain::PaymentMethodData::Reward
                    | domain::PaymentMethodData::RealTimePayment(_)
                    | domain::PaymentMethodData::MobilePayment(_)
                    | domain::PaymentMethodData::Upi(_)
                    | domain::PaymentMethodData::Voucher(_)
                    | domain::PaymentMethodData::GiftCard(_)
                    | domain::PaymentMethodData::OpenBanking(_)
                    | domain::PaymentMethodData::CardToken(_)
                    | domain::PaymentMethodData::NetworkToken(_)
                    | domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                        Err(errors::ConnectorError::NotImplemented(
                            utils::get_unimplemented_payment_method_error_message(
                                "authorizedotnet",
                            ),
                        ))?
                    }
                }
            }?,
        };
        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentsRequest {
                merchant_authentication,
                ref_id,
                transaction_request,
            },
        })
    }
}

impl
    TryFrom<(
        &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
        String,
    )> for TransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, network_trans_id): (
            &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
            String,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_type: TransactionType::try_from(item.router_data.request.capture_method)?,
            amount: item.amount,
            currency_code: item.router_data.request.currency,
            payment: Some(match item.router_data.request.payment_method_data {
                domain::PaymentMethodData::Card(ref ccard) => {
                    PaymentDetails::CreditCard(CreditCardDetails {
                        card_number: (*ccard.card_number).clone(),
                        expiration_date: ccard.get_expiry_date_as_yyyymm("-"),
                        card_code: None,
                    })
                }
                domain::PaymentMethodData::CardRedirect(_)
                | domain::PaymentMethodData::Wallet(_)
                | domain::PaymentMethodData::PayLater(_)
                | domain::PaymentMethodData::BankRedirect(_)
                | domain::PaymentMethodData::BankDebit(_)
                | domain::PaymentMethodData::BankTransfer(_)
                | domain::PaymentMethodData::Crypto(_)
                | domain::PaymentMethodData::MandatePayment
                | domain::PaymentMethodData::Reward
                | domain::PaymentMethodData::RealTimePayment(_)
                | domain::PaymentMethodData::MobilePayment(_)
                | domain::PaymentMethodData::Upi(_)
                | domain::PaymentMethodData::Voucher(_)
                | domain::PaymentMethodData::GiftCard(_)
                | domain::PaymentMethodData::OpenBanking(_)
                | domain::PaymentMethodData::CardToken(_)
                | domain::PaymentMethodData::NetworkToken(_)
                | domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("authorizedotnet"),
                    ))?
                }
            }),
            profile: None,
            order: Order {
                description: item.router_data.connector_request_reference_id.clone(),
            },
            customer: None,
            bill_to: item
                .router_data
                .get_optional_billing()
                .and_then(|billing_address| billing_address.address.as_ref())
                .map(|address| BillTo {
                    first_name: address.first_name.clone(),
                    last_name: address.last_name.clone(),
                    address: address.line1.clone(),
                    city: address.city.clone(),
                    state: address.state.clone(),
                    zip: address.zip.clone(),
                    country: address.country,
                }),
            user_fields: match item.router_data.request.metadata.clone() {
                Some(metadata) => Some(UserFields {
                    user_field: Vec::<UserField>::foreign_try_from(metadata)?,
                }),
                None => None,
            },
            processing_options: Some(ProcessingOptions {
                is_subsequent_auth: true,
            }),
            subsequent_auth_information: Some(SubsequentAuthInformation {
                original_network_trans_id: Secret::new(network_trans_id),
                reason: Reason::Resubmission,
            }),
            authorization_indicator_type: match item.router_data.request.capture_method {
                Some(capture_method) => Some(AuthorizationIndicator {
                    authorization_indicator: capture_method.try_into()?,
                }),
                None => None,
            },
        })
    }
}

impl
    TryFrom<(
        &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
        api_models::payments::ConnectorMandateReferenceId,
    )> for TransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, connector_mandate_id): (
            &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
            api_models::payments::ConnectorMandateReferenceId,
        ),
    ) -> Result<Self, Self::Error> {
        let mandate_id = connector_mandate_id
            .get_connector_mandate_id()
            .ok_or(errors::ConnectorError::MissingConnectorMandateID)?;
        Ok(Self {
            transaction_type: TransactionType::try_from(item.router_data.request.capture_method)?,
            amount: item.amount,
            currency_code: item.router_data.request.currency,
            payment: None,
            profile: mandate_id
                .split_once('-')
                .map(|(customer_profile_id, payment_profile_id)| {
                    ProfileDetails::CustomerProfileDetails(CustomerProfileDetails {
                        customer_profile_id: Secret::from(customer_profile_id.to_string()),
                        payment_profile: PaymentProfileDetails {
                            payment_profile_id: Secret::from(payment_profile_id.to_string()),
                        },
                    })
                }),
            order: Order {
                description: item.router_data.connector_request_reference_id.clone(),
            },
            customer: None,
            bill_to: None,
            user_fields: match item.router_data.request.metadata.clone() {
                Some(metadata) => Some(UserFields {
                    user_field: Vec::<UserField>::foreign_try_from(metadata)?,
                }),
                None => None,
            },
            processing_options: Some(ProcessingOptions {
                is_subsequent_auth: true,
            }),
            subsequent_auth_information: None,
            authorization_indicator_type: match item.router_data.request.capture_method {
                Some(capture_method) => Some(AuthorizationIndicator {
                    authorization_indicator: capture_method.try_into()?,
                }),
                None => None,
            },
        })
    }
}

impl
    TryFrom<(
        &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
        &domain::Card,
    )> for TransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (
            &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
            &domain::Card,
        ),
    ) -> Result<Self, Self::Error> {
        let (profile, customer) =
            if item
                .router_data
                .request
                .setup_future_usage
                .is_some_and(|future_usage| {
                    matches!(future_usage, common_enums::FutureUsage::OffSession)
                })
                && (item.router_data.request.customer_acceptance.is_some()
                    || item
                        .router_data
                        .request
                        .setup_mandate_details
                        .clone()
                        .is_some_and(|mandate_details| {
                            mandate_details.customer_acceptance.is_some()
                        }))
            {
                (
                    Some(ProfileDetails::CreateProfileDetails(CreateProfileDetails {
                        create_profile: true,
                    })),
                    Some(CustomerDetails {
                        //The payment ID is included in the customer details because the connector requires unique customer information with a length of fewer than 20 characters when creating a mandate.
                        //If the length exceeds 20 characters, a random alphanumeric string is used instead.
                        id: if item.router_data.payment_id.len() <= 20 {
                            item.router_data.payment_id.clone()
                        } else {
                            Alphanumeric.sample_string(&mut rand::thread_rng(), 20)
                        },
                    }),
                )
            } else {
                (None, None)
            };
        Ok(Self {
            transaction_type: TransactionType::try_from(item.router_data.request.capture_method)?,
            amount: item.amount,
            currency_code: item.router_data.request.currency,
            payment: Some(PaymentDetails::CreditCard(CreditCardDetails {
                card_number: (*ccard.card_number).clone(),
                expiration_date: ccard.get_expiry_date_as_yyyymm("-"),
                card_code: Some(ccard.card_cvc.clone()),
            })),
            profile,
            order: Order {
                description: item.router_data.connector_request_reference_id.clone(),
            },
            customer,
            bill_to: item
                .router_data
                .get_optional_billing()
                .and_then(|billing_address| billing_address.address.as_ref())
                .map(|address| BillTo {
                    first_name: address.first_name.clone(),
                    last_name: address.last_name.clone(),
                    address: address.line1.clone(),
                    city: address.city.clone(),
                    state: address.state.clone(),
                    zip: address.zip.clone(),
                    country: address.country,
                }),
            user_fields: match item.router_data.request.metadata.clone() {
                Some(metadata) => Some(UserFields {
                    user_field: Vec::<UserField>::foreign_try_from(metadata)?,
                }),
                None => None,
            },
            processing_options: None,
            subsequent_auth_information: None,
            authorization_indicator_type: match item.router_data.request.capture_method {
                Some(capture_method) => Some(AuthorizationIndicator {
                    authorization_indicator: capture_method.try_into()?,
                }),
                None => None,
            },
        })
    }
}

impl
    TryFrom<(
        &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
        &domain::WalletData,
    )> for TransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, wallet_data): (
            &AuthorizedotnetRouterData<&types::PaymentsAuthorizeRouterData>,
            &domain::WalletData,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_type: TransactionType::try_from(item.router_data.request.capture_method)?,
            amount: item.amount,
            currency_code: item.router_data.request.currency,
            payment: Some(get_wallet_data(
                wallet_data,
                &item.router_data.request.complete_authorize_url,
            )?),
            profile: None,
            order: Order {
                description: item.router_data.connector_request_reference_id.clone(),
            },
            customer: None,
            bill_to: item
                .router_data
                .get_optional_billing()
                .and_then(|billing_address| billing_address.address.as_ref())
                .map(|address| BillTo {
                    first_name: address.first_name.clone(),
                    last_name: address.last_name.clone(),
                    address: address.line1.clone(),
                    city: address.city.clone(),
                    state: address.state.clone(),
                    zip: address.zip.clone(),
                    country: address.country,
                }),
            user_fields: match item.router_data.request.metadata.clone() {
                Some(metadata) => Some(UserFields {
                    user_field: Vec::<UserField>::foreign_try_from(metadata)?,
                }),
                None => None,
            },
            processing_options: None,
            subsequent_auth_information: None,
            authorization_indicator_type: match item.router_data.request.capture_method {
                Some(capture_method) => Some(AuthorizationIndicator {
                    authorization_indicator: capture_method.try_into()?,
                }),
                None => None,
            },
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for CancelOrCaptureTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let transaction_request = TransactionVoidOrCaptureRequest {
            amount: None, //amount is not required for void
            transaction_type: TransactionType::Void,
            ref_trans_id: item.request.connector_transaction_id.to_string(),
        };

        let merchant_authentication = AuthorizedotnetAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentCancelOrCaptureRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

impl TryFrom<&AuthorizedotnetRouterData<&types::PaymentsCaptureRouterData>>
    for CancelOrCaptureTransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let transaction_request = TransactionVoidOrCaptureRequest {
            amount: Some(item.amount),
            transaction_type: TransactionType::Capture,
            ref_trans_id: item
                .router_data
                .request
                .connector_transaction_id
                .to_string(),
        };

        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: AuthorizedotnetPaymentCancelOrCaptureRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub enum AuthorizedotnetPaymentStatus {
    #[serde(rename = "1")]
    Approved,
    #[serde(rename = "2")]
    Declined,
    #[serde(rename = "3")]
    Error,
    #[serde(rename = "4")]
    #[default]
    HeldForReview,
    #[serde(rename = "5")]
    RequiresAction,
}

#[derive(Debug, Clone, serde::Deserialize, Serialize)]
pub enum AuthorizedotnetRefundStatus {
    #[serde(rename = "1")]
    Approved,
    #[serde(rename = "2")]
    Declined,
    #[serde(rename = "3")]
    Error,
    #[serde(rename = "4")]
    HeldForReview,
}

impl ForeignFrom<(AuthorizedotnetPaymentStatus, bool)> for enums::AttemptStatus {
    fn foreign_from((item, auto_capture): (AuthorizedotnetPaymentStatus, bool)) -> Self {
        match item {
            AuthorizedotnetPaymentStatus::Approved => {
                if auto_capture {
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            AuthorizedotnetPaymentStatus::Declined | AuthorizedotnetPaymentStatus::Error => {
                Self::Failure
            }
            AuthorizedotnetPaymentStatus::RequiresAction => Self::AuthenticationPending,
            AuthorizedotnetPaymentStatus::HeldForReview => Self::Pending,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Serialize)]
pub struct ResponseMessage {
    code: String,
    pub text: String,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Serialize, strum::Display)]
enum ResultCode {
    #[default]
    Ok,
    Error,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMessages {
    result_code: ResultCode,
    pub message: Vec<ResponseMessage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMessage {
    pub error_code: String,
    pub error_text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TransactionResponse {
    AuthorizedotnetTransactionResponse(Box<AuthorizedotnetTransactionResponse>),
    AuthorizedotnetTransactionResponseError(Box<AuthorizedotnetTransactionResponseError>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthorizedotnetTransactionResponseError {
    _supplemental_data_qualification_indicator: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetTransactionResponse {
    response_code: AuthorizedotnetPaymentStatus,
    #[serde(rename = "transId")]
    transaction_id: String,
    network_trans_id: Option<Secret<String>>,
    pub(super) account_number: Option<Secret<String>>,
    pub(super) errors: Option<Vec<ErrorMessage>>,
    secure_acceptance: Option<SecureAcceptance>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    response_code: AuthorizedotnetRefundStatus,
    #[serde(rename = "transId")]
    transaction_id: String,
    #[allow(dead_code)]
    network_trans_id: Option<Secret<String>>,
    pub account_number: Option<Secret<String>>,
    pub errors: Option<Vec<ErrorMessage>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SecureAcceptance {
    secure_acceptance_url: Option<url::Url>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentsResponse {
    pub transaction_response: Option<TransactionResponse>,
    pub profile_response: Option<AuthorizedotnetNonZeroMandateResponse>,
    pub messages: ResponseMessages,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetNonZeroMandateResponse {
    customer_profile_id: Option<String>,
    customer_payment_profile_id_list: Option<Vec<String>>,
    pub messages: ResponseMessages,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetVoidResponse {
    pub transaction_response: Option<VoidResponse>,
    pub messages: ResponseMessages,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidResponse {
    response_code: AuthorizedotnetVoidStatus,
    #[serde(rename = "transId")]
    transaction_id: String,
    network_trans_id: Option<Secret<String>>,
    pub account_number: Option<Secret<String>>,
    pub errors: Option<Vec<ErrorMessage>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum AuthorizedotnetVoidStatus {
    #[serde(rename = "1")]
    Approved,
    #[serde(rename = "2")]
    Declined,
    #[serde(rename = "3")]
    Error,
    #[serde(rename = "4")]
    HeldForReview,
}

impl From<AuthorizedotnetVoidStatus> for enums::AttemptStatus {
    fn from(item: AuthorizedotnetVoidStatus) -> Self {
        match item {
            AuthorizedotnetVoidStatus::Approved => Self::Voided,
            AuthorizedotnetVoidStatus::Declined | AuthorizedotnetVoidStatus::Error => {
                Self::VoidFailed
            }
            AuthorizedotnetVoidStatus::HeldForReview => Self::VoidInitiated,
        }
    }
}

impl<F, T>
    ForeignTryFrom<(
        types::ResponseRouterData<
            F,
            AuthorizedotnetPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
        bool,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (item, is_auto_capture): (
            types::ResponseRouterData<
                F,
                AuthorizedotnetPaymentsResponse,
                T,
                types::PaymentsResponseData,
            >,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        match &item.response.transaction_response {
            Some(TransactionResponse::AuthorizedotnetTransactionResponse(transaction_response)) => {
                let status = enums::AttemptStatus::foreign_from((
                    transaction_response.response_code.clone(),
                    is_auto_capture,
                ));
                let error = transaction_response.errors.as_ref().and_then(|errors| {
                    errors.iter().next().map(|error| types::ErrorResponse {
                        code: error.error_code.clone(),
                        message: error.error_text.clone(),
                        reason: Some(error.error_text.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_response.transaction_id.clone()),
                    })
                });
                let metadata = transaction_response
                    .account_number
                    .as_ref()
                    .map(|acc_no| {
                        construct_refund_payment_details(acc_no.clone().expose()).encode_to_value()
                    })
                    .transpose()
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_metadata",
                    })?;
                let url = transaction_response
                    .secure_acceptance
                    .as_ref()
                    .and_then(|x| x.secure_acceptance_url.to_owned());
                let redirection_data =
                    url.map(|url| services::RedirectForm::from((url, services::Method::Get)));
                let mandate_reference = item.response.profile_response.map(|profile_response| {
                    let payment_profile_id = profile_response
                        .customer_payment_profile_id_list
                        .and_then(|customer_payment_profile_id_list| {
                            customer_payment_profile_id_list.first().cloned()
                        });
                    types::MandateReference {
                        connector_mandate_id: profile_response.customer_profile_id.and_then(
                            |customer_profile_id| {
                                payment_profile_id.map(|payment_profile_id| {
                                    format!("{customer_profile_id}-{payment_profile_id}")
                                })
                            },
                        ),
                        payment_method_id: None,
                        mandate_metadata: None,
                        connector_mandate_request_reference_id: None,
                    }
                });

                Ok(Self {
                    status,
                    response: match error {
                        Some(err) => Err(err),
                        None => Ok(types::PaymentsResponseData::TransactionResponse {
                            resource_id: types::ResponseId::ConnectorTransactionId(
                                transaction_response.transaction_id.clone(),
                            ),
                            redirection_data: Box::new(redirection_data),
                            mandate_reference: Box::new(mandate_reference),
                            connector_metadata: metadata,
                            network_txn_id: transaction_response
                                .network_trans_id
                                .clone()
                                .map(|network_trans_id| network_trans_id.expose()),
                            connector_response_reference_id: Some(
                                transaction_response.transaction_id.clone(),
                            ),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                    },
                    ..item.data
                })
            }
            Some(TransactionResponse::AuthorizedotnetTransactionResponseError(_)) | None => {
                Ok(Self {
                    status: enums::AttemptStatus::Failure,
                    response: Err(get_err_response(item.http_code, item.response.messages)?),
                    ..item.data
                })
            }
        }
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, AuthorizedotnetVoidResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            AuthorizedotnetVoidResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match &item.response.transaction_response {
            Some(transaction_response) => {
                let status = enums::AttemptStatus::from(transaction_response.response_code.clone());
                let error = transaction_response.errors.as_ref().and_then(|errors| {
                    errors.iter().next().map(|error| types::ErrorResponse {
                        code: error.error_code.clone(),
                        message: error.error_text.clone(),
                        reason: Some(error.error_text.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_response.transaction_id.clone()),
                    })
                });
                let metadata = transaction_response
                    .account_number
                    .as_ref()
                    .map(|acc_no| {
                        construct_refund_payment_details(acc_no.clone().expose()).encode_to_value()
                    })
                    .transpose()
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_metadata",
                    })?;
                Ok(Self {
                    status,
                    response: match error {
                        Some(err) => Err(err),
                        None => Ok(types::PaymentsResponseData::TransactionResponse {
                            resource_id: types::ResponseId::ConnectorTransactionId(
                                transaction_response.transaction_id.clone(),
                            ),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: metadata,
                            network_txn_id: transaction_response
                                .network_trans_id
                                .clone()
                                .map(|network_trans_id| network_trans_id.expose()),
                            connector_response_reference_id: Some(
                                transaction_response.transaction_id.clone(),
                            ),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                    },
                    ..item.data
                })
            }
            None => Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(get_err_response(item.http_code, item.response.messages)?),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefundTransactionRequest {
    transaction_type: TransactionType,
    amount: f64,
    currency_code: String,
    payment: PaymentDetails,
    #[serde(rename = "refTransId")]
    reference_transaction_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetRefundRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    transaction_request: RefundTransactionRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
// The connector enforces field ordering, it expects fields to be in the same order as in their API documentation
pub struct CreateRefundRequest {
    create_transaction_request: AuthorizedotnetRefundRequest,
}

impl<F> TryFrom<&AuthorizedotnetRouterData<&types::RefundsRouterData<F>>> for CreateRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let payment_details = item
            .router_data
            .request
            .connector_metadata
            .as_ref()
            .get_required_value("connector_metadata")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_metadata",
            })?
            .clone();

        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        let transaction_request = RefundTransactionRequest {
            transaction_type: TransactionType::Refund,
            amount: item.amount,
            payment: payment_details
                .parse_value("PaymentDetails")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_details",
                })?,
            currency_code: item.router_data.request.currency.to_string(),
            reference_transaction_id: item.router_data.request.connector_transaction_id.clone(),
        };

        Ok(Self {
            create_transaction_request: AuthorizedotnetRefundRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}

impl From<AuthorizedotnetRefundStatus> for enums::RefundStatus {
    fn from(item: AuthorizedotnetRefundStatus) -> Self {
        match item {
            AuthorizedotnetRefundStatus::Declined | AuthorizedotnetRefundStatus::Error => {
                Self::Failure
            }
            AuthorizedotnetRefundStatus::Approved | AuthorizedotnetRefundStatus::HeldForReview => {
                Self::Pending
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetRefundResponse {
    pub transaction_response: RefundResponse,
    pub messages: ResponseMessages,
}

impl<F> TryFrom<types::RefundsResponseRouterData<F, AuthorizedotnetRefundResponse>>
    for types::RefundsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<F, AuthorizedotnetRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let transaction_response = &item.response.transaction_response;
        let refund_status = enums::RefundStatus::from(transaction_response.response_code.clone());
        let error = transaction_response.errors.clone().and_then(|errors| {
            errors.first().map(|error| types::ErrorResponse {
                code: error.error_code.clone(),
                message: error.error_text.clone(),
                reason: Some(error.error_text.clone()),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(transaction_response.transaction_id.clone()),
            })
        });

        Ok(Self {
            response: match error {
                Some(err) => Err(err),
                None => Ok(types::RefundsResponseData {
                    connector_refund_id: transaction_response.transaction_id.clone(),
                    refund_status,
                }),
            },
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    merchant_authentication: AuthorizedotnetAuthType,
    #[serde(rename = "transId")]
    transaction_id: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetCreateSyncRequest {
    get_transaction_details_request: TransactionDetails,
}

impl<F> TryFrom<&AuthorizedotnetRouterData<&types::RefundsRouterData<F>>>
    for AuthorizedotnetCreateSyncRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &AuthorizedotnetRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = item.router_data.request.get_connector_refund_id()?;
        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        let payload = Self {
            get_transaction_details_request: TransactionDetails {
                merchant_authentication,
                transaction_id: Some(transaction_id),
            },
        };
        Ok(payload)
    }
}

impl TryFrom<&types::PaymentsSyncRouterData> for AuthorizedotnetCreateSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let transaction_id = Some(
            item.request
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
        );

        let merchant_authentication = AuthorizedotnetAuthType::try_from(&item.connector_auth_type)?;

        let payload = Self {
            get_transaction_details_request: TransactionDetails {
                merchant_authentication,
                transaction_id,
            },
        };
        Ok(payload)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncStatus {
    RefundSettledSuccessfully,
    RefundPendingSettlement,
    AuthorizedPendingCapture,
    CapturedPendingSettlement,
    SettledSuccessfully,
    Declined,
    Voided,
    CouldNotVoid,
    GeneralError,
    #[serde(rename = "FDSPendingReview")]
    FDSPendingReview,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RSyncStatus {
    RefundSettledSuccessfully,
    RefundPendingSettlement,
    Declined,
    GeneralError,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTransactionResponse {
    #[serde(rename = "transId")]
    transaction_id: String,
    transaction_status: SyncStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthorizedotnetSyncResponse {
    transaction: Option<SyncTransactionResponse>,
    messages: ResponseMessages,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RSyncTransactionResponse {
    #[serde(rename = "transId")]
    transaction_id: String,
    transaction_status: RSyncStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthorizedotnetRSyncResponse {
    transaction: Option<RSyncTransactionResponse>,
    messages: ResponseMessages,
}

impl From<SyncStatus> for enums::AttemptStatus {
    fn from(transaction_status: SyncStatus) -> Self {
        match transaction_status {
            SyncStatus::SettledSuccessfully => Self::Charged,
            SyncStatus::CapturedPendingSettlement => Self::CaptureInitiated,
            SyncStatus::AuthorizedPendingCapture => Self::Authorized,
            SyncStatus::Declined => Self::AuthenticationFailed,
            SyncStatus::Voided => Self::Voided,
            SyncStatus::CouldNotVoid => Self::VoidFailed,
            SyncStatus::GeneralError => Self::Failure,
            SyncStatus::RefundSettledSuccessfully
            | SyncStatus::RefundPendingSettlement
            | SyncStatus::FDSPendingReview => Self::Pending,
        }
    }
}

impl From<RSyncStatus> for enums::RefundStatus {
    fn from(transaction_status: RSyncStatus) -> Self {
        match transaction_status {
            RSyncStatus::RefundSettledSuccessfully => Self::Success,
            RSyncStatus::RefundPendingSettlement => Self::Pending,
            RSyncStatus::Declined | RSyncStatus::GeneralError => Self::Failure,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, AuthorizedotnetRSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, AuthorizedotnetRSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.transaction {
            Some(transaction) => {
                let refund_status = enums::RefundStatus::from(transaction.transaction_status);
                Ok(Self {
                    response: Ok(types::RefundsResponseData {
                        connector_refund_id: transaction.transaction_id,
                        refund_status,
                    }),
                    ..item.data
                })
            }
            None => Ok(Self {
                response: Err(get_err_response(item.http_code, item.response.messages)?),
                ..item.data
            }),
        }
    }
}

impl<F, Req>
    TryFrom<
        types::ResponseRouterData<F, AuthorizedotnetSyncResponse, Req, types::PaymentsResponseData>,
    > for types::RouterData<F, Req, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: types::ResponseRouterData<
            F,
            AuthorizedotnetSyncResponse,
            Req,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.transaction {
            Some(transaction) => {
                let payment_status = enums::AttemptStatus::from(transaction.transaction_status);
                Ok(Self {
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(
                            transaction.transaction_id.clone(),
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(transaction.transaction_id.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    status: payment_status,
                    ..item.data
                })
            }
            None => Ok(Self {
                response: Err(get_err_response(item.http_code, item.response.messages)?),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
}

#[derive(Default, Debug, Deserialize)]
pub struct AuthorizedotnetErrorResponse {
    pub error: ErrorDetails,
}

fn construct_refund_payment_details(masked_number: String) -> PaymentDetails {
    PaymentDetails::CreditCard(CreditCardDetails {
        card_number: masked_number.into(),
        expiration_date: "XXXX".to_string().into(),
        card_code: None,
    })
}

impl TryFrom<Option<enums::CaptureMethod>> for TransactionType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(capture_method: Option<enums::CaptureMethod>) -> Result<Self, Self::Error> {
        match capture_method {
            Some(enums::CaptureMethod::Manual) => Ok(Self::Authorization),
            Some(enums::CaptureMethod::SequentialAutomatic)
            | Some(enums::CaptureMethod::Automatic)
            | None => Ok(Self::Payment),
            Some(enums::CaptureMethod::ManualMultiple) => {
                Err(utils::construct_not_supported_error_report(
                    enums::CaptureMethod::ManualMultiple,
                    "authorizedotnet",
                ))?
            }
            Some(enums::CaptureMethod::Scheduled) => {
                Err(utils::construct_not_supported_error_report(
                    enums::CaptureMethod::Scheduled,
                    "authorizedotnet",
                ))?
            }
        }
    }
}

fn get_err_response(
    status_code: u16,
    message: ResponseMessages,
) -> Result<types::ErrorResponse, errors::ConnectorError> {
    let response_message = message
        .message
        .first()
        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
    Ok(types::ErrorResponse {
        code: response_message.code.clone(),
        message: response_message.text.clone(),
        reason: Some(response_message.text.clone()),
        status_code,
        attempt_status: None,
        connector_transaction_id: None,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetWebhookObjectId {
    pub webhook_id: String,
    pub event_type: AuthorizedotnetWebhookEvent,
    pub payload: AuthorizedotnetWebhookPayload,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizedotnetWebhookPayload {
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetWebhookEventType {
    pub event_type: AuthorizedotnetIncomingWebhookEventType,
}

#[derive(Debug, Deserialize)]
pub enum AuthorizedotnetWebhookEvent {
    #[serde(rename = "net.authorize.payment.authorization.created")]
    AuthorizationCreated,
    #[serde(rename = "net.authorize.payment.priorAuthCapture.created")]
    PriorAuthCapture,
    #[serde(rename = "net.authorize.payment.authcapture.created")]
    AuthCapCreated,
    #[serde(rename = "net.authorize.payment.capture.created")]
    CaptureCreated,
    #[serde(rename = "net.authorize.payment.void.created")]
    VoidCreated,
    #[serde(rename = "net.authorize.payment.refund.created")]
    RefundCreated,
}
///Including Unknown to map unknown webhook events
#[derive(Debug, Deserialize)]
pub enum AuthorizedotnetIncomingWebhookEventType {
    #[serde(rename = "net.authorize.payment.authorization.created")]
    AuthorizationCreated,
    #[serde(rename = "net.authorize.payment.priorAuthCapture.created")]
    PriorAuthCapture,
    #[serde(rename = "net.authorize.payment.authcapture.created")]
    AuthCapCreated,
    #[serde(rename = "net.authorize.payment.capture.created")]
    CaptureCreated,
    #[serde(rename = "net.authorize.payment.void.created")]
    VoidCreated,
    #[serde(rename = "net.authorize.payment.refund.created")]
    RefundCreated,
    #[serde(other)]
    Unknown,
}

impl From<AuthorizedotnetIncomingWebhookEventType> for api::IncomingWebhookEvent {
    fn from(event_type: AuthorizedotnetIncomingWebhookEventType) -> Self {
        match event_type {
            AuthorizedotnetIncomingWebhookEventType::AuthorizationCreated
            | AuthorizedotnetIncomingWebhookEventType::PriorAuthCapture
            | AuthorizedotnetIncomingWebhookEventType::AuthCapCreated
            | AuthorizedotnetIncomingWebhookEventType::CaptureCreated
            | AuthorizedotnetIncomingWebhookEventType::VoidCreated => Self::PaymentIntentSuccess,
            AuthorizedotnetIncomingWebhookEventType::RefundCreated => Self::RefundSuccess,
            AuthorizedotnetIncomingWebhookEventType::Unknown => Self::EventNotSupported,
        }
    }
}

impl From<AuthorizedotnetWebhookEvent> for SyncStatus {
    // status mapping reference https://developer.authorize.net/api/reference/features/webhooks.html#Event_Types_and_Payloads
    fn from(event_type: AuthorizedotnetWebhookEvent) -> Self {
        match event_type {
            AuthorizedotnetWebhookEvent::AuthorizationCreated => Self::AuthorizedPendingCapture,
            AuthorizedotnetWebhookEvent::CaptureCreated
            | AuthorizedotnetWebhookEvent::AuthCapCreated => Self::CapturedPendingSettlement,
            AuthorizedotnetWebhookEvent::PriorAuthCapture => Self::SettledSuccessfully,
            AuthorizedotnetWebhookEvent::VoidCreated => Self::Voided,
            AuthorizedotnetWebhookEvent::RefundCreated => Self::RefundSettledSuccessfully,
        }
    }
}

pub fn get_trans_id(
    details: &AuthorizedotnetWebhookObjectId,
) -> Result<String, errors::ConnectorError> {
    details
        .payload
        .id
        .clone()
        .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)
}

impl TryFrom<AuthorizedotnetWebhookObjectId> for AuthorizedotnetSyncResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: AuthorizedotnetWebhookObjectId) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction: Some(SyncTransactionResponse {
                transaction_id: get_trans_id(&item)?,
                transaction_status: SyncStatus::from(item.event_type),
            }),
            messages: ResponseMessages {
                ..Default::default()
            },
        })
    }
}

fn get_wallet_data(
    wallet_data: &domain::WalletData,
    return_url: &Option<String>,
) -> CustomResult<PaymentDetails, errors::ConnectorError> {
    match wallet_data {
        domain::WalletData::GooglePay(_) => Ok(PaymentDetails::OpaqueData(WalletDetails {
            data_descriptor: WalletMethod::Googlepay,
            data_value: Secret::new(wallet_data.get_encoded_wallet_token()?),
        })),
        domain::WalletData::ApplePay(applepay_token) => {
            Ok(PaymentDetails::OpaqueData(WalletDetails {
                data_descriptor: WalletMethod::Applepay,
                data_value: Secret::new(applepay_token.payment_data.clone()),
            }))
        }
        domain::WalletData::PaypalRedirect(_) => Ok(PaymentDetails::PayPal(PayPalDetails {
            success_url: return_url.to_owned(),
            cancel_url: return_url.to_owned(),
        })),
        domain::WalletData::AliPayQr(_)
        | domain::WalletData::AliPayRedirect(_)
        | domain::WalletData::AliPayHkRedirect(_)
        | domain::WalletData::AmazonPayRedirect(_)
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
        | domain::WalletData::PaypalSdk(_)
        | domain::WalletData::Paze(_)
        | domain::WalletData::SamsungPay(_)
        | domain::WalletData::TwintRedirect {}
        | domain::WalletData::VippsRedirect {}
        | domain::WalletData::TouchNGoRedirect(_)
        | domain::WalletData::WeChatPayRedirect(_)
        | domain::WalletData::WeChatPayQr(_)
        | domain::WalletData::CashappQr(_)
        | domain::WalletData::SwishQr(_)
        | domain::WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("authorizedotnet"),
        ))?,
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetQueryParams {
    payer_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaypalConfirmRequest {
    create_transaction_request: PaypalConfirmTransactionRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaypalConfirmTransactionRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    transaction_request: TransactionConfirmRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionConfirmRequest {
    transaction_type: TransactionType,
    payment: PaypalPaymentConfirm,
    ref_trans_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaypalPaymentConfirm {
    pay_pal: Paypal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paypal {
    #[serde(rename = "payerID")]
    payer_id: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaypalQueryParams {
    #[serde(rename = "PayerID")]
    payer_id: Option<Secret<String>>,
}

impl TryFrom<&AuthorizedotnetRouterData<&types::PaymentsCompleteAuthorizeRouterData>>
    for PaypalConfirmRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let params = item
            .router_data
            .request
            .redirect_response
            .as_ref()
            .and_then(|redirect_response| redirect_response.params.as_ref())
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

        let query_params: PaypalQueryParams = serde_urlencoded::from_str(params.peek())
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            .attach_printable("Failed to parse connector response")?;

        let payer_id = query_params.payer_id;

        let transaction_type = match item.router_data.request.capture_method {
            Some(enums::CaptureMethod::Manual) => Ok(TransactionType::ContinueAuthorization),
            Some(enums::CaptureMethod::SequentialAutomatic)
            | Some(enums::CaptureMethod::Automatic)
            | None => Ok(TransactionType::ContinueCapture),
            Some(enums::CaptureMethod::ManualMultiple) => {
                Err(errors::ConnectorError::NotSupported {
                    message: enums::CaptureMethod::ManualMultiple.to_string(),
                    connector: "authorizedotnet",
                })
            }
            Some(enums::CaptureMethod::Scheduled) => Err(errors::ConnectorError::NotSupported {
                message: enums::CaptureMethod::Scheduled.to_string(),
                connector: "authorizedotnet",
            }),
        }?;
        let transaction_request = TransactionConfirmRequest {
            transaction_type,
            payment: PaypalPaymentConfirm {
                pay_pal: Paypal { payer_id },
            },
            ref_trans_id: item.router_data.request.connector_transaction_id.clone(),
        };

        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        Ok(Self {
            create_transaction_request: PaypalConfirmTransactionRequest {
                merchant_authentication,
                transaction_request,
            },
        })
    }
}
