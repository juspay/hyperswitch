use base64::Engine;
use common_enums::enums;
use common_utils::{consts, pii};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{GooglePayWalletData, PaymentMethodData, WalletData},
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        ErrorResponse, RouterData,
    },
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData,
        ResponseId,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::{api, errors};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    constants,
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, AddressDetailsData, CardData, PaymentsAuthorizeRequestData, PaymentsSyncRequestData,
        RouterData as OtherRouterData,
    },
};
pub struct BarclaycardAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_account: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BarclaycardAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
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

pub struct BarclaycardRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, api_models::enums::Currency, i64, T)>
    for BarclaycardRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &api::CurrencyUnit,
            api_models::enums::Currency,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardPaymentsRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    consumer_authentication_information: Option<BarclaycardConsumerAuthInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInformation {
    commerce_indicator: String,
    capture: Option<bool>,
    payment_solution: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantDefinedInformation {
    key: u8,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardConsumerAuthInformation {
    ucaf_collection_indicator: Option<String>,
    cavv: Option<String>,
    ucaf_authentication_data: Option<Secret<String>>,
    xid: Option<String>,
    directory_server_transaction_id: Option<Secret<String>>,
    specification_version: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPaymentInformation {
    card: Card,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayPaymentInformation {
    fluid_data: FluidData,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentInformation {
    Cards(Box<CardPaymentInformation>),
    GooglePay(Box<GooglePayPaymentInformation>),
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
pub struct FluidData {
    value: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformationWithBill {
    amount_details: Amount,
    bill_to: Option<BillTo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    total_amount: String,
    currency: api_models::enums::Currency,
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
    country: enums::CountryAlpha2,
    email: pii::Email,
}

fn truncate_string(state: &Secret<String>, max_len: usize) -> Secret<String> {
    let exposed = state.clone().expose();
    let truncated = exposed.get(..max_len).unwrap_or(&exposed);
    Secret::new(truncated.to_string())
}

fn build_bill_to(
    address_details: &hyperswitch_domain_models::address::AddressDetails,
    email: pii::Email,
) -> Result<BillTo, error_stack::Report<errors::ConnectorError>> {
    let administrative_area = address_details
        .to_state_code_as_optional()
        .unwrap_or_else(|_| {
            address_details
                .get_state()
                .ok()
                .map(|state| truncate_string(state, 20))
        })
        .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
            field_name: "billing_address.state",
        })?;

    Ok(BillTo {
        first_name: address_details.get_first_name()?.clone(),
        last_name: address_details.get_last_name()?.clone(),
        address1: address_details.get_line1()?.clone(),
        locality: address_details.get_city()?.clone(),
        administrative_area,
        postal_code: address_details.get_zip()?.clone(),
        country: address_details.get_country()?.to_owned(),
        email,
    })
}

fn get_barclaycard_card_type(card_network: common_enums::CardNetwork) -> Option<&'static str> {
    match card_network {
        common_enums::CardNetwork::Visa => Some("001"),
        common_enums::CardNetwork::Mastercard => Some("002"),
        common_enums::CardNetwork::AmericanExpress => Some("003"),
        common_enums::CardNetwork::JCB => Some("007"),
        common_enums::CardNetwork::DinersClub => Some("005"),
        common_enums::CardNetwork::Discover => Some("004"),
        common_enums::CardNetwork::CartesBancaires => Some("006"),
        common_enums::CardNetwork::UnionPay => Some("062"),
        //"042" is the type code for Masetro Cards(International). For Maestro Cards(UK-Domestic) the mapping should be "024"
        common_enums::CardNetwork::Maestro => Some("042"),
        common_enums::CardNetwork::Interac
        | common_enums::CardNetwork::RuPay
        | common_enums::CardNetwork::Star
        | common_enums::CardNetwork::Accel
        | common_enums::CardNetwork::Pulse
        | common_enums::CardNetwork::Nyce => None,
    }
}

#[derive(Debug, Serialize)]
pub enum PaymentSolution {
    GooglePay,
}

impl From<PaymentSolution> for String {
    fn from(solution: PaymentSolution) -> Self {
        let payment_solution = match solution {
            PaymentSolution::GooglePay => "012",
        };
        payment_solution.to_string()
    }
}

impl
    From<(
        &BarclaycardRouterData<&PaymentsAuthorizeRouterData>,
        Option<BillTo>,
    )> for OrderInformationWithBill
{
    fn from(
        (item, bill_to): (
            &BarclaycardRouterData<&PaymentsAuthorizeRouterData>,
            Option<BillTo>,
        ),
    ) -> Self {
        Self {
            amount_details: Amount {
                total_amount: item.amount.to_owned(),
                currency: item.router_data.request.currency,
            },
            bill_to,
        }
    }
}

impl
    TryFrom<(
        &BarclaycardRouterData<&PaymentsAuthorizeRouterData>,
        Option<PaymentSolution>,
        Option<String>,
    )> for ProcessingInformation
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (item, solution, network): (
            &BarclaycardRouterData<&PaymentsAuthorizeRouterData>,
            Option<PaymentSolution>,
            Option<String>,
        ),
    ) -> Result<Self, Self::Error> {
        let commerce_indicator = get_commerce_indicator(network);

        Ok(Self {
            capture: Some(matches!(
                item.router_data.request.capture_method,
                Some(enums::CaptureMethod::Automatic) | None
            )),
            payment_solution: solution.map(String::from),
            commerce_indicator,
        })
    }
}

impl From<&BarclaycardRouterData<&PaymentsAuthorizeRouterData>> for ClientReferenceInformation {
    fn from(item: &BarclaycardRouterData<&PaymentsAuthorizeRouterData>) -> Self {
        Self {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        }
    }
}

fn convert_metadata_to_merchant_defined_info(metadata: Value) -> Vec<MerchantDefinedInformation> {
    let hashmap: std::collections::BTreeMap<String, Value> =
        serde_json::from_str(&metadata.to_string()).unwrap_or(std::collections::BTreeMap::new());
    let mut vector = Vec::new();
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReferenceInformation {
    code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientProcessorInformation {
    avs: Option<Avs>,
    card_verification: Option<CardVerification>,
    processor: Option<ProcessorResponse>,
    network_transaction_id: Option<Secret<String>>,
    approval_code: Option<String>,
    merchant_advice: Option<MerchantAdvice>,
    response_code: Option<String>,
    ach_verification: Option<AchVerification>,
    system_trace_audit_number: Option<String>,
    event_status: Option<String>,
    retrieval_reference_number: Option<String>,
    consumer_authentication_response: Option<ConsumerAuthenticationResponse>,
    response_details: Option<String>,
    transaction_id: Option<Secret<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantAdvice {
    code: Option<String>,
    code_raw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumerAuthenticationResponse {
    code: Option<String>,
    code_raw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchVerification {
    result_code_raw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorResponse {
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardVerification {
    result_code: Option<String>,
    result_code_raw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRiskInformation {
    rules: Option<Vec<ClientRiskInformationRules>>,
    profile: Option<Profile>,
    score: Option<Score>,
    info_codes: Option<InfoCodes>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoCodes {
    address: Option<Vec<String>>,
    identity_change: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Score {
    factor_codes: Option<Vec<String>>,
    result: Option<RiskResult>,
    model_used: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RiskResult {
    StringVariant(String),
    IntVariant(u64),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    early_decision: Option<String>,
    name: Option<String>,
    decision: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientRiskInformationRules {
    name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Avs {
    code: Option<String>,
    code_raw: Option<String>,
}

impl
    TryFrom<(
        &BarclaycardRouterData<&PaymentsAuthorizeRouterData>,
        hyperswitch_domain_models::payment_method_data::Card,
    )> for BarclaycardPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (
            &BarclaycardRouterData<&PaymentsAuthorizeRouterData>,
            hyperswitch_domain_models::payment_method_data::Card,
        ),
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "Card 3DS".to_string(),
                connector: "Barclaycard",
            })?
        };

        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing_address()?, email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));
        let payment_information = PaymentInformation::try_from(&ccard)?;
        let processing_information = ProcessingInformation::try_from((item, None, None))?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information = item
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);

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

impl
    TryFrom<(
        &BarclaycardRouterData<&PaymentsAuthorizeRouterData>,
        GooglePayWalletData,
    )> for BarclaycardPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, google_pay_data): (
            &BarclaycardRouterData<&PaymentsAuthorizeRouterData>,
            GooglePayWalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing_address()?, email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));
        let payment_information = PaymentInformation::try_from(&google_pay_data)?;
        let processing_information =
            ProcessingInformation::try_from((item, Some(PaymentSolution::GooglePay), None))?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information = item
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);

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

impl TryFrom<&BarclaycardRouterData<&PaymentsAuthorizeRouterData>> for BarclaycardPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BarclaycardRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(ccard) => Self::try_from((item, ccard)),
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::GooglePay(google_pay_data) => Self::try_from((item, google_pay_data)),
                WalletData::AliPayQr(_)
                | WalletData::AliPayRedirect(_)
                | WalletData::AliPayHkRedirect(_)
                | WalletData::AmazonPayRedirect(_)
                | WalletData::ApplePay(_)
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
                | WalletData::PaypalSdk(_)
                | WalletData::Paze(_)
                | WalletData::RevolutPay(_)
                | WalletData::SamsungPay(_)
                | WalletData::TwintRedirect {}
                | WalletData::VippsRedirect {}
                | WalletData::TouchNGoRedirect(_)
                | WalletData::WeChatPayRedirect(_)
                | WalletData::WeChatPayQr(_)
                | WalletData::CashappQr(_)
                | WalletData::SwishQr(_)
                | WalletData::Paysera(_)
                | WalletData::Skrill(_)
                | WalletData::BluecodeRedirect {}
                | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Barclaycard"),
                )
                .into()),
            },
            PaymentMethodData::MandatePayment
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
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
                    utils::get_unimplemented_payment_method_error_message("Barclaycard"),
                )
                .into())
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BarclaycardPaymentStatus {
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

fn map_barclaycard_attempt_status(
    (status, auto_capture): (BarclaycardPaymentStatus, bool),
) -> enums::AttemptStatus {
    match status {
        BarclaycardPaymentStatus::Authorized
        | BarclaycardPaymentStatus::AuthorizedPendingReview => {
            if auto_capture {
                // Because Barclaycard will return Payment Status as Authorized even in AutoCapture Payment
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Authorized
            }
        }
        BarclaycardPaymentStatus::Pending => {
            if auto_capture {
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Pending
            }
        }
        BarclaycardPaymentStatus::Succeeded | BarclaycardPaymentStatus::Transmitted => {
            enums::AttemptStatus::Charged
        }
        BarclaycardPaymentStatus::Voided
        | BarclaycardPaymentStatus::Reversed
        | BarclaycardPaymentStatus::Cancelled => enums::AttemptStatus::Voided,
        BarclaycardPaymentStatus::Failed
        | BarclaycardPaymentStatus::Declined
        | BarclaycardPaymentStatus::AuthorizedRiskDeclined
        | BarclaycardPaymentStatus::InvalidRequest
        | BarclaycardPaymentStatus::Rejected
        | BarclaycardPaymentStatus::ServerError => enums::AttemptStatus::Failure,
        BarclaycardPaymentStatus::PendingAuthentication => {
            enums::AttemptStatus::AuthenticationPending
        }
        BarclaycardPaymentStatus::PendingReview
        | BarclaycardPaymentStatus::Challenge
        | BarclaycardPaymentStatus::Accepted => enums::AttemptStatus::Pending,
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BarclaycardPaymentsResponse {
    ClientReferenceInformation(Box<BarclaycardClientReferenceResponse>),
    ErrorInformation(Box<BarclaycardErrorInformationResponse>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardClientReferenceResponse {
    id: String,
    status: BarclaycardPaymentStatus,
    client_reference_information: ClientReferenceInformation,
    processor_information: Option<ClientProcessorInformation>,
    processing_information: Option<ProcessingInformationResponse>,
    payment_information: Option<PaymentInformationResponse>,
    payment_insights_information: Option<PaymentInsightsInformation>,
    risk_information: Option<ClientRiskInformation>,
    error_information: Option<BarclaycardErrorInformation>,
    issuer_information: Option<IssuerInformation>,
    sender_information: Option<SenderInformation>,
    payment_account_information: Option<PaymentAccountInformation>,
    reconciliation_id: Option<String>,
    consumer_authentication_information: Option<ConsumerAuthenticationInformation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumerAuthenticationInformation {
    eci_raw: Option<String>,
    eci: Option<String>,
    acs_transaction_id: Option<String>,
    cavv: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SenderInformation {
    payment_information: Option<PaymentInformationResponse>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInsightsInformation {
    response_insights: Option<ResponseInsights>,
    rule_results: Option<RuleResults>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseInsights {
    category_code: Option<String>,
    category: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleResults {
    id: Option<String>,
    decision: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInformationResponse {
    tokenized_card: Option<CardResponseObject>,
    customer: Option<CustomerResponseObject>,
    card: Option<CardResponseObject>,
    scheme: Option<String>,
    bin: Option<String>,
    account_type: Option<String>,
    issuer: Option<String>,
    bin_country: Option<enums::CountryAlpha2>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerResponseObject {
    customer_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAccountInformation {
    card: Option<PaymentAccountCardInformation>,
    features: Option<PaymentAccountFeatureInformation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAccountFeatureInformation {
    health_card: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAccountCardInformation {
    #[serde(rename = "type")]
    card_type: Option<String>,
    hashed_number: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInformationResponse {
    payment_solution: Option<String>,
    commerce_indicator: Option<String>,
    commerce_indicator_label: Option<String>,
    ecommerce_indicator: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuerInformation {
    country: Option<enums::CountryAlpha2>,
    discretionary_data: Option<String>,
    country_specific_discretionary_data: Option<String>,
    response_code: Option<String>,
    pin_request_indicator: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardResponseObject {
    suffix: Option<String>,
    prefix: Option<String>,
    expiration_month: Option<Secret<String>>,
    expiration_year: Option<Secret<String>>,
    #[serde(rename = "type")]
    card_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardErrorInformationResponse {
    id: String,
    error_information: BarclaycardErrorInformation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BarclaycardErrorInformation {
    reason: Option<String>,
    message: Option<String>,
    details: Option<Vec<Details>>,
}

fn map_error_response<F, T>(
    error_response: &BarclaycardErrorInformationResponse,
    item: ResponseRouterData<F, BarclaycardPaymentsResponse, T, PaymentsResponseData>,
    transaction_status: Option<enums::AttemptStatus>,
) -> RouterData<F, T, PaymentsResponseData> {
    let detailed_error_info = error_response
        .error_information
        .details
        .as_ref()
        .map(|details| {
            details
                .iter()
                .map(|details| format!("{} : {}", details.field, details.reason))
                .collect::<Vec<_>>()
                .join(", ")
        });

    let reason = get_error_reason(
        error_response.error_information.message.clone(),
        detailed_error_info,
        None,
    );
    let response = Err(ErrorResponse {
        code: error_response
            .error_information
            .reason
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
        message: error_response
            .error_information
            .reason
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
        reason,
        status_code: item.http_code,
        attempt_status: None,
        connector_transaction_id: Some(error_response.id.clone()),
        network_advice_code: None,
        network_decline_code: None,
        network_error_message: None,
        connector_metadata: None,
    });

    match transaction_status {
        Some(status) => RouterData {
            response,
            status,
            ..item.data
        },
        None => RouterData {
            response,
            ..item.data
        },
    }
}

fn get_error_response_if_failure(
    (info_response, status, http_code): (
        &BarclaycardClientReferenceResponse,
        enums::AttemptStatus,
        u16,
    ),
) -> Option<ErrorResponse> {
    if utils::is_payment_failure(status) {
        Some(get_error_response(
            &info_response.error_information,
            &info_response.processor_information,
            &info_response.risk_information,
            Some(status),
            http_code,
            info_response.id.clone(),
        ))
    } else {
        None
    }
}

fn get_payment_response(
    (info_response, status, http_code): (
        &BarclaycardClientReferenceResponse,
        enums::AttemptStatus,
        u16,
    ),
) -> Result<PaymentsResponseData, Box<ErrorResponse>> {
    let error_response = get_error_response_if_failure((info_response, status, http_code));
    match error_response {
        Some(error) => Err(Box::new(error)),
        None => Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(info_response.id.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(
                info_response
                    .client_reference_information
                    .code
                    .clone()
                    .unwrap_or(info_response.id.clone()),
            ),
            incremental_authorization_allowed: None,
            charges: None,
        }),
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            BarclaycardPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BarclaycardPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BarclaycardPaymentsResponse::ClientReferenceInformation(info_response) => {
                let status = map_barclaycard_attempt_status((
                    info_response.status.clone(),
                    item.data.request.is_auto_capture()?,
                ));
                let response = get_payment_response((&info_response, status, item.http_code))
                    .map_err(|err| *err);
                let connector_response = match item.data.payment_method {
                    common_enums::PaymentMethod::Card => info_response
                        .processor_information
                        .as_ref()
                        .and_then(|processor_information| {
                            info_response
                                .consumer_authentication_information
                                .as_ref()
                                .map(|consumer_auth_information| {
                                    convert_to_additional_payment_method_connector_response(
                                        processor_information,
                                        consumer_auth_information,
                                    )
                                })
                        })
                        .map(ConnectorResponseData::with_additional_payment_method_data),
                    common_enums::PaymentMethod::CardRedirect
                    | common_enums::PaymentMethod::PayLater
                    | common_enums::PaymentMethod::Wallet
                    | common_enums::PaymentMethod::BankRedirect
                    | common_enums::PaymentMethod::BankTransfer
                    | common_enums::PaymentMethod::Crypto
                    | common_enums::PaymentMethod::BankDebit
                    | common_enums::PaymentMethod::Reward
                    | common_enums::PaymentMethod::RealTimePayment
                    | common_enums::PaymentMethod::MobilePayment
                    | common_enums::PaymentMethod::Upi
                    | common_enums::PaymentMethod::Voucher
                    | common_enums::PaymentMethod::OpenBanking
                    | common_enums::PaymentMethod::GiftCard => None,
                };

                Ok(Self {
                    status,
                    response,
                    connector_response,
                    ..item.data
                })
            }
            BarclaycardPaymentsResponse::ErrorInformation(ref error_response) => {
                Ok(map_error_response(
                    &error_response.clone(),
                    item,
                    Some(enums::AttemptStatus::Failure),
                ))
            }
        }
    }
}

fn convert_to_additional_payment_method_connector_response(
    processor_information: &ClientProcessorInformation,
    consumer_authentication_information: &ConsumerAuthenticationInformation,
) -> AdditionalPaymentMethodConnectorResponse {
    let payment_checks = Some(serde_json::json!({
        "avs_response": processor_information.avs,
        "card_verification": processor_information.card_verification,
        "approval_code": processor_information.approval_code,
        "consumer_authentication_response": processor_information.consumer_authentication_response,
        "cavv": consumer_authentication_information.cavv,
        "eci": consumer_authentication_information.eci,
        "eci_raw": consumer_authentication_information.eci_raw,
    }));

    let authentication_data = Some(serde_json::json!({
        "retrieval_reference_number": processor_information.retrieval_reference_number,
        "acs_transaction_id": consumer_authentication_information.acs_transaction_id,
        "system_trace_audit_number": processor_information.system_trace_audit_number,
    }));

    AdditionalPaymentMethodConnectorResponse::Card {
        authentication_data,
        payment_checks,
        card_network: None,
        domestic_network: None,
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            BarclaycardPaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BarclaycardPaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BarclaycardPaymentsResponse::ClientReferenceInformation(info_response) => {
                let status = map_barclaycard_attempt_status((info_response.status.clone(), true));
                let response = get_payment_response((&info_response, status, item.http_code))
                    .map_err(|err| *err);
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            BarclaycardPaymentsResponse::ErrorInformation(ref error_response) => {
                Ok(map_error_response(&error_response.clone(), item, None))
            }
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            BarclaycardPaymentsResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BarclaycardPaymentsResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            BarclaycardPaymentsResponse::ClientReferenceInformation(info_response) => {
                let status = map_barclaycard_attempt_status((info_response.status.clone(), false));
                let response = get_payment_response((&info_response, status, item.http_code))
                    .map_err(|err| *err);
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            BarclaycardPaymentsResponse::ErrorInformation(ref error_response) => {
                Ok(map_error_response(&error_response.clone(), item, None))
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardTransactionResponse {
    id: String,
    application_information: ApplicationInformation,
    client_reference_information: Option<ClientReferenceInformation>,
    processor_information: Option<ClientProcessorInformation>,
    processing_information: Option<ProcessingInformationResponse>,
    payment_information: Option<PaymentInformationResponse>,
    payment_insights_information: Option<PaymentInsightsInformation>,
    error_information: Option<BarclaycardErrorInformation>,
    fraud_marking_information: Option<FraudMarkingInformation>,
    risk_information: Option<ClientRiskInformation>,
    reconciliation_id: Option<String>,
    consumer_authentication_information: Option<ConsumerAuthenticationInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FraudMarkingInformation {
    reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInformation {
    status: Option<BarclaycardPaymentStatus>,
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            BarclaycardTransactionResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            BarclaycardTransactionResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.application_information.status {
            Some(app_status) => {
                let status = map_barclaycard_attempt_status((
                    app_status,
                    item.data.request.is_auto_capture()?,
                ));

                let connector_response = match item.data.payment_method {
                    common_enums::PaymentMethod::Card => item
                        .response
                        .processor_information
                        .as_ref()
                        .and_then(|processor_information| {
                            item.response
                                .consumer_authentication_information
                                .as_ref()
                                .map(|consumer_auth_information| {
                                    convert_to_additional_payment_method_connector_response(
                                        processor_information,
                                        consumer_auth_information,
                                    )
                                })
                        })
                        .map(ConnectorResponseData::with_additional_payment_method_data),
                    common_enums::PaymentMethod::CardRedirect
                    | common_enums::PaymentMethod::PayLater
                    | common_enums::PaymentMethod::Wallet
                    | common_enums::PaymentMethod::BankRedirect
                    | common_enums::PaymentMethod::BankTransfer
                    | common_enums::PaymentMethod::Crypto
                    | common_enums::PaymentMethod::BankDebit
                    | common_enums::PaymentMethod::Reward
                    | common_enums::PaymentMethod::RealTimePayment
                    | common_enums::PaymentMethod::MobilePayment
                    | common_enums::PaymentMethod::Upi
                    | common_enums::PaymentMethod::Voucher
                    | common_enums::PaymentMethod::OpenBanking
                    | common_enums::PaymentMethod::GiftCard => None,
                };

                let risk_info: Option<ClientRiskInformation> = None;
                if utils::is_payment_failure(status) {
                    Ok(Self {
                        response: Err(get_error_response(
                            &item.response.error_information,
                            &item.response.processor_information,
                            &risk_info,
                            Some(status),
                            item.http_code,
                            item.response.id.clone(),
                        )),
                        status: enums::AttemptStatus::Failure,
                        connector_response,
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                item.response.id.clone(),
                            ),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: item
                                .response
                                .client_reference_information
                                .map(|cref| cref.code)
                                .unwrap_or(Some(item.response.id)),
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        connector_response,
                        ..item.data
                    })
                }
            }
            None => Ok(Self {
                status: item.data.status,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.id),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformation {
    amount_details: Amount,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardCaptureRequest {
    order_information: OrderInformation,
    client_reference_information: ClientReferenceInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

impl TryFrom<&BarclaycardRouterData<&PaymentsCaptureRouterData>> for BarclaycardCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: &BarclaycardRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let merchant_defined_information = value
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);
        Ok(Self {
            order_information: OrderInformation {
                amount_details: Amount {
                    total_amount: value.amount.to_owned(),
                    currency: value.router_data.request.currency,
                },
            },
            client_reference_information: ClientReferenceInformation {
                code: Some(value.router_data.connector_request_reference_id.clone()),
            },
            merchant_defined_information,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardVoidRequest {
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

impl TryFrom<&BarclaycardRouterData<&PaymentsCancelRouterData>> for BarclaycardVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: &BarclaycardRouterData<&PaymentsCancelRouterData>,
    ) -> Result<Self, Self::Error> {
        let merchant_defined_information = value
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardRefundRequest {
    order_information: OrderInformation,
    client_reference_information: ClientReferenceInformation,
}

impl<F> TryFrom<&BarclaycardRouterData<&RefundsRouterData<F>>> for BarclaycardRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &BarclaycardRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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

impl From<BarclaycardRefundResponse> for enums::RefundStatus {
    fn from(item: BarclaycardRefundResponse) -> Self {
        let error_reason = item
            .error_information
            .and_then(|error_info| error_info.reason);
        match item.status {
            BarclaycardRefundStatus::Succeeded | BarclaycardRefundStatus::Transmitted => {
                Self::Success
            }
            BarclaycardRefundStatus::Cancelled
            | BarclaycardRefundStatus::Failed
            | BarclaycardRefundStatus::Voided => Self::Failure,
            BarclaycardRefundStatus::Pending => Self::Pending,
            BarclaycardRefundStatus::TwoZeroOne => {
                if error_reason == Some("PROCESSOR_DECLINED".to_string()) {
                    Self::Failure
                } else {
                    Self::Pending
                }
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardRefundResponse {
    id: String,
    status: BarclaycardRefundStatus,
    error_information: Option<BarclaycardErrorInformation>,
}

impl TryFrom<RefundsResponseRouterData<Execute, BarclaycardRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, BarclaycardRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.clone());
        let response = if utils::is_refund_failure(refund_status) {
            Err(get_error_response(
                &item.response.error_information,
                &None,
                &None,
                None,
                item.http_code,
                item.response.id,
            ))
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BarclaycardRefundStatus {
    Succeeded,
    Transmitted,
    Failed,
    Pending,
    Voided,
    Cancelled,
    #[serde(rename = "201")]
    TwoZeroOne,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RsyncApplicationInformation {
    status: Option<BarclaycardRefundStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardRsyncResponse {
    id: String,
    application_information: Option<RsyncApplicationInformation>,
    error_information: Option<BarclaycardErrorInformation>,
}

impl TryFrom<RefundsResponseRouterData<RSync, BarclaycardRsyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, BarclaycardRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item
            .response
            .application_information
            .and_then(|application_information| application_information.status)
        {
            Some(status) => {
                let error_reason = item
                    .response
                    .error_information
                    .clone()
                    .and_then(|error_info| error_info.reason);
                let refund_status = match status {
                    BarclaycardRefundStatus::Succeeded | BarclaycardRefundStatus::Transmitted => {
                        enums::RefundStatus::Success
                    }
                    BarclaycardRefundStatus::Cancelled
                    | BarclaycardRefundStatus::Failed
                    | BarclaycardRefundStatus::Voided => enums::RefundStatus::Failure,
                    BarclaycardRefundStatus::Pending => enums::RefundStatus::Pending,
                    BarclaycardRefundStatus::TwoZeroOne => {
                        if error_reason == Some("PROCESSOR_DECLINED".to_string()) {
                            enums::RefundStatus::Failure
                        } else {
                            enums::RefundStatus::Pending
                        }
                    }
                };
                if utils::is_refund_failure(refund_status) {
                    if status == BarclaycardRefundStatus::Voided {
                        Err(get_error_response(
                            &Some(BarclaycardErrorInformation {
                                message: Some(constants::REFUND_VOIDED.to_string()),
                                reason: Some(constants::REFUND_VOIDED.to_string()),
                                details: None,
                            }),
                            &None,
                            &None,
                            None,
                            item.http_code,
                            item.response.id.clone(),
                        ))
                    } else {
                        Err(get_error_response(
                            &item.response.error_information,
                            &None,
                            &None,
                            None,
                            item.http_code,
                            item.response.id.clone(),
                        ))
                    }
                } else {
                    Ok(RefundsResponseData {
                        connector_refund_id: item.response.id,
                        refund_status,
                    })
                }
            }

            None => Ok(RefundsResponseData {
                connector_refund_id: item.response.id.clone(),
                refund_status: match item.data.response {
                    Ok(response) => response.refund_status,
                    Err(_) => common_enums::RefundStatus::Pending,
                },
            }),
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardStandardErrorResponse {
    pub error_information: Option<ErrorInformation>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarclaycardServerErrorResponse {
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
pub struct BarclaycardAuthenticationErrorResponse {
    pub response: AuthenticationErrorInformation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BarclaycardErrorResponse {
    AuthenticationError(BarclaycardAuthenticationErrorResponse),
    StandardError(BarclaycardStandardErrorResponse),
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
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AuthenticationErrorInformation {
    pub rmsg: String,
}

fn get_error_response(
    error_data: &Option<BarclaycardErrorInformation>,
    processor_information: &Option<ClientProcessorInformation>,
    risk_information: &Option<ClientRiskInformation>,
    attempt_status: Option<enums::AttemptStatus>,
    status_code: u16,
    transaction_id: String,
) -> ErrorResponse {
    let avs_message = risk_information
        .clone()
        .map(|client_risk_information| {
            client_risk_information.rules.map(|rules| {
                rules
                    .iter()
                    .map(|risk_info| {
                        risk_info.name.clone().map_or("".to_string(), |name| {
                            format!(" , {}", name.clone().expose())
                        })
                    })
                    .collect::<Vec<String>>()
                    .join("")
            })
        })
        .unwrap_or(Some("".to_string()));

    let detailed_error_info = error_data.to_owned().and_then(|error_info| {
        error_info.details.map(|error_details| {
            error_details
                .iter()
                .map(|details| format!("{} : {}", details.field, details.reason))
                .collect::<Vec<_>>()
                .join(", ")
        })
    });
    let network_decline_code = processor_information
        .as_ref()
        .and_then(|info| info.response_code.clone());
    let network_advice_code = processor_information.as_ref().and_then(|info| {
        info.merchant_advice
            .as_ref()
            .and_then(|merchant_advice| merchant_advice.code_raw.clone())
    });

    let reason = get_error_reason(
        error_data
            .clone()
            .and_then(|error_details| error_details.message),
        detailed_error_info,
        avs_message,
    );
    let error_message = error_data
        .clone()
        .and_then(|error_details| error_details.reason);

    ErrorResponse {
        code: error_message
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
        message: error_message
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
        reason,
        status_code,
        attempt_status,
        connector_transaction_id: Some(transaction_id.clone()),
        network_advice_code,
        network_decline_code,
        network_error_message: None,
        connector_metadata: None,
    }
}

impl TryFrom<&hyperswitch_domain_models::payment_method_data::Card> for PaymentInformation {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        ccard: &hyperswitch_domain_models::payment_method_data::Card,
    ) -> Result<Self, Self::Error> {
        let card_type = match ccard
            .card_network
            .clone()
            .and_then(get_barclaycard_card_type)
        {
            Some(card_network) => Some(card_network.to_string()),
            None => ccard.get_card_issuer().ok().map(String::from),
        };
        Ok(Self::Cards(Box::new(CardPaymentInformation {
            card: Card {
                number: ccard.card_number.clone(),
                expiration_month: ccard.card_exp_month.clone(),
                expiration_year: ccard.card_exp_year.clone(),
                security_code: ccard.card_cvc.clone(),
                card_type,
            },
        })))
    }
}

impl TryFrom<&GooglePayWalletData> for PaymentInformation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(google_pay_data: &GooglePayWalletData) -> Result<Self, Self::Error> {
        Ok(Self::GooglePay(Box::new(GooglePayPaymentInformation {
            fluid_data: FluidData {
                value: Secret::from(
                    consts::BASE64_ENGINE.encode(
                        google_pay_data
                            .tokenization_data
                            .get_encrypted_google_pay_token()
                            .change_context(errors::ConnectorError::MissingRequiredField {
                                field_name: "gpay wallet_token",
                            })?
                            .clone(),
                    ),
                ),
            },
        })))
    }
}

fn get_commerce_indicator(network: Option<String>) -> String {
    match network {
        Some(card_network) => match card_network.to_lowercase().as_str() {
            "amex" => "aesk",
            "discover" => "dipb",
            "mastercard" => "spa",
            "visa" => "internet",
            _ => "internet",
        },
        None => "internet",
    }
    .to_string()
}

pub fn get_error_reason(
    error_info: Option<String>,
    detailed_error_info: Option<String>,
    avs_error_info: Option<String>,
) -> Option<String> {
    match (error_info, detailed_error_info, avs_error_info) {
        (Some(message), Some(details), Some(avs_message)) => Some(format!(
            "{message}, detailed_error_information: {details}, avs_message: {avs_message}",
        )),
        (Some(message), Some(details), None) => {
            Some(format!("{message}, detailed_error_information: {details}"))
        }
        (Some(message), None, Some(avs_message)) => {
            Some(format!("{message}, avs_message: {avs_message}"))
        }
        (None, Some(details), Some(avs_message)) => {
            Some(format!("{details}, avs_message: {avs_message}"))
        }
        (Some(message), None, None) => Some(message),
        (None, Some(details), None) => Some(details),
        (None, None, Some(avs_message)) => Some(avs_message),
        (None, None, None) => None,
    }
}
