use common_utils::ext_traits::{Encode, ValueExt};
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, CardData},
    core::errors,
    pii::{self},
    types::{self, api, storage::enums},
    utils::OptionExt,
};

#[derive(Eq, PartialEq, Serialize, Clone, Debug)]
pub struct PayeezyCard {
    #[serde(rename = "type")]
    pub card_type: PayeezyCardType,
    pub cardholder_name: Secret<String>,
    pub card_number: Secret<String, pii::CardNumber>,
    pub exp_date: Secret<String>,
    pub cvv: Secret<String>,
}

#[derive(Serialize, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PayeezyCardType {
    #[serde(rename = "American Express")]
    AmericanExpress,
    Visa,
    Mastercard,
    Discover,
}

impl TryFrom<utils::CardIssuer> for PayeezyCardType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(issuer: utils::CardIssuer) -> Result<Self, Self::Error> {
        match issuer {
            utils::CardIssuer::AmericanExpress => Ok(Self::AmericanExpress),
            utils::CardIssuer::Master => Ok(Self::Mastercard),
            utils::CardIssuer::Discover => Ok(Self::Discover),
            utils::CardIssuer::Visa => Ok(Self::Visa),
            _ => Err(errors::ConnectorError::NotSupported {
                payment_method: api::enums::PaymentMethod::Card.to_string(),
                connector: "Payeezy",
                payment_experience: api::enums::PaymentExperience::RedirectToUrl.to_string(),
            }
            .into()),
        }
    }
}

#[derive(Serialize, Eq, PartialEq, Clone, Debug)]
#[serde(untagged)]
pub enum PayeezyPaymentMethod {
    PayeezyCard(PayeezyCard),
}

#[derive(Default, Debug, Serialize, Eq, PartialEq, Clone)]
pub enum PayeezyPaymentMethodType {
    #[default]
    #[serde(rename = "credit_card")]
    Card,
}

#[derive(Serialize, Eq, PartialEq, Clone, Debug)]
pub struct PayeezyPaymentsRequest {
    pub merchant_ref: String,
    pub transaction_type: PayeezyTransactionType,
    pub method: PayeezyPaymentMethodType,
    pub amount: i64,
    pub currency_code: String,
    pub credit_card: PayeezyPaymentMethod,
    pub stored_credentials: Option<StoredCredentials>,
}

#[derive(Serialize, Eq, PartialEq, Clone, Debug)]
pub struct StoredCredentials {
    pub sequence: Sequence,
    pub initiator: Initiator,
    pub is_scheduled: bool,
    pub cardbrand_original_transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Sequence {
    First,
    Subsequent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Initiator {
    Merchant,
    CardHolder,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PayeezyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.payment_method {
            storage_models::enums::PaymentMethod::Card => get_card_specific_payment_data(item),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

fn get_card_specific_payment_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<PayeezyPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let merchant_ref = format!("{}_{}_{}", item.merchant_id, item.payment_id, "1");
    let method = PayeezyPaymentMethodType::Card;
    let amount = item.request.amount;
    let currency_code = item.request.currency.to_string();
    let credit_card = get_payment_method_data(item)?;

    let mandate = item
        .request
        .mandate_id
        .clone()
        .and_then(|mandate_ids| mandate_ids.connector_mandate_id);

    let off_session = item
        .request
        .off_session
        .and_then(|value| mandate.as_ref().map(|_| value));

    let (transaction_type, stored_credentials) = match &item.request.setup_mandate_details {
        Some(_setup_mandate_details) => (
            PayeezyTransactionType::Recurring,
            Some(StoredCredentials {
                sequence: Sequence::First,
                initiator: Initiator::Merchant,
                is_scheduled: true,
                cardbrand_original_transaction_id: None,
            }),
        ),
        _ => match off_session {
            Some(true) => (
                PayeezyTransactionType::Recurring,
                Some(StoredCredentials {
                    sequence: Sequence::Subsequent,
                    initiator: Initiator::CardHolder,
                    is_scheduled: true,
                    cardbrand_original_transaction_id: mandate,
                }),
            ),
            _ => match item.request.capture_method {
                Some(storage_models::enums::CaptureMethod::Manual) => {
                    (PayeezyTransactionType::Authorize, None)
                }
                _ => (PayeezyTransactionType::Purchase, None),
            },
        },
    };

    Ok(PayeezyPaymentsRequest {
        merchant_ref,
        transaction_type,
        method,
        amount,
        currency_code,
        credit_card,
        stored_credentials,
    })
}

fn get_payment_method_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<PayeezyPaymentMethod, error_stack::Report<errors::ConnectorError>> {
    match item.request.payment_method_data {
        api::PaymentMethodData::Card(ref card) => {
            let card_type = PayeezyCardType::try_from(card.get_card_issuer()?)?;
            let payeezy_card = PayeezyCard {
                card_type,
                cardholder_name: card.card_holder_name.clone(),
                card_number: card.card_number.clone(),
                exp_date: card.get_card_expiry_as_mmyy(),
                cvv: card.card_cvc.clone(),
            };
            Ok(PayeezyPaymentMethod::PayeezyCard(payeezy_card))
        }
        _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
    }
}

// Auth Struct
pub struct PayeezyAuthType {
    pub(super) api_key: String,
    pub(super) api_secret: String,
    pub(super) merchant_token: String,
}

impl TryFrom<&types::ConnectorAuthType> for PayeezyAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = item
        {
            Ok(Self {
                api_key: api_key.to_string(),
                api_secret: api_secret.to_string(),
                merchant_token: key1.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PayeezyPaymentStatus {
    Approved,
    Declined,
    #[default]
    #[serde(rename = "Not Processed")]
    NotProcessed,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayeezyPaymentsResponse {
    correlation_id: String,
    transaction_status: PayeezyPaymentStatus,
    validation_status: String,
    transaction_type: PayeezyTransactionType,
    transaction_id: String,
    transaction_tag: Option<String>,
    method: Option<String>,
    amount: String,
    currency: String,
    bank_resp_code: String,
    bank_message: String,
    gateway_resp_code: String,
    gateway_message: String,
    stored_credentials: Option<PaymentsStoredCredentials>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentsStoredCredentials {
    cardbrand_original_transaction_id: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayeezyCaptureOrVoidRequest {
    transaction_tag: String,
    transaction_type: PayeezyTransactionType,
    amount: String,
    currency_code: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for PayeezyCaptureOrVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let payment_details = item
            .request
            .connector_meta
            .as_ref()
            .get_required_value("connector_meta")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_meta",
            })?
            .clone();
        let metadata: PayeezyPaymentsMetadata = payment_details
            .parse_value("PayeezyPaymentsMetadata")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "transaction_tag",
            })?;
        Ok(Self {
            transaction_type: PayeezyTransactionType::Capture,
            amount: item.request.amount.to_string(),
            currency_code: item.request.currency.to_string(),
            transaction_tag: metadata.transaction_tag,
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for PayeezyCaptureOrVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let payment_details = item
            .request
            .connector_meta
            .as_ref()
            .get_required_value("connector_meta")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_meta",
            })?
            .clone();
        let metadata: PayeezyPaymentsMetadata = payment_details
            .parse_value("PayeezyPaymentsMetadata")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "transaction_tag",
            })?;
        Ok(Self {
            transaction_type: PayeezyTransactionType::Void,
            amount: item.request.amount.unwrap_or_default().to_string(),
            currency_code: item.request.currency.unwrap_or_default().to_string(),
            transaction_tag: metadata.transaction_tag,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PayeezyTransactionType {
    Authorize,
    Capture,
    Purchase,
    Recurring,
    Void,
    Refund,
    #[default]
    Pending,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayeezyPaymentsMetadata {
    transaction_tag: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PayeezyPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PayeezyPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let metadata = item
            .response
            .transaction_tag
            .map(|txn_tag| {
                Encode::<'_, PayeezyPaymentsMetadata>::encode_to_value(
                    &construct_payeezy_payments_metadata(txn_tag),
                )
            })
            .transpose()
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_meta",
            })?;

        let mandate_reference = item
            .response
            .stored_credentials
            .map(|credentials| credentials.cardbrand_original_transaction_id);

        let status = match item.response.transaction_status {
            PayeezyPaymentStatus::Approved => match item.response.transaction_type {
                PayeezyTransactionType::Authorize => enums::AttemptStatus::Authorized,
                PayeezyTransactionType::Capture => enums::AttemptStatus::Charged,
                PayeezyTransactionType::Purchase => enums::AttemptStatus::Charged,
                PayeezyTransactionType::Recurring => enums::AttemptStatus::Charged,
                PayeezyTransactionType::Void => enums::AttemptStatus::Voided,
                _ => enums::AttemptStatus::Pending,
            },
            PayeezyPaymentStatus::Declined | PayeezyPaymentStatus::NotProcessed => {
                match item.response.transaction_type {
                    PayeezyTransactionType::Authorize => enums::AttemptStatus::AuthorizationFailed,
                    PayeezyTransactionType::Capture => enums::AttemptStatus::CaptureFailed,
                    PayeezyTransactionType::Purchase => enums::AttemptStatus::AuthorizationFailed,
                    PayeezyTransactionType::Recurring => enums::AttemptStatus::AuthorizationFailed,
                    PayeezyTransactionType::Void => enums::AttemptStatus::VoidFailed,
                    _ => enums::AttemptStatus::Pending,
                }
            }
        };

        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id,
                ),
                redirection_data: None,
                mandate_reference,
                connector_metadata: metadata,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayeezyRefundRequest {
    transaction_tag: String,
    transaction_type: PayeezyTransactionType,
    amount: String,
    currency_code: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for PayeezyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let payment_details = item
            .request
            .connector_metadata
            .as_ref()
            .get_required_value("connector_metadata")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_metadata",
            })?
            .clone();
        let metadata: PayeezyPaymentsMetadata = payment_details
            .parse_value("PayeezyPaymentsMetadata")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "transaction_tag",
            })?;
        Ok(Self {
            transaction_type: PayeezyTransactionType::Refund,
            amount: item.request.refund_amount.to_string(),
            currency_code: item.request.currency.to_string(),
            transaction_tag: metadata.transaction_tag,
        })
    }
}

// Type definition for Refund Response

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RefundStatus {
    Approved,
    Declined,
    #[default]
    #[serde(rename = "Not Processed")]
    NotProcessed,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Approved => Self::Success,
            RefundStatus::Declined => Self::Failure,
            RefundStatus::NotProcessed => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefundResponse {
    correlation_id: String,
    transaction_status: RefundStatus,
    validation_status: String,
    transaction_type: String,
    transaction_id: String,
    transaction_tag: Option<String>,
    method: Option<String>,
    amount: String,
    currency: String,
    bank_resp_code: String,
    bank_message: String,
    gateway_resp_code: String,
    gateway_message: String,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status: enums::RefundStatus::from(item.response.transaction_status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PayeezyError {
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PayeezyErrorResponse {
    pub transaction_status: String,
    #[serde(rename = "Error")]
    pub error: PayeezyError,
}

fn construct_payeezy_payments_metadata(transaction_tag: String) -> PayeezyPaymentsMetadata {
    PayeezyPaymentsMetadata { transaction_tag }
}
