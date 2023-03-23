use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PaymentsCancelRequestData, PaymentsSyncRequestData, RouterData},
    core::errors,
    pii::{self, Secret},
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FiservPaymentsRequest {
    amount: Amount,
    source: Source,
    transaction_details: TransactionDetails,
    merchant_details: MerchantDetails,
    transaction_interaction: TransactionInteraction,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(tag = "sourceType")]
pub enum Source {
    PaymentCard {
        card: CardData,
    },
    #[allow(dead_code)]
    GooglePay {
        data: String,
        signature: String,
        version: String,
    },
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    card_data: Secret<String, pii::CardNumber>,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    security_code: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayToken {
    signature: String,
    signed_message: String,
    protocol_version: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct Amount {
    #[serde(serialize_with = "utils::str_to_f32")]
    total: String,
    currency: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    capture_flag: Option<bool>,
    reversal_reason_code: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantDetails {
    merchant_id: String,
    terminal_id: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInteraction {
    origin: TransactionInteractionOrigin,
    eci_indicator: TransactionInteractionEciIndicator,
    pos_condition_code: TransactionInteractionPosConditionCode,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TransactionInteractionOrigin {
    #[default]
    Ecom,
}
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionInteractionEciIndicator {
    #[default]
    ChannelEncrypted,
}
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionInteractionPosConditionCode {
    #[default]
    CardNotPresentEcom,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FiservPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
        let amount = Amount {
            total: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            currency: item.request.currency.to_string(),
        };
        let transaction_details = TransactionDetails {
            capture_flag: Some(matches!(
                item.request.capture_method,
                Some(enums::CaptureMethod::Automatic) | None
            )),
            reversal_reason_code: None,
        };
        let metadata = item.get_connector_meta()?;
        let session: SessionObject = metadata
            .parse_value("SessionObject")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let merchant_details = MerchantDetails {
            merchant_id: auth.merchant_account,
            terminal_id: Some(session.terminal_id),
        };

        let transaction_interaction = TransactionInteraction {
            //Payment is being made in online mode, card not present
            origin: TransactionInteractionOrigin::Ecom,
            // transaction encryption such as SSL/TLS, but authentication was not performed
            eci_indicator: TransactionInteractionEciIndicator::ChannelEncrypted,
            //card not present in online transaction
            pos_condition_code: TransactionInteractionPosConditionCode::CardNotPresentEcom,
        };
        let source = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ref ccard) => {
                let card = CardData {
                    card_data: ccard
                        .card_number
                        .clone()
                        .map(|card| card.split_whitespace().collect()),
                    expiration_month: ccard.card_exp_month.clone(),
                    expiration_year: ccard.card_exp_year.clone(),
                    security_code: ccard.card_cvc.clone(),
                };
                Source::PaymentCard { card }
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Methods".to_string(),
            ))?,
        };
        Ok(Self {
            amount,
            source,
            transaction_details,
            merchant_details,
            transaction_interaction,
        })
    }
}

pub struct FiservAuthType {
    pub(super) api_key: String,
    pub(super) merchant_account: String,
    pub(super) api_secret: String,
}

impl TryFrom<&types::ConnectorAuthType> for FiservAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_string(),
                merchant_account: key1.to_string(),
                api_secret: api_secret.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FiservCancelRequest {
    transaction_details: TransactionDetails,
    merchant_details: MerchantDetails,
    reference_transaction_details: ReferenceTransactionDetails,
}

impl TryFrom<&types::PaymentsCancelRouterData> for FiservCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
        let metadata = item.get_connector_meta()?;
        let session: SessionObject = metadata
            .parse_value("SessionObject")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: Some(session.terminal_id),
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item.request.connector_transaction_id.to_string(),
            },
            transaction_details: TransactionDetails {
                capture_flag: None,
                reversal_reason_code: Some(item.request.get_cancellation_reason()?),
            },
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub details: Option<Vec<ErrorDetails>>,
    pub error: Option<Vec<ErrorDetails>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails {
    #[serde(rename = "type")]
    pub error_type: String,
    pub code: Option<String>,
    pub message: String,
    pub field: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum FiservPaymentStatus {
    Succeeded,
    Failed,
    Captured,
    Declined,
    Voided,
    Authorized,
    #[default]
    Processing,
}

impl From<FiservPaymentStatus> for enums::AttemptStatus {
    fn from(item: FiservPaymentStatus) -> Self {
        match item {
            FiservPaymentStatus::Captured | FiservPaymentStatus::Succeeded => Self::Charged,
            FiservPaymentStatus::Declined | FiservPaymentStatus::Failed => Self::Failure,
            FiservPaymentStatus::Processing => Self::Authorizing,
            FiservPaymentStatus::Voided => Self::Voided,
            FiservPaymentStatus::Authorized => Self::Authorized,
        }
    }
}

impl From<FiservPaymentStatus> for enums::RefundStatus {
    fn from(item: FiservPaymentStatus) -> Self {
        match item {
            FiservPaymentStatus::Succeeded
            | FiservPaymentStatus::Authorized
            | FiservPaymentStatus::Captured => Self::Success,
            FiservPaymentStatus::Declined | FiservPaymentStatus::Failed => Self::Failure,
            _ => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FiservPaymentsResponse {
    gateway_response: GatewayResponse,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(transparent)]
pub struct FiservSyncResponse {
    sync_responses: Vec<FiservPaymentsResponse>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GatewayResponse {
    gateway_transaction_id: Option<String>,
    transaction_state: FiservPaymentStatus,
    transaction_processing_details: TransactionProcessingDetails,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionProcessingDetails {
    order_id: String,
    transaction_id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, FiservPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, FiservPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let gateway_resp = item.response.gateway_response;

        Ok(Self {
            status: enums::AttemptStatus::from(gateway_resp.transaction_state),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    gateway_resp.transaction_processing_details.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, FiservSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, FiservSyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let gateway_resp = match item.response.sync_responses.first() {
            Some(gateway_response) => gateway_response,
            _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        };

        Ok(Self {
            status: enums::AttemptStatus::from(
                gateway_resp.gateway_response.transaction_state.clone(),
            ),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    gateway_resp
                        .gateway_response
                        .transaction_processing_details
                        .transaction_id
                        .clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FiservCaptureRequest {
    amount: Amount,
    transaction_details: TransactionDetails,
    merchant_details: MerchantDetails,
    reference_transaction_details: ReferenceTransactionDetails,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceTransactionDetails {
    reference_transaction_id: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionObject {
    pub terminal_id: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for FiservCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
        let metadata = item
            .connector_meta_data
            .clone()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let session: SessionObject = metadata
            .parse_value("SessionObject")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let amount = match item.request.amount_to_capture {
            Some(a) => utils::to_currency_base_unit(a, item.request.currency)?,
            _ => utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
        };
        Ok(Self {
            amount: Amount {
                total: amount,
                currency: item.request.currency.to_string(),
            },
            transaction_details: TransactionDetails {
                capture_flag: Some(true),
                reversal_reason_code: None,
            },
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: Some(session.terminal_id),
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item.request.connector_transaction_id.to_string(),
            },
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FiservSyncRequest {
    merchant_details: MerchantDetails,
    reference_transaction_details: ReferenceTransactionDetails,
}

impl TryFrom<&types::PaymentsSyncRouterData> for FiservSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: None,
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item
                    .request
                    .get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
            },
        })
    }
}

impl TryFrom<&types::RefundSyncRouterData> for FiservSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: None,
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item
                    .request
                    .connector_refund_id
                    .clone()
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?,
            },
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservRefundRequest {
    amount: Amount,
    merchant_details: MerchantDetails,
    reference_transaction_details: ReferenceTransactionDetails,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for FiservRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
        let metadata = item
            .connector_meta_data
            .clone()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let session: SessionObject = metadata
            .parse_value("SessionObject")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            amount: Amount {
                total: utils::to_currency_base_unit(
                    item.request.refund_amount,
                    item.request.currency,
                )?,
                currency: item.request.currency.to_string(),
            },
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: Some(session.terminal_id),
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item.request.connector_transaction_id.to_string(),
            },
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    gateway_response: GatewayResponse,
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
                connector_refund_id: item
                    .response
                    .gateway_response
                    .transaction_processing_details
                    .transaction_id,
                refund_status: enums::RefundStatus::from(
                    item.response.gateway_response.transaction_state,
                ),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, FiservSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, FiservSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let gateway_resp = item
            .response
            .sync_responses
            .first()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: gateway_resp
                    .gateway_response
                    .transaction_processing_details
                    .transaction_id
                    .clone(),
                refund_status: enums::RefundStatus::from(
                    gateway_resp.gateway_response.transaction_state.clone(),
                ),
            }),
            ..item.data
        })
    }
}
