use std::collections::BTreeMap;

use api_models::{payments::AdditionalPaymentData, webhooks::IncomingWebhookEvent};
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::{Encode, OptionExt, ValueExt},
    id_type::CustomerId,
    pii::Email,
    request::Method,
    types::FloatMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData, WalletData},
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        ErrorResponse, RouterData,
    },
    router_flow_types::RSync,
    router_request_types::ResponseId,
    router_response_types::{
        ConnectorCustomerResponseData, MandateReference, PaymentsResponseData, RedirectForm,
        RefundsResponseData,
    },
    types::{
        ConnectorCustomerRouterData, PaymentsAuthorizeRouterData, PaymentsCancelRouterData,
        PaymentsCaptureRouterData, PaymentsCompleteAuthorizeRouterData, PaymentsSyncRouterData,
        RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret, StrongSecret};
use rand::distributions::{Alphanumeric, DistString};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, CardData, ForeignTryFrom, PaymentsAuthorizeRequestData, PaymentsSyncRequestData,
        RefundsRequestData, RouterData as OtherRouterData, WalletData as OtherWalletData,
    },
};

const MAX_ID_LENGTH: usize = 20;
const ADDRESS_MAX_LENGTH: usize = 60;

fn get_random_string() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), MAX_ID_LENGTH)
}

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
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(FloatMajorUnit, T)> for AuthorizedotnetRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (FloatMajorUnit, T)) -> Result<Self, Self::Error> {
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

impl TryFrom<&ConnectorAuthType> for AuthorizedotnetAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
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
    amount: FloatMajorUnit,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    customer_profile_id: Option<Secret<String>>,
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
    email: Option<Email>,
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
    country: Option<enums::CountryAlpha2>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    invoice_number: String,
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
    amount: Option<FloatMajorUnit>,
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
pub struct CustomerRequest {
    create_customer_profile_request: CreateCustomerRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCustomerRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    profile: Profile,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCustomerPaymentProfileRequest {
    create_customer_payment_profile_request: AuthorizedotnetPaymentProfileRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetPaymentProfileRequest {
    merchant_authentication: AuthorizedotnetAuthType,
    customer_profile_id: Secret<String>,
    payment_profile: PaymentProfile,
    validation_mode: ValidationMode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipToList {
    #[serde(skip_serializing_if = "Option::is_none")]
    first_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zip: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    country: Option<enums::CountryAlpha2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone_number: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_customer_id: Option<CustomerId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment_profiles: Option<PaymentProfiles>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ship_to_list: Option<Vec<ShipToList>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentProfiles {
    customer_type: CustomerType,
    #[serde(skip_serializing_if = "Option::is_none")]
    bill_to: Option<BillTo>,
    payment: PaymentDetails,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    bill_to: Option<BillTo>,
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

impl TryFrom<&ConnectorCustomerRouterData> for CustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        let merchant_authentication = AuthorizedotnetAuthType::try_from(&item.connector_auth_type)?;
        let ship_to_list = item.get_optional_shipping().and_then(|shipping| {
            shipping.address.as_ref().map(|address| {
                vec![ShipToList {
                    first_name: address.first_name.clone(),
                    last_name: address.last_name.clone(),
                    address: address.line1.clone(),
                    city: address.city.clone(),
                    state: address.state.clone(),
                    zip: address.zip.clone(),
                    country: address.country,
                    phone_number: shipping
                        .phone
                        .as_ref()
                        .and_then(|phone| phone.number.as_ref().map(|number| number.to_owned())),
                }]
            })
        });

        let merchant_customer_id = match item.customer_id.as_ref() {
            Some(cid) if cid.get_string_repr().len() <= MAX_ID_LENGTH => Some(cid.clone()),
            _ => None,
        };

        Ok(Self {
            create_customer_profile_request: CreateCustomerRequest {
                merchant_authentication,
                profile: Profile {
                    merchant_customer_id,
                    description: None,
                    email: item.request.email.clone(),
                    payment_profiles: None,
                    ship_to_list,
                },
            },
        })
    }
}

impl TryFrom<&SetupMandateRouterData> for CreateCustomerPaymentProfileRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SetupMandateRouterData) -> Result<Self, Self::Error> {
        let merchant_authentication = AuthorizedotnetAuthType::try_from(&item.connector_auth_type)?;
        let validation_mode = match item.test_mode {
            Some(true) | None => ValidationMode::TestMode,
            Some(false) => ValidationMode::LiveMode,
        };
        let customer_profile_id = item.get_connector_customer_id()?.into();

        let bill_to = item
            .get_optional_billing()
            .and_then(|billing_address| billing_address.address.as_ref())
            .map(|address| BillTo {
                first_name: address.first_name.clone(),
                last_name: address.last_name.clone(),
                address: get_address_line(&address.line1, &address.line2, &address.line3),
                city: address.city.clone(),
                state: address.state.clone(),
                zip: address.zip.clone(),
                country: address.country,
            });
        let payment_profile = match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(ccard) => Ok(PaymentProfile {
                bill_to,
                payment: PaymentDetails::CreditCard(CreditCardDetails {
                    card_number: (*ccard.card_number).clone(),
                    expiration_date: ccard.get_expiry_date_as_yyyymm("-"),
                    card_code: Some(ccard.card_cvc.clone()),
                }),
            }),
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::GooglePay(_) => Ok(PaymentProfile {
                    bill_to,
                    payment: PaymentDetails::OpaqueData(WalletDetails {
                        data_descriptor: WalletMethod::Googlepay,
                        data_value: Secret::new(wallet_data.get_encoded_wallet_token()?),
                    }),
                }),
                WalletData::ApplePay(applepay_token) => {
                    let apple_pay_encrypted_data = applepay_token
                        .payment_data
                        .get_encrypted_apple_pay_payment_data_mandatory()
                        .change_context(errors::ConnectorError::MissingRequiredField {
                            field_name: "Apple pay encrypted data",
                        })?;

                    Ok(PaymentProfile {
                        bill_to,
                        payment: PaymentDetails::OpaqueData(WalletDetails {
                            data_descriptor: WalletMethod::Applepay,
                            data_value: Secret::new(apple_pay_encrypted_data.clone()),
                        }),
                    })
                }

                WalletData::AliPayQr(_)
                | WalletData::AliPayRedirect(_)
                | WalletData::AliPayHkRedirect(_)
                | WalletData::AmazonPayRedirect(_)
                | WalletData::Paysera(_)
                | WalletData::BluecodeRedirect {}
                | WalletData::Skrill(_)
                | WalletData::MomoRedirect(_)
                | WalletData::KakaoPayRedirect(_)
                | WalletData::GoPayRedirect(_)
                | WalletData::GcashRedirect(_)
                | WalletData::ApplePayRedirect(_)
                | WalletData::ApplePayThirdPartySdk(_)
                | WalletData::DanaRedirect {}
                | WalletData::GooglePayRedirect(_)
                | WalletData::GooglePayThirdPartySdk(_)
                | WalletData::MbWayRedirect(_)
                | WalletData::MobilePayRedirect(_)
                | WalletData::PaypalRedirect(_)
                | WalletData::AmazonPay(_)
                | WalletData::PaypalSdk(_)
                | WalletData::Paze(_)
                | WalletData::SamsungPay(_)
                | WalletData::TwintRedirect {}
                | WalletData::VippsRedirect {}
                | WalletData::TouchNGoRedirect(_)
                | WalletData::WeChatPayRedirect(_)
                | WalletData::WeChatPayQr(_)
                | WalletData::CashappQr(_)
                | WalletData::SwishQr(_)
                | WalletData::Mifinity(_)
                | WalletData::RevolutPay(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("authorizedotnet"),
                )),
            },
            PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("authorizedotnet"),
                ))
            }
        }?;
        Ok(Self {
            create_customer_payment_profile_request: AuthorizedotnetPaymentProfileRequest {
                merchant_authentication,
                customer_profile_id,
                payment_profile,
                validation_mode,
            },
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetSetupMandateResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    customer_payment_profile_id_list: Vec<String>,
    customer_profile_id: Option<String>,
    #[serde(rename = "customerPaymentProfileId")]
    customer_payment_profile_id: Option<String>,
    validation_direct_response_list: Option<Vec<Secret<String>>>,
    pub messages: ResponseMessages,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedotnetCustomerResponse {
    customer_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    customer_payment_profile_id_list: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    customer_shipping_address_id_list: Vec<String>,
    pub messages: ResponseMessages,
}

fn extract_customer_id(text: &str) -> Option<String> {
    let re = Regex::new(r"ID (\d+)").ok()?;
    re.captures(text)
        .and_then(|captures| captures.get(1))
        .map(|capture_match| capture_match.as_str().to_string())
}

impl<F, T> TryFrom<ResponseRouterData<F, AuthorizedotnetCustomerResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AuthorizedotnetCustomerResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.messages.result_code {
            ResultCode::Ok => match item.response.customer_profile_id.clone() {
                Some(connector_customer_id) => Ok(Self {
                    response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                        ConnectorCustomerResponseData::new_with_customer_id(connector_customer_id),
                    )),
                    ..item.data
                }),
                None => Err(
                    errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                        "Missing customer profile id from Authorizedotnet".to_string(),
                    ))
                    .into(),
                ),
            },
            ResultCode::Error => {
                let error_message = item.response.messages.message.first();
                if let Some(connector_customer_id) =
                    error_message.and_then(|error| extract_customer_id(&error.text))
                {
                    Ok(Self {
                        response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                            ConnectorCustomerResponseData::new_with_customer_id(
                                connector_customer_id,
                            ),
                        )),
                        ..item.data
                    })
                } else {
                    let error_code = error_message.map(|error| error.code.clone());
                    let error_code = error_code.unwrap_or_else(|| {
                        hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()
                    });
                    let error_reason = item
                        .response
                        .messages
                        .message
                        .iter()
                        .map(|error: &ResponseMessage| error.text.clone())
                        .collect::<Vec<String>>()
                        .join(" ");
                    let response = Err(ErrorResponse {
                        code: error_code,
                        message: item.response.messages.result_code.to_string(),
                        reason: Some(error_reason),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
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
}

// zero dollar response
impl<F, T>
    TryFrom<ResponseRouterData<F, AuthorizedotnetSetupMandateResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AuthorizedotnetSetupMandateResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_customer_id = item.data.get_connector_customer_id()?;
        if item.response.customer_profile_id.is_some() {
            Ok(Self {
                status: enums::AttemptStatus::Charged,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(Some(MandateReference {
                        connector_mandate_id: item
                            .response
                            .customer_payment_profile_id_list
                            .first()
                            .or(item.response.customer_payment_profile_id.as_ref())
                            .map(|payment_profile_id| {
                                format!("{connector_customer_id}-{payment_profile_id}")
                            }),
                        payment_method_id: None,
                        mandate_metadata: None,
                        connector_mandate_request_reference_id: None,
                    })),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        } else {
            let error_message = item.response.messages.message.first();
            let error_code = error_message.map(|error| error.code.clone());
            let error_code = error_code
                .unwrap_or_else(|| hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string());
            let error_reason = item
                .response
                .messages
                .message
                .iter()
                .map(|error: &ResponseMessage| error.text.clone())
                .collect::<Vec<String>>()
                .join(" ");
            let response = Err(ErrorResponse {
                code: error_code,
                message: item.response.messages.result_code.to_string(),
                reason: Some(error_reason),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            });
            Ok(Self {
                response,
                status: enums::AttemptStatus::Failure,
                ..item.data
            })
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

impl TryFrom<&AuthorizedotnetRouterData<&PaymentsAuthorizeRouterData>>
    for CreateTransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            return Err(errors::ConnectorError::NotSupported {
                message: "3DS flow".to_string(),
                connector: "Authorizedotnet",
            }
            .into());
        };

        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        let ref_id = if item.router_data.connector_request_reference_id.len() <= MAX_ID_LENGTH {
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
                    PaymentMethodData::Card(ccard) => TransactionRequest::try_from((item, ccard)),
                    PaymentMethodData::Wallet(wallet_data) => {
                        TransactionRequest::try_from((item, wallet_data))
                    }
                    PaymentMethodData::CardRedirect(_)
                    | PaymentMethodData::PayLater(_)
                    | PaymentMethodData::BankRedirect(_)
                    | PaymentMethodData::BankDebit(_)
                    | PaymentMethodData::BankTransfer(_)
                    | PaymentMethodData::Crypto(_)
                    | PaymentMethodData::MandatePayment
                    | PaymentMethodData::Reward
                    | PaymentMethodData::RealTimePayment(_)
                    | PaymentMethodData::MobilePayment(_)
                    | PaymentMethodData::Upi(_)
                    | PaymentMethodData::Voucher(_)
                    | PaymentMethodData::GiftCard(_)
                    | PaymentMethodData::OpenBanking(_)
                    | PaymentMethodData::CardToken(_)
                    | PaymentMethodData::NetworkToken(_)
                    | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
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
        &AuthorizedotnetRouterData<&PaymentsAuthorizeRouterData>,
        String,
    )> for TransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, network_trans_id): (
            &AuthorizedotnetRouterData<&PaymentsAuthorizeRouterData>,
            String,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_type: TransactionType::try_from(item.router_data.request.capture_method)?,
            amount: item.amount,
            currency_code: item.router_data.request.currency,
            payment: Some(match item.router_data.request.payment_method_data {
                PaymentMethodData::Card(ref ccard) => {
                    PaymentDetails::CreditCard(CreditCardDetails {
                        card_number: (*ccard.card_number).clone(),
                        expiration_date: ccard.get_expiry_date_as_yyyymm("-"),
                        card_code: None,
                    })
                }
                PaymentMethodData::CardRedirect(_)
                | PaymentMethodData::Wallet(_)
                | PaymentMethodData::PayLater(_)
                | PaymentMethodData::BankRedirect(_)
                | PaymentMethodData::BankDebit(_)
                | PaymentMethodData::BankTransfer(_)
                | PaymentMethodData::Crypto(_)
                | PaymentMethodData::MandatePayment
                | PaymentMethodData::Reward
                | PaymentMethodData::RealTimePayment(_)
                | PaymentMethodData::MobilePayment(_)
                | PaymentMethodData::Upi(_)
                | PaymentMethodData::Voucher(_)
                | PaymentMethodData::GiftCard(_)
                | PaymentMethodData::OpenBanking(_)
                | PaymentMethodData::CardToken(_)
                | PaymentMethodData::NetworkToken(_)
                | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("authorizedotnet"),
                    ))?
                }
            }),
            profile: None,
            order: Order {
                invoice_number: match &item.router_data.request.merchant_order_reference_id {
                    Some(merchant_order_reference_id) => {
                        if merchant_order_reference_id.len() <= MAX_ID_LENGTH {
                            merchant_order_reference_id.to_string()
                        } else {
                            get_random_string()
                        }
                    }
                    None => get_random_string(),
                },

                description: item.router_data.connector_request_reference_id.clone(),
            },
            customer: Some(CustomerDetails {
                id: if item.router_data.payment_id.len() <= MAX_ID_LENGTH {
                    item.router_data.payment_id.clone()
                } else {
                    get_random_string()
                },
                email: item.router_data.request.get_optional_email(),
            }),
            bill_to: item
                .router_data
                .get_optional_billing()
                .and_then(|billing_address| billing_address.address.as_ref())
                .map(|address| BillTo {
                    first_name: address.first_name.clone(),
                    last_name: address.last_name.clone(),
                    address: get_address_line(&address.line1, &address.line2, &address.line3),
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
fn get_address_line(
    address_line1: &Option<Secret<String>>,
    address_line2: &Option<Secret<String>>,
    address_line3: &Option<Secret<String>>,
) -> Option<Secret<String>> {
    for lines in [
        vec![address_line1, address_line2, address_line3],
        vec![address_line1, address_line2],
    ] {
        let combined: String = lines
            .into_iter()
            .flatten()
            .map(|s| s.clone().expose())
            .collect::<Vec<_>>()
            .join(" ");

        if !combined.is_empty() && combined.len() <= ADDRESS_MAX_LENGTH {
            return Some(Secret::new(combined));
        }
    }
    address_line1.clone()
}

impl
    TryFrom<(
        &AuthorizedotnetRouterData<&PaymentsAuthorizeRouterData>,
        api_models::payments::ConnectorMandateReferenceId,
    )> for TransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, connector_mandate_id): (
            &AuthorizedotnetRouterData<&PaymentsAuthorizeRouterData>,
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
                invoice_number: match &item.router_data.request.merchant_order_reference_id {
                    Some(merchant_order_reference_id) => {
                        if merchant_order_reference_id.len() <= MAX_ID_LENGTH {
                            merchant_order_reference_id.to_string()
                        } else {
                            get_random_string()
                        }
                    }
                    None => get_random_string(),
                },

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
        &AuthorizedotnetRouterData<&PaymentsAuthorizeRouterData>,
        &Card,
    )> for TransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (
            &AuthorizedotnetRouterData<&PaymentsAuthorizeRouterData>,
            &Card,
        ),
    ) -> Result<Self, Self::Error> {
        let profile = if item
            .router_data
            .request
            .is_customer_initiated_mandate_payment()
        {
            let connector_customer_id =
                Secret::new(item.router_data.connector_customer.clone().ok_or(
                    errors::ConnectorError::MissingConnectorRelatedTransactionID {
                        id: "connector_customer_id".to_string(),
                    },
                )?);
            Some(ProfileDetails::CreateProfileDetails(CreateProfileDetails {
                create_profile: true,
                customer_profile_id: Some(connector_customer_id),
            }))
        } else {
            None
        };

        let customer = if !item
            .router_data
            .request
            .is_customer_initiated_mandate_payment()
        {
            item.router_data.customer_id.as_ref().and_then(|customer| {
                let customer_id = customer.get_string_repr();
                (customer_id.len() <= MAX_ID_LENGTH).then_some(CustomerDetails {
                    id: customer_id.to_string(),
                    email: item.router_data.request.get_optional_email(),
                })
            })
        } else {
            None
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
                invoice_number: match &item.router_data.request.merchant_order_reference_id {
                    Some(merchant_order_reference_id) => {
                        if merchant_order_reference_id.len() <= MAX_ID_LENGTH {
                            merchant_order_reference_id.to_string()
                        } else {
                            get_random_string()
                        }
                    }
                    None => get_random_string(),
                },

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
                    address: get_address_line(&address.line1, &address.line2, &address.line3),
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
        &AuthorizedotnetRouterData<&PaymentsAuthorizeRouterData>,
        &WalletData,
    )> for TransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, wallet_data): (
            &AuthorizedotnetRouterData<&PaymentsAuthorizeRouterData>,
            &WalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let profile = if item
            .router_data
            .request
            .is_customer_initiated_mandate_payment()
        {
            let connector_customer_id =
                Secret::new(item.router_data.connector_customer.clone().ok_or(
                    errors::ConnectorError::MissingConnectorRelatedTransactionID {
                        id: "connector_customer_id".to_string(),
                    },
                )?);
            Some(ProfileDetails::CreateProfileDetails(CreateProfileDetails {
                create_profile: true,
                customer_profile_id: Some(connector_customer_id),
            }))
        } else {
            None
        };

        let customer = if !item
            .router_data
            .request
            .is_customer_initiated_mandate_payment()
        {
            item.router_data.customer_id.as_ref().and_then(|customer| {
                let customer_id = customer.get_string_repr();
                (customer_id.len() <= MAX_ID_LENGTH).then_some(CustomerDetails {
                    id: customer_id.to_string(),
                    email: item.router_data.request.get_optional_email(),
                })
            })
        } else {
            None
        };

        Ok(Self {
            transaction_type: TransactionType::try_from(item.router_data.request.capture_method)?,
            amount: item.amount,
            currency_code: item.router_data.request.currency,
            payment: Some(get_wallet_data(
                wallet_data,
                &item.router_data.request.complete_authorize_url,
            )?),
            profile,
            order: Order {
                invoice_number: match &item.router_data.request.merchant_order_reference_id {
                    Some(merchant_order_reference_id) => {
                        if merchant_order_reference_id.len() <= MAX_ID_LENGTH {
                            merchant_order_reference_id.to_string()
                        } else {
                            get_random_string()
                        }
                    }
                    None => get_random_string(),
                },

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
                    address: get_address_line(&address.line1, &address.line2, &address.line3),
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

impl TryFrom<&PaymentsCancelRouterData> for CancelOrCaptureTransactionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
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

impl TryFrom<&AuthorizedotnetRouterData<&PaymentsCaptureRouterData>>
    for CancelOrCaptureTransactionRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&PaymentsCaptureRouterData>,
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

fn get_payment_status(
    (item, auto_capture): (AuthorizedotnetPaymentStatus, bool),
) -> enums::AttemptStatus {
    match item {
        AuthorizedotnetPaymentStatus::Approved => {
            if auto_capture {
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Authorized
            }
        }
        AuthorizedotnetPaymentStatus::Declined | AuthorizedotnetPaymentStatus::Error => {
            enums::AttemptStatus::Failure
        }
        AuthorizedotnetPaymentStatus::RequiresAction => enums::AttemptStatus::AuthenticationPending,
        AuthorizedotnetPaymentStatus::HeldForReview => enums::AttemptStatus::Pending,
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
    avs_result_code: Option<String>,
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

fn get_avs_response_description(code: &str) -> Option<&'static str> {
    match code {
        "A" => Some("The street address matched, but the postal code did not."),
        "B" => Some("No address information was provided."),
        "E" => Some(
            "AVS data provided is invalid or AVS is not allowed for the card type that was used.",
        ),
        "G" => Some("The card was issued by a bank outside the U.S. and does not support AVS."),
        "N" => Some("Neither the street address nor postal code matched."),
        "P" => Some("AVS is not applicable for this transaction."),
        "R" => Some("Retry — AVS was unavailable or timed out."),
        "S" => Some("AVS is not supported by card issuer."),
        "U" => Some("Address information is unavailable."),
        "W" => Some("The US ZIP+4 code matches, but the street address does not."),
        "X" => Some("Both the street address and the US ZIP+4 code matched."),
        "Y" => Some("The street address and postal code matched."),
        "Z" => Some("The postal code matched, but the street address did not."),
        _ => None,
    }
}

fn convert_to_additional_payment_method_connector_response(
    transaction_response: &AuthorizedotnetTransactionResponse,
) -> Option<AdditionalPaymentMethodConnectorResponse> {
    match transaction_response.avs_result_code.as_deref() {
        Some("P") | None => None,
        Some(code) => {
            let description = get_avs_response_description(code);
            let payment_checks = serde_json::json!({
                "avs_result_code": code,
                "description": description
            });
            Some(AdditionalPaymentMethodConnectorResponse::Card {
                authentication_data: None,
                payment_checks: Some(payment_checks),
                card_network: None,
                domestic_network: None,
            })
        }
    }
}

impl<F, T>
    ForeignTryFrom<(
        ResponseRouterData<F, AuthorizedotnetPaymentsResponse, T, PaymentsResponseData>,
        bool,
    )> for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (item, is_auto_capture): (
            ResponseRouterData<F, AuthorizedotnetPaymentsResponse, T, PaymentsResponseData>,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        match &item.response.transaction_response {
            Some(TransactionResponse::AuthorizedotnetTransactionResponse(transaction_response)) => {
                let status = get_payment_status((
                    transaction_response.response_code.clone(),
                    is_auto_capture,
                ));
                let error = transaction_response.errors.as_ref().and_then(|errors| {
                    errors.iter().next().map(|error| ErrorResponse {
                        code: error.error_code.clone(),
                        message: error.error_text.clone(),
                        reason: Some(error.error_text.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_response.transaction_id.clone()),
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                });
                let metadata = transaction_response
                    .account_number
                    .as_ref()
                    .map(|acc_no| {
                        construct_refund_payment_details(PaymentDetailAccountNumber::Masked(
                            acc_no.clone().expose(),
                        ))
                        .encode_to_value()
                    })
                    .transpose()
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_metadata",
                    })?;

                let connector_response_data =
                    convert_to_additional_payment_method_connector_response(transaction_response)
                        .map(ConnectorResponseData::with_additional_payment_method_data);

                let url = transaction_response
                    .secure_acceptance
                    .as_ref()
                    .and_then(|x| x.secure_acceptance_url.to_owned());
                let redirection_data = url.map(|url| RedirectForm::from((url, Method::Get)));
                let mandate_reference = item.response.profile_response.map(|profile_response| {
                    let payment_profile_id = profile_response
                        .customer_payment_profile_id_list
                        .and_then(|customer_payment_profile_id_list| {
                            customer_payment_profile_id_list.first().cloned()
                        });
                    MandateReference {
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
                        None => Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
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
                    connector_response: connector_response_data,
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

impl<F, T> TryFrom<ResponseRouterData<F, AuthorizedotnetVoidResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AuthorizedotnetVoidResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match &item.response.transaction_response {
            Some(transaction_response) => {
                let status = enums::AttemptStatus::from(transaction_response.response_code.clone());
                let error = transaction_response.errors.as_ref().and_then(|errors| {
                    errors.iter().next().map(|error| ErrorResponse {
                        code: error.error_code.clone(),
                        message: error.error_text.clone(),
                        reason: Some(error.error_text.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(transaction_response.transaction_id.clone()),
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                });
                let metadata = transaction_response
                    .account_number
                    .as_ref()
                    .map(|acc_no| {
                        construct_refund_payment_details(PaymentDetailAccountNumber::Masked(
                            acc_no.clone().expose(),
                        ))
                        .encode_to_value()
                    })
                    .transpose()
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_metadata",
                    })?;
                Ok(Self {
                    status,
                    response: match error {
                        Some(err) => Err(err),
                        None => Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
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
    amount: FloatMajorUnit,
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

impl<F> TryFrom<&AuthorizedotnetRouterData<&RefundsRouterData<F>>> for CreateRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let merchant_authentication =
            AuthorizedotnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        let transaction_request = RefundTransactionRequest {
            transaction_type: TransactionType::Refund,
            amount: item.amount,
            payment: get_refund_metadata(
                &item.router_data.request.connector_metadata,
                &item.router_data.request.additional_payment_method_data,
            )?,
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

fn get_refund_metadata(
    connector_metadata: &Option<Value>,
    additional_payment_method: &Option<AdditionalPaymentData>,
) -> Result<PaymentDetails, error_stack::Report<errors::ConnectorError>> {
    let payment_details_from_metadata = connector_metadata
        .as_ref()
        .get_required_value("connector_metadata")
        .ok()
        .and_then(|value| {
            value
                .clone()
                .parse_value::<PaymentDetails>("PaymentDetails")
                .ok()
        });
    let payment_details_from_additional_payment_method = match additional_payment_method {
        Some(AdditionalPaymentData::Card(additional_card_info)) => {
            additional_card_info.last4.clone().map(|last4| {
                construct_refund_payment_details(PaymentDetailAccountNumber::UnMasked(
                    last4.to_string(),
                ))
            })
        }
        _ => None,
    };
    match (
        payment_details_from_metadata,
        payment_details_from_additional_payment_method,
    ) {
        (Some(payment_detail), _) => Ok(payment_detail),
        (_, Some(payment_detail)) => Ok(payment_detail),
        (None, None) => Err(errors::ConnectorError::MissingRequiredField {
            field_name: "payment_details",
        }
        .into()),
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

impl<F> TryFrom<RefundsResponseRouterData<F, AuthorizedotnetRefundResponse>>
    for RefundsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<F, AuthorizedotnetRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let transaction_response = &item.response.transaction_response;
        let refund_status = enums::RefundStatus::from(transaction_response.response_code.clone());
        let error = transaction_response.errors.clone().and_then(|errors| {
            errors.first().map(|error| ErrorResponse {
                code: error.error_code.clone(),
                message: error.error_text.clone(),
                reason: Some(error.error_text.clone()),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(transaction_response.transaction_id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        });

        Ok(Self {
            response: match error {
                Some(err) => Err(err),
                None => Ok(RefundsResponseData {
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

impl<F> TryFrom<&AuthorizedotnetRouterData<&RefundsRouterData<F>>>
    for AuthorizedotnetCreateSyncRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &AuthorizedotnetRouterData<&RefundsRouterData<F>>,
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

impl TryFrom<&PaymentsSyncRouterData> for AuthorizedotnetCreateSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
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
    Voided,
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
            SyncStatus::SettledSuccessfully | SyncStatus::CapturedPendingSettlement => {
                Self::Charged
            }
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
            RSyncStatus::Declined | RSyncStatus::GeneralError | RSyncStatus::Voided => {
                Self::Failure
            }
        }
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, AuthorizedotnetRSyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: RefundsResponseRouterData<RSync, AuthorizedotnetRSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.transaction {
            Some(transaction) => {
                let refund_status = enums::RefundStatus::from(transaction.transaction_status);
                Ok(Self {
                    response: Ok(RefundsResponseData {
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

impl<F, Req> TryFrom<ResponseRouterData<F, AuthorizedotnetSyncResponse, Req, PaymentsResponseData>>
    for RouterData<F, Req, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, AuthorizedotnetSyncResponse, Req, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.transaction {
            Some(transaction) => {
                let payment_status = enums::AttemptStatus::from(transaction.transaction_status);
                Ok(Self {
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
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

            // E00053 indicates "server too busy"
            // E00104 indicates "Server in maintenance"
            // If the server is too busy or Server in maintenance, we return the already available data
            None => match item
                .response
                .messages
                .message
                .iter()
                .find(|msg| msg.code == "E00053" || msg.code == "E00104")
            {
                Some(_) => Ok(item.data),
                None => Ok(Self {
                    response: Err(get_err_response(item.http_code, item.response.messages)?),
                    ..item.data
                }),
            },
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct AuthorizedotnetErrorResponse {
    pub error: ErrorDetails,
}
enum PaymentDetailAccountNumber {
    Masked(String),
    UnMasked(String),
}
fn construct_refund_payment_details(detail: PaymentDetailAccountNumber) -> PaymentDetails {
    PaymentDetails::CreditCard(CreditCardDetails {
        card_number: match detail {
            PaymentDetailAccountNumber::Masked(masked) => masked.into(),
            PaymentDetailAccountNumber::UnMasked(unmasked) => format!("XXXX{:}", unmasked).into(),
        },
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
) -> Result<ErrorResponse, errors::ConnectorError> {
    let response_message = message
        .message
        .first()
        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
    Ok(ErrorResponse {
        code: response_message.code.clone(),
        message: response_message.text.clone(),
        reason: Some(response_message.text.clone()),
        status_code,
        attempt_status: None,
        connector_transaction_id: None,
        network_advice_code: None,
        network_decline_code: None,
        network_error_message: None,
        connector_metadata: None,
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

impl From<AuthorizedotnetIncomingWebhookEventType> for IncomingWebhookEvent {
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
    wallet_data: &WalletData,
    return_url: &Option<String>,
) -> CustomResult<PaymentDetails, errors::ConnectorError> {
    match wallet_data {
        WalletData::GooglePay(_) => Ok(PaymentDetails::OpaqueData(WalletDetails {
            data_descriptor: WalletMethod::Googlepay,
            data_value: Secret::new(wallet_data.get_encoded_wallet_token()?),
        })),
        WalletData::ApplePay(applepay_token) => {
            let apple_pay_encrypted_data = applepay_token
                .payment_data
                .get_encrypted_apple_pay_payment_data_mandatory()
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "Apple pay encrypted data",
                })?;
            Ok(PaymentDetails::OpaqueData(WalletDetails {
                data_descriptor: WalletMethod::Applepay,
                data_value: Secret::new(apple_pay_encrypted_data.clone()),
            }))
        }
        WalletData::PaypalRedirect(_) => Ok(PaymentDetails::PayPal(PayPalDetails {
            success_url: return_url.to_owned(),
            cancel_url: return_url.to_owned(),
        })),
        WalletData::AliPayQr(_)
        | WalletData::AliPayRedirect(_)
        | WalletData::AliPayHkRedirect(_)
        | WalletData::AmazonPay(_)
        | WalletData::AmazonPayRedirect(_)
        | WalletData::Paysera(_)
        | WalletData::Skrill(_)
        | WalletData::BluecodeRedirect {}
        | WalletData::MomoRedirect(_)
        | WalletData::KakaoPayRedirect(_)
        | WalletData::GoPayRedirect(_)
        | WalletData::GcashRedirect(_)
        | WalletData::ApplePayRedirect(_)
        | WalletData::ApplePayThirdPartySdk(_)
        | WalletData::DanaRedirect {}
        | WalletData::GooglePayRedirect(_)
        | WalletData::GooglePayThirdPartySdk(_)
        | WalletData::MbWayRedirect(_)
        | WalletData::MobilePayRedirect(_)
        | WalletData::PaypalSdk(_)
        | WalletData::Paze(_)
        | WalletData::SamsungPay(_)
        | WalletData::TwintRedirect {}
        | WalletData::VippsRedirect {}
        | WalletData::TouchNGoRedirect(_)
        | WalletData::WeChatPayRedirect(_)
        | WalletData::WeChatPayQr(_)
        | WalletData::CashappQr(_)
        | WalletData::SwishQr(_)
        | WalletData::Mifinity(_)
        | WalletData::RevolutPay(_) => Err(errors::ConnectorError::NotImplemented(
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

impl TryFrom<&AuthorizedotnetRouterData<&PaymentsCompleteAuthorizeRouterData>>
    for PaypalConfirmRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthorizedotnetRouterData<&PaymentsCompleteAuthorizeRouterData>,
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
