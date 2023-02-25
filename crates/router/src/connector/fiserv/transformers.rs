use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    pii::{self, Secret},
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FiservPaymentsRequest {
    amount: Amount,
    source: Source,
    transaction_details: TransactionDetails,
    merchant_details: MerchantDetails,
    transaction_interaction: TransactionInteraction,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    source_type: String,
    card: CardData,
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
pub struct Amount {
    total: i64,
    currency: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    capture_flag: bool,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantDetails {
    merchant_id: String,
    terminal_id: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInteraction {
    origin: String,
    eci_indicator: String,
    pos_condition_code: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FiservPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
                let amount = Amount {
                    total: item.request.amount,
                    currency: item.request.currency.to_string(),
                };

                let card = CardData {
                    card_data: ccard.card_number.clone(),
                    expiration_month: ccard.card_exp_month.clone(),
                    expiration_year: ccard.card_exp_year.clone(),
                    security_code: ccard.card_cvc.clone(),
                };
                let source = Source {
                    source_type: "PaymentCard".to_string(),
                    card,
                };
                let transaction_details = TransactionDetails {
                    capture_flag: matches!(
                        item.request.capture_method,
                        Some(enums::CaptureMethod::Automatic) | None
                    ),
                };
                let metadata = item
                    .connector_meta_data
                    .clone()
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
                let session: SessionObject = metadata
                    .parse_value("SessionObject")
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                let merchant_details = MerchantDetails {
                    merchant_id: auth.merchant_account,
                    terminal_id: session.terminal_id,
                };

                let transaction_interaction = TransactionInteraction {
                    origin: "ECOM".to_string(), //Payment is being made in online mode, card not present
                    eci_indicator: "CHANNEL_ENCRYPTED".to_string(), // transaction encryption such as SSL/TLS, but authentication was not performed
                    pos_condition_code: "CARD_NOT_PRESENT_ECOM".to_string(), //card not present in online transaction
                };
                Ok(Self {
                    amount,
                    source,
                    transaction_details,
                    merchant_details,
                    transaction_interaction,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Methods".to_string(),
            ))?,
        }
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

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FiservPaymentsResponse {
    gateway_response: GatewayResponse,
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
            status: gateway_resp.transaction_state.into(),
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
        let amount = item
            .request
            .amount_to_capture
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            amount: Amount {
                total: amount,
                currency: item.request.currency.to_string(),
            },
            transaction_details: TransactionDetails { capture_flag: true },
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: session.terminal_id,
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item.request.connector_transaction_id.to_string(),
            },
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct FiservRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for FiservRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented("fiserv".to_string()).into())
    }
}

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
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented("fiserv".to_string()).into())
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented("fiserv".to_string()).into())
    }
}
