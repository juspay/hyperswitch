use cards::CardNumber;
use common_enums::{enums, AttemptStatus, CaptureMethod, Currency, PaymentMethod};
use common_utils::{errors::ParsingError, ext_traits::Encode};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::Execute,
    router_request_types::ResponseId,
    router_response_types::{MandateReference, PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::{api::CurrencyUnit, errors::ConnectorError};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        get_amount_as_string, get_unimplemented_payment_method_error_message, to_connector_meta,
        CardData, CardIssuer, RouterData as _,
    },
};

#[derive(Debug, Serialize)]
pub struct PayeezyRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T> TryFrom<(&CurrencyUnit, Currency, i64, T)> for PayeezyRouterData<T> {
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(
        (currency_unit, currency, amount, router_data): (&CurrencyUnit, Currency, i64, T),
    ) -> Result<Self, Self::Error> {
        let amount = get_amount_as_string(currency_unit, amount, currency)?;
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

impl TryFrom<CardIssuer> for PayeezyCardType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(issuer: CardIssuer) -> Result<Self, Self::Error> {
        match issuer {
            CardIssuer::AmericanExpress => Ok(Self::AmericanExpress),
            CardIssuer::Master => Ok(Self::Mastercard),
            CardIssuer::Discover => Ok(Self::Discover),
            CardIssuer::Visa => Ok(Self::Visa),

            CardIssuer::Maestro
            | CardIssuer::DinersClub
            | CardIssuer::JCB
            | CardIssuer::CarteBlanche => Err(ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Payeezy"),
            ))?,
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
    pub cardbrand_original_transaction_id: Option<Secret<String>>,
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

impl TryFrom<&PayeezyRouterData<&PaymentsAuthorizeRouterData>> for PayeezyPaymentsRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &PayeezyRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.payment_method {
            PaymentMethod::Card => get_card_specific_payment_data(item),

            PaymentMethod::CardRedirect
            | PaymentMethod::PayLater
            | PaymentMethod::Wallet
            | PaymentMethod::BankRedirect
            | PaymentMethod::BankTransfer
            | PaymentMethod::Crypto
            | PaymentMethod::BankDebit
            | PaymentMethod::Reward
            | PaymentMethod::RealTimePayment
            | PaymentMethod::MobilePayment
            | PaymentMethod::Upi
            | PaymentMethod::Voucher
            | PaymentMethod::OpenBanking
            | PaymentMethod::GiftCard => {
                Err(ConnectorError::NotImplemented("Payment methods".to_string()).into())
            }
        }
    }
}

fn get_card_specific_payment_data(
    item: &PayeezyRouterData<&PaymentsAuthorizeRouterData>,
) -> Result<PayeezyPaymentsRequest, error_stack::Report<ConnectorError>> {
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
    item: &PaymentsAuthorizeRouterData,
) -> Result<(PayeezyTransactionType, Option<StoredCredentials>), error_stack::Report<ConnectorError>>
{
    let connector_mandate_id = item.request.mandate_id.as_ref().and_then(|mandate_ids| {
        match mandate_ids.mandate_reference_id.clone() {
            Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                connector_mandate_ids,
            )) => connector_mandate_ids.get_connector_mandate_id(),
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
                    cardbrand_original_transaction_id: connector_mandate_id.map(Secret::new),
                }),
            )
        } else {
            match item.request.capture_method {
                Some(CaptureMethod::Manual) => Ok((PayeezyTransactionType::Authorize, None)),
                Some(CaptureMethod::SequentialAutomatic) | Some(CaptureMethod::Automatic) => {
                    Ok((PayeezyTransactionType::Purchase, None))
                }

                Some(CaptureMethod::ManualMultiple) | Some(CaptureMethod::Scheduled) | None => {
                    Err(ConnectorError::FlowNotSupported {
                        flow: item.request.capture_method.unwrap_or_default().to_string(),
                        connector: "Payeezy".to_string(),
                    })
                }
            }?
        };
    Ok((transaction_type, stored_credentials))
}

fn is_mandate_payment(
    item: &PaymentsAuthorizeRouterData,
    connector_mandate_id: Option<&String>,
) -> bool {
    item.request.setup_mandate_details.is_some() || connector_mandate_id.is_some()
}

fn get_payment_method_data(
    item: &PayeezyRouterData<&PaymentsAuthorizeRouterData>,
) -> Result<PayeezyPaymentMethod, error_stack::Report<ConnectorError>> {
    match item.router_data.request.payment_method_data {
        PaymentMethodData::Card(ref card) => {
            let card_type = PayeezyCardType::try_from(card.get_card_issuer()?)?;
            let payeezy_card = PayeezyCard {
                card_type,
                cardholder_name: item
                    .router_data
                    .get_optional_billing_full_name()
                    .unwrap_or(Secret::new("".to_string())),
                card_number: card.card_number.clone(),
                exp_date: card.get_card_expiry_month_year_2_digit_with_delimiter("".to_string())?,
                cvv: card.card_cvc.clone(),
            };
            Ok(PayeezyPaymentMethod::PayeezyCard(payeezy_card))
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
            Err(ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("Payeezy"),
            ))?
        }
    }
}

// Auth Struct
pub struct PayeezyAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_secret: Secret<String>,
    pub(super) merchant_token: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PayeezyAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
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
            Err(ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PayeezyPaymentStatus {
    Approved,
    Declined,
    #[default]
    #[serde(rename = "Not Processed")]
    NotProcessed,
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct PaymentsStoredCredentials {
    cardbrand_original_transaction_id: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct PayeezyCaptureOrVoidRequest {
    transaction_tag: String,
    transaction_type: PayeezyTransactionType,
    amount: String,
    currency_code: String,
}

impl TryFrom<&PayeezyRouterData<&PaymentsCaptureRouterData>> for PayeezyCaptureOrVoidRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &PayeezyRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let metadata: PayeezyPaymentsMetadata =
            to_connector_meta(item.router_data.request.connector_meta.clone())
                .change_context(ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            transaction_type: PayeezyTransactionType::Capture,
            amount: item.amount.clone(),
            currency_code: item.router_data.request.currency.to_string(),
            transaction_tag: metadata.transaction_tag,
        })
    }
}

impl TryFrom<&PaymentsCancelRouterData> for PayeezyCaptureOrVoidRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let metadata: PayeezyPaymentsMetadata =
            to_connector_meta(item.request.connector_meta.clone())
                .change_context(ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            transaction_type: PayeezyTransactionType::Void,
            amount: item
                .request
                .amount
                .ok_or(ConnectorError::RequestEncodingFailed)?
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

impl<F, T> TryFrom<ResponseRouterData<F, PayeezyPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PayeezyPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let metadata = item
            .response
            .transaction_tag
            .map(|txn_tag| construct_payeezy_payments_metadata(txn_tag).encode_to_value())
            .transpose()
            .change_context(ConnectorError::ResponseHandlingFailed)?;

        let mandate_reference = item
            .response
            .stored_credentials
            .map(|credentials| credentials.cardbrand_original_transaction_id)
            .map(|id| MandateReference {
                connector_mandate_id: Some(id.expose()),
                payment_method_id: None,
                mandate_metadata: None,
                connector_mandate_request_reference_id: None,
            });
        let status = get_status(
            item.response.transaction_status,
            item.response.transaction_type,
        );

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata: metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    item.response
                        .reference
                        .unwrap_or(item.response.transaction_id),
                ),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

fn get_status(status: PayeezyPaymentStatus, method: PayeezyTransactionType) -> AttemptStatus {
    match status {
        PayeezyPaymentStatus::Approved => match method {
            PayeezyTransactionType::Authorize => AttemptStatus::Authorized,
            PayeezyTransactionType::Capture
            | PayeezyTransactionType::Purchase
            | PayeezyTransactionType::Recurring => AttemptStatus::Charged,
            PayeezyTransactionType::Void => AttemptStatus::Voided,
            PayeezyTransactionType::Refund | PayeezyTransactionType::Pending => {
                AttemptStatus::Pending
            }
        },
        PayeezyPaymentStatus::Declined | PayeezyPaymentStatus::NotProcessed => match method {
            PayeezyTransactionType::Capture => AttemptStatus::CaptureFailed,
            PayeezyTransactionType::Authorize
            | PayeezyTransactionType::Purchase
            | PayeezyTransactionType::Recurring => AttemptStatus::AuthorizationFailed,
            PayeezyTransactionType::Void => AttemptStatus::VoidFailed,
            PayeezyTransactionType::Refund | PayeezyTransactionType::Pending => {
                AttemptStatus::Pending
            }
        },
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

impl<F> TryFrom<&PayeezyRouterData<&RefundsRouterData<F>>> for PayeezyRefundRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(item: &PayeezyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let metadata: PayeezyPaymentsMetadata =
            to_connector_meta(item.router_data.request.connector_metadata.clone())
                .change_context(ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            transaction_type: PayeezyTransactionType::Refund,
            amount: item.amount.clone(),
            currency_code: item.router_data.request.currency.to_string(),
            transaction_tag: metadata.transaction_tag,
        })
    }
}

// Type definition for Refund Response

#[derive(Debug, Deserialize, Default, Serialize)]
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

#[derive(Deserialize, Debug, Serialize)]
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

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<ParsingError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status: enums::RefundStatus::from(item.response.transaction_status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PayeezyError {
    pub messages: Vec<Message>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PayeezyErrorResponse {
    pub transaction_status: String,
    #[serde(rename = "Error")]
    pub error: PayeezyError,
}

fn construct_payeezy_payments_metadata(transaction_tag: String) -> PayeezyPaymentsMetadata {
    PayeezyPaymentsMetadata { transaction_tag }
}
