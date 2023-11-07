use cards::CardNumber;
use common_utils::ext_traits::Encode;
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, CardData},
    core::errors,
    types::{self, api, storage::enums, transformers::ForeignFrom},
};
#[derive(Debug, Serialize)]
pub struct PayeezyRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for PayeezyRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (currency_unit, currency, amount, router_data): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data,
        })
    }
}

#[derive(Serialize, Debug)]
pub struct PayeezyCard {
    #[serde(rename = "type")]
    pub card_type: PayeezyCardType,
    pub cardholder_name: Secret<String>,
    pub card_number: CardNumber,
    pub exp_date: Secret<String>,
    pub cvv: Secret<String>,
}

#[derive(Serialize, Debug)]
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

            utils::CardIssuer::Maestro | utils::CardIssuer::DinersClub | utils::CardIssuer::JCB => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Payeezy",
                }
                .into())
            }
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum PayeezyPaymentMethod {
    PayeezyCard(PayeezyCard),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayeezyPaymentMethodType {
    CreditCard,
}

#[derive(Serialize, Debug)]
pub struct PayeezyPaymentsRequest {
    pub merchant_ref: String,
    pub transaction_type: PayeezyTransactionType,
    pub method: PayeezyPaymentMethodType,
    pub amount: String,
    pub currency_code: String,
    pub credit_card: PayeezyPaymentMethod,
    pub stored_credentials: Option<StoredCredentials>,
    pub reference: String,
}

#[derive(Serialize, Debug)]
pub struct StoredCredentials {
    pub sequence: Sequence,
    pub initiator: Initiator,
    pub is_scheduled: bool,
    pub cardbrand_original_transaction_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Sequence {
    First,
    Subsequent,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Initiator {
    Merchant,
    CardHolder,
}

impl TryFrom<&PayeezyRouterData<&types::PaymentsAuthorizeRouterData>> for PayeezyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayeezyRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.payment_method {
            diesel_models::enums::PaymentMethod::Card => get_card_specific_payment_data(item),

            diesel_models::enums::PaymentMethod::CardRedirect
            | diesel_models::enums::PaymentMethod::PayLater
            | diesel_models::enums::PaymentMethod::Wallet
            | diesel_models::enums::PaymentMethod::BankRedirect
            | diesel_models::enums::PaymentMethod::BankTransfer
            | diesel_models::enums::PaymentMethod::Crypto
            | diesel_models::enums::PaymentMethod::BankDebit
            | diesel_models::enums::PaymentMethod::Reward
            | diesel_models::enums::PaymentMethod::Upi
            | diesel_models::enums::PaymentMethod::Voucher
            | diesel_models::enums::PaymentMethod::GiftCard => {
                Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into())
            }
        }
    }
}

fn get_card_specific_payment_data(
    item: &PayeezyRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<PayeezyPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let merchant_ref = item.router_data.attempt_id.to_string();
    let method = PayeezyPaymentMethodType::CreditCard;
    let amount = item.amount.clone();
    let currency_code = item.router_data.request.currency.to_string();
    let credit_card = get_payment_method_data(item)?;
    let (transaction_type, stored_credentials) =
        get_transaction_type_and_stored_creds(item.router_data)?;
    Ok(PayeezyPaymentsRequest {
        merchant_ref,
        transaction_type,
        method,
        amount,
        currency_code,
        credit_card,
        stored_credentials,
        reference: item.router_data.connector_request_reference_id.clone(),
    })
}
fn get_transaction_type_and_stored_creds(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<
    (PayeezyTransactionType, Option<StoredCredentials>),
    error_stack::Report<errors::ConnectorError>,
> {
    let connector_mandate_id = item.request.mandate_id.as_ref().and_then(|mandate_ids| {
        match mandate_ids.mandate_reference_id.clone() {
            Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                connector_mandate_ids,
            )) => connector_mandate_ids.connector_mandate_id,
            _ => None,
        }
    });
    let (transaction_type, stored_credentials) =
        if is_mandate_payment(item, connector_mandate_id.as_ref()) {
            // Mandate payment
            (
                PayeezyTransactionType::Recurring,
                Some(StoredCredentials {
                    // connector_mandate_id is not present then it is a First payment, else it is a Subsequent mandate payment
                    sequence: match connector_mandate_id.is_some() {
                        true => Sequence::Subsequent,
                        false => Sequence::First,
                    },
                    // off_session true denotes the customer not present during the checkout process. In other cases customer present at the checkout.
                    initiator: match item.request.off_session {
                        Some(true) => Initiator::Merchant,
                        _ => Initiator::CardHolder,
                    },
                    is_scheduled: true,
                    // In case of first mandate payment connector_mandate_id would be None, otherwise holds some value
                    cardbrand_original_transaction_id: connector_mandate_id,
                }),
            )
        } else {
            match item.request.capture_method {
                Some(diesel_models::enums::CaptureMethod::Manual) => {
                    Ok((PayeezyTransactionType::Authorize, None))
                }
                Some(diesel_models::enums::CaptureMethod::Automatic) => {
                    Ok((PayeezyTransactionType::Purchase, None))
                }

                Some(diesel_models::enums::CaptureMethod::ManualMultiple)
                | Some(diesel_models::enums::CaptureMethod::Scheduled)
                | None => Err(errors::ConnectorError::FlowNotSupported {
                    flow: item.request.capture_method.unwrap_or_default().to_string(),
                    connector: "Payeezy".to_string(),
                }),
            }?
        };
    Ok((transaction_type, stored_credentials))
}

fn is_mandate_payment(
    item: &types::PaymentsAuthorizeRouterData,
    connector_mandate_id: Option<&String>,
) -> bool {
    item.request.setup_mandate_details.is_some() || connector_mandate_id.is_some()
}

fn get_payment_method_data(
    item: &PayeezyRouterData<&types::PaymentsAuthorizeRouterData>,
) -> Result<PayeezyPaymentMethod, error_stack::Report<errors::ConnectorError>> {
    match item.router_data.request.payment_method_data {
        api::PaymentMethodData::Card(ref card) => {
            let card_type = PayeezyCardType::try_from(card.get_card_issuer()?)?;
            let payeezy_card = PayeezyCard {
                card_type,
                cardholder_name: card.card_holder_name.clone(),
                card_number: card.card_number.clone(),
                exp_date: card.get_card_expiry_month_year_2_digit_with_delimiter("".to_string()),
                cvv: card.card_cvc.clone(),
            };
            Ok(PayeezyPaymentMethod::PayeezyCard(payeezy_card))
        }

        api::PaymentMethodData::CardRedirect(_)
        | api::PaymentMethodData::Wallet(_)
        | api::PaymentMethodData::PayLater(_)
        | api::PaymentMethodData::BankRedirect(_)
        | api::PaymentMethodData::BankDebit(_)
        | api::PaymentMethodData::BankTransfer(_)
        | api::PaymentMethodData::Crypto(_)
        | api::PaymentMethodData::MandatePayment
        | api::PaymentMethodData::Reward
        | api::PaymentMethodData::Upi(_)
        | api::PaymentMethodData::Voucher(_)
        | api::PaymentMethodData::GiftCard(_) => Err(errors::ConnectorError::NotSupported {
            message: utils::SELECTED_PAYMENT_METHOD.to_string(),
            connector: "Payeezy",
        }
        .into()),
    }
}

// Auth Struct
pub struct PayeezyAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_secret: Secret<String>,
    pub(super) merchant_token: Secret<String>,
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
                api_key: api_key.to_owned(),
                api_secret: api_secret.to_owned(),
                merchant_token: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PayeezyPaymentStatus {
    Approved,
    Declined,
    #[default]
    #[serde(rename = "Not Processed")]
    NotProcessed,
}

#[derive(Deserialize)]
pub struct PayeezyPaymentsResponse {
    pub correlation_id: String,
    pub transaction_status: PayeezyPaymentStatus,
    pub validation_status: String,
    pub transaction_type: PayeezyTransactionType,
    pub transaction_id: String,
    pub transaction_tag: Option<String>,
    pub method: Option<String>,
    pub amount: String,
    pub currency: String,
    pub bank_resp_code: String,
    pub bank_message: String,
    pub gateway_resp_code: String,
    pub gateway_message: String,
    pub stored_credentials: Option<PaymentsStoredCredentials>,
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentsStoredCredentials {
    cardbrand_original_transaction_id: String,
}

#[derive(Debug, Serialize)]
pub struct PayeezyCaptureOrVoidRequest {
    transaction_tag: String,
    transaction_type: PayeezyTransactionType,
    amount: String,
    currency_code: String,
}

impl TryFrom<&PayeezyRouterData<&types::PaymentsCaptureRouterData>>
    for PayeezyCaptureOrVoidRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayeezyRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let metadata: PayeezyPaymentsMetadata =
            utils::to_connector_meta(item.router_data.request.connector_meta.clone())
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            transaction_type: PayeezyTransactionType::Capture,
            amount: item.amount.clone(),
            currency_code: item.router_data.request.currency.to_string(),
            transaction_tag: metadata.transaction_tag,
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for PayeezyCaptureOrVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let metadata: PayeezyPaymentsMetadata =
            utils::to_connector_meta(item.request.connector_meta.clone())
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            transaction_type: PayeezyTransactionType::Void,
            amount: item
                .request
                .amount
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                .to_string(),
            currency_code: item.request.currency.unwrap_or_default().to_string(),
            transaction_tag: metadata.transaction_tag,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
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

#[derive(Debug, Serialize, Deserialize)]
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
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        let mandate_reference = item
            .response
            .stored_credentials
            .map(|credentials| credentials.cardbrand_original_transaction_id)
            .map(|id| types::MandateReference {
                connector_mandate_id: Some(id),
                payment_method_id: None,
            });
        let status = enums::AttemptStatus::foreign_from((
            item.response.transaction_status,
            item.response.transaction_type,
        ));

        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: None,
                mandate_reference,
                connector_metadata: metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    item.response
                        .reference
                        .unwrap_or(item.response.transaction_id),
                ),
            }),
            ..item.data
        })
    }
}

impl ForeignFrom<(PayeezyPaymentStatus, PayeezyTransactionType)> for enums::AttemptStatus {
    fn foreign_from((status, method): (PayeezyPaymentStatus, PayeezyTransactionType)) -> Self {
        match status {
            PayeezyPaymentStatus::Approved => match method {
                PayeezyTransactionType::Authorize => Self::Authorized,
                PayeezyTransactionType::Capture
                | PayeezyTransactionType::Purchase
                | PayeezyTransactionType::Recurring => Self::Charged,
                PayeezyTransactionType::Void => Self::Voided,
                PayeezyTransactionType::Refund | PayeezyTransactionType::Pending => Self::Pending,
            },
            PayeezyPaymentStatus::Declined | PayeezyPaymentStatus::NotProcessed => match method {
                PayeezyTransactionType::Capture => Self::CaptureFailed,
                PayeezyTransactionType::Authorize
                | PayeezyTransactionType::Purchase
                | PayeezyTransactionType::Recurring => Self::AuthorizationFailed,
                PayeezyTransactionType::Void => Self::VoidFailed,
                PayeezyTransactionType::Refund | PayeezyTransactionType::Pending => Self::Pending,
            },
        }
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct PayeezyRefundRequest {
    transaction_tag: String,
    transaction_type: PayeezyTransactionType,
    amount: String,
    currency_code: String,
}

impl<F> TryFrom<&PayeezyRouterData<&types::RefundsRouterData<F>>> for PayeezyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayeezyRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let metadata: PayeezyPaymentsMetadata =
            utils::to_connector_meta(item.router_data.request.connector_metadata.clone())
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            transaction_type: PayeezyTransactionType::Refund,
            amount: item.amount.clone(),
            currency_code: item.router_data.request.currency.to_string(),
            transaction_tag: metadata.transaction_tag,
        })
    }
}

// Type definition for Refund Response

#[derive(Debug, Deserialize, Default)]
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

#[derive(Deserialize)]
pub struct RefundResponse {
    pub correlation_id: String,
    pub transaction_status: RefundStatus,
    pub validation_status: String,
    pub transaction_type: String,
    pub transaction_id: String,
    pub transaction_tag: Option<String>,
    pub method: Option<String>,
    pub amount: String,
    pub currency: String,
    pub bank_resp_code: String,
    pub bank_message: String,
    pub gateway_resp_code: String,
    pub gateway_message: String,
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

#[derive(Debug, Deserialize)]
pub struct Message {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct PayeezyError {
    pub messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
pub struct PayeezyErrorResponse {
    pub transaction_status: String,
    #[serde(rename = "Error")]
    pub error: PayeezyError,
}

fn construct_payeezy_payments_metadata(transaction_tag: String) -> PayeezyPaymentsMetadata {
    PayeezyPaymentsMetadata { transaction_tag }
}
