use api_models::admin::{AdditionalMerchantData, MerchantRecipientData};
use common_enums::enums;
use common_utils::types::MinorUnit;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::refunds::Execute,
    router_request_types::{
        MandateRevokeRequestData, PaymentsAuthorizeData, PaymentsSyncData, SetupMandateRequestData,
    },
    router_response_types::{MandateReference, MandateRevokeResponseData, PaymentsResponseData},
    types::{
        PaymentsAuthorizeRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

fn get_merchant_reference(
    additional_merchant_data: &Option<AdditionalMerchantData>,
    fallback: &str,
) -> String {
    additional_merchant_data
        .as_ref()
        .and_then(|data| match data {
            AdditionalMerchantData::OpenBankingRecipientData(recipient_data) => {
                match recipient_data {
                    MerchantRecipientData::ConnectorRecipientId(id) => Some(id.peek().to_string()),
                    _ => None,
                }
            }
        })
        .unwrap_or_else(|| fallback.to_string())
}

// Auth type for Capitec VRP - OAuth2 Password Flow
#[derive(Debug, Clone)]
pub struct CapitecvrpAuthType {
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub username: Secret<String>,
    pub password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CapitecvrpAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Ok(Self {
                client_id: api_key.to_owned(),
                client_secret: key1.to_owned(),
                username: api_secret.to_owned(),
                password: key2.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// Access Token Request for OAuth2 Password Flow
#[derive(Debug, Serialize)]
pub struct CapitecvrpAccessTokenRequest {
    pub grant_type: String,
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub username: Secret<String>,
    pub password: Secret<String>,
    pub resource: Secret<String>,
    pub scope: String,
}


#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct CapitecvrpAccessTokenResponse {
    pub access_token: Secret<String>,
    pub token_type: String,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub expires_in: i64,
}

impl<F, T> TryFrom<ResponseRouterData<F, CapitecvrpAccessTokenResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CapitecvrpAccessTokenResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClientIdentifierKey {
    Cellphone,
    Idnumber,
    Accountnumber,
    Capitecpay,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitecvrpClient {
    pub identifier_key: ClientIdentifierKey,
    pub identifier_value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnceOffPaymentConsent {
    pub merchant_reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_description: Option<String>,
    pub beneficiary_statement_description: String,
    pub client_statement_description: String,
    pub minimum_amount: i64,
    pub maximum_amount: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip_amount: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitecvrpOnceOffConsentRequest {
    pub client: CapitecvrpClient,
    pub payment_consent: OnceOffPaymentConsent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

// Recurring consent request structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecurringInterval {
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Biannually,
    Annually,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recurrence {
    pub first_payment_date: String,
    pub interval: RecurringInterval,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrences: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurringPaymentConsent {
    pub merchant_reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_amount: Option<i64>,
    pub maximum_amount: i64,
    pub recurrence: Recurrence,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitecvrpRecurringConsentRequest {
    pub client: CapitecvrpClient,
    pub payment_consent: RecurringPaymentConsent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

// Consent response
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitecvrpConsentResponse {
    pub consent_id: String,
    pub expiry_date_time: String,
    pub time_to_live: i32,
    pub last_payment_date_time: Option<String>,
    pub client_app_notified: bool,
}

// Consent status response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConsentStatus {
    Pending,
    Approved,
    Declined,
    Timeout,
    Fraud,
    InsufficientFunds,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitecvrpConsentStatusResponse {
    pub status: ConsentStatus,
    pub consent_receipt: Option<String>,
    pub client_response_description: Option<String>,
}

impl From<ConsentStatus> for enums::AttemptStatus {
    fn from(status: ConsentStatus) -> Self {
        match status {
            ConsentStatus::Pending => Self::AuthenticationPending,
            ConsentStatus::Approved => Self::Authorized,
            ConsentStatus::Declined => Self::AuthenticationFailed,
            ConsentStatus::Timeout => Self::Failure,
            ConsentStatus::Fraud => Self::Failure,
            ConsentStatus::InsufficientFunds => Self::Failure,
            ConsentStatus::Cancelled => Self::Voided,
        }
    }
}

// Payment action requests
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitecvrpOnceOffPaymentRequest {
    pub consent_receipt: String,
    pub beneficiary_statement_description: String,
    pub client_statement_description: String,
    pub amount: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitecvrpRecurringPaymentRequest {
    pub consent_receipt: String,
    pub beneficiary_statement_description: String,
    pub client_statement_description: String,
    pub amount: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

// Payment responses
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodInfo {
    pub account_number: String,
    #[serde(rename = "type")]
    pub payment_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitecvrpPaymentResponse {
    pub effective_date: String,
    pub batch_settlement_id: Option<String>,
    pub batch_settlement_description: Option<String>,
    pub payment_transaction_id: String,
    pub payment_method: PaymentMethodInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitecvrpRecurringPaymentResponse {
    pub payment_id: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecurringPaymentStatus {
    Pending,
    Success,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitecvrpRecurringPaymentStatusResponse {
    pub status: RecurringPaymentStatus,
    pub payment_response: Option<CapitecvrpPaymentResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapitecvrpErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitecvrpMandateMetadata {
    pub flow_type: CapitecvrpFlowType,
    pub consent_receipt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapitecvrpFlowType {
    OnceOff,
    Recurring,
}

impl TryFrom<&SetupMandateRouterData> for CapitecvrpOnceOffConsentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &SetupMandateRouterData) -> Result<Self, Self::Error> {
        let payment_method_data = item.request.payment_method_data.clone();

        let capitec_data = match payment_method_data {
            PaymentMethodData::OpenBanking(
                hyperswitch_domain_models::payment_method_data::OpenBankingData::OpenBankingCapitec {
                    client_identifier,
                    minimum_amount,
                    maximum_amount,
                    product_description,
                    beneficiary_statement_description,
                    client_statement_description,
                    ..
                },
            ) => Ok(CapitecVrpData {
                client_identifier,
                minimum_amount,
                maximum_amount,
                product_description,
                beneficiary_statement_description,
                client_statement_description,
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Only OpenBankingCapitec payment method is supported for Capitec VRP".to_string(),
            )),
        }?;

        let identifier_key = parse_client_identifier_key(&capitec_data.client_identifier.identifier_key)?;
        let identifier_value = capitec_data.client_identifier.identifier_value.clone();

        let amount = item.request.minor_amount.get_amount_as_i64();

        let min_amount = capitec_data.minimum_amount.map(|a| a.get_amount_as_i64()).unwrap_or(amount);
        let max_amount = capitec_data.maximum_amount.map(|a| a.get_amount_as_i64()).unwrap_or(amount);

        let merchant_reference = get_merchant_reference(
            &item.additional_merchant_data,
            &item.connector_request_reference_id,
        );

        let payment_consent = OnceOffPaymentConsent {
            merchant_reference: merchant_reference.clone(),
            product_description: capitec_data.product_description.or(item.description.clone())
                .map(|desc| truncate_string(&desc, 20)),
            beneficiary_statement_description: truncate_string(
                capitec_data.beneficiary_statement_description.as_deref()
                    .unwrap_or(&merchant_reference),
                20,
            ),
            client_statement_description: truncate_string(
                capitec_data.client_statement_description.as_deref()
                    .unwrap_or(item.description.as_deref().unwrap_or("Payment")),
                20,
            ),
            minimum_amount: min_amount,
            maximum_amount: max_amount,
            tip_amount: None,
        };

        Ok(Self {
            client: CapitecvrpClient {
                identifier_key,
                identifier_value,
            },
            payment_consent,
            callback_url: item.request.router_return_url.clone(),
        })
    }
}

struct CapitecVrpData {
    client_identifier: hyperswitch_domain_models::payment_method_data::CapitecClientIdentifier,
    minimum_amount: Option<MinorUnit>,
    maximum_amount: Option<MinorUnit>,
    product_description: Option<String>,
    beneficiary_statement_description: Option<String>,
    client_statement_description: Option<String>,
}

fn parse_client_identifier_key(key: &str) -> Result<ClientIdentifierKey, error_stack::Report<errors::ConnectorError>> {
    match key.to_uppercase().as_str() {
        "CELLPHONE" => Ok(ClientIdentifierKey::Cellphone),
        "IDNUMBER" => Ok(ClientIdentifierKey::Idnumber),
        "ACCOUNTNUMBER" => Ok(ClientIdentifierKey::Accountnumber),
        "CAPITECPAY" => Ok(ClientIdentifierKey::Capitecpay),
        _ => Err(errors::ConnectorError::InvalidDataFormat {
            field_name: "identifier_key",
        }.into()),
    }
}

impl TryFrom<&SetupMandateRouterData> for CapitecvrpRecurringConsentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &SetupMandateRouterData) -> Result<Self, Self::Error> {
        let payment_method_data = item.request.payment_method_data.clone();

        let (capitec_data, recurrence_data) = match payment_method_data {
            PaymentMethodData::OpenBanking(
                hyperswitch_domain_models::payment_method_data::OpenBankingData::OpenBankingCapitec {
                    client_identifier,
                    minimum_amount,
                    maximum_amount,
                    product_description,
                    beneficiary_statement_description: _,
                    client_statement_description: _,
                    recurrence,
                },
            ) => Ok((
                CapitecVrpData {
                    client_identifier,
                    minimum_amount,
                    maximum_amount,
                    product_description,
                    beneficiary_statement_description: None,
                    client_statement_description: None,
                },
                recurrence,
            )),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Only OpenBankingCapitec payment method is supported for Capitec VRP".to_string(),
            )),
        }?;

        let identifier_key = parse_client_identifier_key(&capitec_data.client_identifier.identifier_key)?;
        let identifier_value = capitec_data.client_identifier.identifier_value.clone();

        let amount = item.request.minor_amount.get_amount_as_i64();
        let max_amount = capitec_data.maximum_amount.map(|a| a.get_amount_as_i64()).unwrap_or(amount);

        let recurrence = extract_recurrence(recurrence_data)?;

        let merchant_reference = get_merchant_reference(
            &item.additional_merchant_data,
            &item.connector_request_reference_id,
        );

        let payment_consent = RecurringPaymentConsent {
            merchant_reference,
            product_description: capitec_data.product_description.or(item.description.clone())
                .map(|desc| truncate_string(&desc, 20)),
            minimum_amount: capitec_data.minimum_amount.map(|a| a.get_amount_as_i64()).or(Some(1)),
            maximum_amount: max_amount,
            recurrence,
        };

        Ok(Self {
            client: CapitecvrpClient {
                identifier_key,
                identifier_value,
            },
            payment_consent,
            callback_url: item.request.router_return_url.clone(),
        })
    }
}

fn extract_recurrence(
    recurrence_data: Option<hyperswitch_domain_models::payment_method_data::CapitecRecurrence>,
) -> Result<Recurrence, error_stack::Report<errors::ConnectorError>> {
    // Use provided recurrence or default to monthly starting tomorrow
    let tomorrow = chrono::Utc::now() + chrono::Duration::days(1);
    let default_first_payment = tomorrow.format("%Y-%m-%d").to_string();

    match recurrence_data {
        Some(r) => {
            let interval = r.interval
                .as_deref()
                .map(parse_recurring_interval)
                .transpose()?
                .unwrap_or(RecurringInterval::Monthly);
            Ok(Recurrence {
                first_payment_date: r.first_payment_date.unwrap_or(default_first_payment),
                interval,
                occurrences: r.occurrences,
            })
        }
        None => Ok(Recurrence {
            first_payment_date: default_first_payment,
            interval: RecurringInterval::Monthly,
            occurrences: Some(0),
        }),
    }
}

fn parse_recurring_interval(interval: &str) -> Result<RecurringInterval, error_stack::Report<errors::ConnectorError>> {
    match interval.to_uppercase().as_str() {
        "DAILY" => Ok(RecurringInterval::Daily),
        "WEEKLY" => Ok(RecurringInterval::Weekly),
        "FORTNIGHTLY" => Ok(RecurringInterval::Fortnightly),
        "MONTHLY" => Ok(RecurringInterval::Monthly),
        "BIANNUALLY" => Ok(RecurringInterval::Biannually),
        "ANNUALLY" => Ok(RecurringInterval::Annually),
        _ => Err(errors::ConnectorError::InvalidDataFormat {
            field_name: "recurrence.interval",
        }.into()),
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        s.chars().take(max_len).collect()
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<F, CapitecvrpConsentResponse, SetupMandateRequestData, PaymentsResponseData>,
    > for RouterData<F, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CapitecvrpConsentResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let metadata = CapitecvrpMandateMetadata {
            flow_type: CapitecvrpFlowType::OnceOff, 
            consent_receipt: None,
        };

        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(
                    item.response.consent_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(Some(MandateReference {
                    connector_mandate_id: Some(item.response.consent_id),
                    payment_method_id: None,
                    mandate_metadata: Some(
                    Secret::new(
                        serde_json::to_value(&metadata)
                            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?
                    )
                ),
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
    }
}

impl TryFrom<&PaymentsAuthorizeRouterData> for CapitecvrpOnceOffPaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let consent_receipt = get_consent_receipt_from_metadata(item)?;
        let amount = item.request.minor_amount.get_amount_as_i64();

        Ok(Self {
            consent_receipt,
            beneficiary_statement_description: truncate_string(
                &item.connector_request_reference_id,
                20,
            ),
            client_statement_description: truncate_string(
                item.description.as_deref().unwrap_or("Payment"),
                20,
            ),
            amount,
        })
    }
}

impl TryFrom<&PaymentsAuthorizeRouterData> for CapitecvrpRecurringPaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let consent_receipt = get_consent_receipt_from_metadata(item)?;
        let amount = item.request.minor_amount.get_amount_as_i64();

        Ok(Self {
            consent_receipt,
            beneficiary_statement_description: truncate_string(
                &item.connector_request_reference_id,
                20,
            ),
            client_statement_description: truncate_string(
                item.description.as_deref().unwrap_or("Payment"),
                20,
            ),
            amount,
            callback_url: item.request.router_return_url.clone(),
        })
    }
}

fn get_consent_receipt_from_metadata(
    item: &PaymentsAuthorizeRouterData,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    item.connector_meta_data
        .as_ref()
        .and_then(|meta| meta.peek().as_object())
        .and_then(|obj| obj.get("consent_receipt"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| {
            errors::ConnectorError::MissingRequiredField {
                field_name: "consent_receipt",
            }
            .into()
        })
}

// Payment response transformers
impl<F>
    TryFrom<
        ResponseRouterData<F, CapitecvrpPaymentResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CapitecvrpPaymentResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Charged,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(
                    item.response.payment_transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: Some(
                    serde_json::json!({
                        "effective_date": item.response.effective_date,
                        "batch_settlement_id": item.response.batch_settlement_id,
                        "payment_method_type": item.response.payment_method.payment_type,
                    })
                ),
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.payment_transaction_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            CapitecvrpRecurringPaymentResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CapitecvrpRecurringPaymentResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Pending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(
                    item.response.payment_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.payment_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<F, CapitecvrpConsentStatusResponse, PaymentsSyncData, PaymentsResponseData>,
    > for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CapitecvrpConsentStatusResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.status.clone());

        let connector_metadata = if item.response.status == ConsentStatus::Approved {
            item.response.consent_receipt.as_ref().map(|receipt| {
                serde_json::json!({
                    "consent_receipt": receipt,
                })
            })
        } else {
            None
        };

        let connector_transaction_id = item
            .data
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .ok();

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(
                    connector_transaction_id.unwrap_or_default(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            CapitecvrpRecurringPaymentStatusResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CapitecvrpRecurringPaymentStatusResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = match item.response.status {
            RecurringPaymentStatus::Pending => enums::AttemptStatus::Pending,
            RecurringPaymentStatus::Success => enums::AttemptStatus::Charged,
        };

        let (connector_metadata, connector_response_reference_id) =
            if let Some(payment_response) = item.response.payment_response {
                (
                    Some(serde_json::json!({
                        "effective_date": payment_response.effective_date,
                        "batch_settlement_id": payment_response.batch_settlement_id,
                    })),
                    Some(payment_response.payment_transaction_id),
                )
            } else {
                (None, None)
            };

        let connector_transaction_id = item
            .data
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .ok();

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(
                    connector_transaction_id.unwrap_or_default(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, (), MandateRevokeRequestData, MandateRevokeResponseData>>
    for RouterData<F, MandateRevokeRequestData, MandateRevokeResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, (), MandateRevokeRequestData, MandateRevokeResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(MandateRevokeResponseData {
                mandate_status: common_enums::MandateStatus::Revoked,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct CapitecvrpRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&RefundsRouterData<F>> for CapitecvrpRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented(
            "Refunds are not supported for Capitec VRP".to_string(),
        )
        .into())
    }
}

#[derive(Debug, Deserialize)]
pub struct CapitecvrpRefundResponse {
    pub id: String,
    pub status: String,
}

impl TryFrom<RefundsResponseRouterData<Execute, CapitecvrpRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: RefundsResponseRouterData<Execute, CapitecvrpRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented(
            "Refunds are not supported for Capitec VRP".to_string(),
        )
        .into())
    }
}
