use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self as conn_utils, RefundsRequestData, RouterData},
    core::errors,
    services,
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NoonChannels {
    Web,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonOrder {
    amount: String,
    currency: storage_models::enums::Currency,
    channel: NoonChannels,
    category: String,
    //Short description of the order.
    name: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NoonPaymentActions {
    Authorize,
    Sale,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonConfiguration {
    payment_action: NoonPaymentActions,
    return_url: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonCard {
    name_on_card: Secret<String>,
    number_plain: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvv: Secret<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum NoonPaymentData {
    Card(NoonCard),
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NoonApiOperations {
    Initiate,
    Capture,
    Reverse,
    Refund,
}
#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsRequest {
    api_operation: NoonApiOperations,
    order: NoonOrder,
    configuration: NoonConfiguration,
    payment_data: NoonPaymentData,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NoonPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let payment_data = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => Ok(NoonPaymentData::Card(NoonCard {
                name_on_card: req_card.card_holder_name,
                number_plain: req_card.card_number,
                expiry_month: req_card.card_exp_month,
                expiry_year: req_card.card_exp_year,
                cvv: req_card.card_cvc,
            })),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment methods".to_string(),
            )),
        }?;

        let order = NoonOrder {
            amount: conn_utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            currency: item.request.currency,
            channel: NoonChannels::Web,
            category: "pay".to_string(),
            name: item.get_description()?,
        };
        let payment_action = match item.request.capture_method {
            Some(enums::CaptureMethod::Manual) => NoonPaymentActions::Authorize,
            _ => NoonPaymentActions::Sale,
        };
        Ok(Self {
            api_operation: NoonApiOperations::Initiate,
            order,
            configuration: NoonConfiguration {
                payment_action,
                return_url: item.request.router_return_url.clone(),
            },
            payment_data,
        })
    }
}

// Auth Struct
pub struct NoonAuthType {
    pub(super) api_key: String,
    pub(super) application_identifier: String,
    pub(super) business_identifier: String,
}

impl TryFrom<&types::ConnectorAuthType> for NoonAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_key: api_key.to_string(),
                application_identifier: api_secret.to_string(),
                business_identifier: key1.to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NoonPaymentStatus {
    Authorized,
    Captured,
    Reversed,
    #[serde(rename = "3DS_ENROLL_INITIATED")]
    ThreeDsEnrollInitiated,
    Failed,
}

impl From<NoonPaymentStatus> for enums::AttemptStatus {
    fn from(item: NoonPaymentStatus) -> Self {
        match item {
            NoonPaymentStatus::Authorized => Self::Authorized,
            NoonPaymentStatus::Captured => Self::Charged,
            NoonPaymentStatus::Reversed => Self::Voided,
            NoonPaymentStatus::ThreeDsEnrollInitiated => Self::AuthenticationPending,
            NoonPaymentStatus::Failed => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsOrderResponse {
    status: NoonPaymentStatus,
    id: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonCheckoutData {
    post_url: url::Url,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsResponseResult {
    order: NoonPaymentsOrderResponse,
    checkout_data: Option<NoonCheckoutData>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct NoonPaymentsResponse {
    result: NoonPaymentsResponseResult,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, NoonPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, NoonPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item.response.result.checkout_data.map(|redirection_data| {
            services::RedirectForm::Form {
                endpoint: redirection_data.post_url.to_string(),
                method: services::Method::Post,
                form_fields: std::collections::HashMap::new(),
            }
        });
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.result.order.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.result.order.id.to_string(),
                ),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonActionTransaction {
    amount: String,
    currency: storage_models::enums::Currency,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonActionOrder {
    id: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsActionRequest {
    api_operation: NoonApiOperations,
    order: NoonActionOrder,
    transaction: NoonActionTransaction,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for NoonPaymentsActionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let order = NoonActionOrder {
            id: item.request.connector_transaction_id.clone(),
        };
        let transaction = NoonActionTransaction {
            amount: conn_utils::to_currency_base_unit(
                item.request.amount_to_capture,
                item.request.currency,
            )?,
            currency: item.request.currency,
        };
        Ok(Self {
            api_operation: NoonApiOperations::Capture,
            order,
            transaction,
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsCancelRequest {
    api_operation: NoonApiOperations,
    order: NoonActionOrder,
}

impl TryFrom<&types::PaymentsCancelRouterData> for NoonPaymentsCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let order = NoonActionOrder {
            id: item.request.connector_transaction_id.clone(),
        };
        Ok(Self {
            api_operation: NoonApiOperations::Reverse,
            order,
        })
    }
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for NoonPaymentsActionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let order = NoonActionOrder {
            id: item.request.connector_transaction_id.clone(),
        };
        let transaction = NoonActionTransaction {
            amount: conn_utils::to_currency_base_unit(
                item.request.refund_amount,
                item.request.currency,
            )?,
            currency: item.request.currency,
        };
        Ok(Self {
            api_operation: NoonApiOperations::Refund,
            order,
            transaction,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Success,
    Failed,
    #[default]
    Pending,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Success => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsTransactionResponse {
    id: String,
    status: RefundStatus,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct NoonRefundResponseResult {
    transaction: NoonPaymentsTransactionResponse,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct RefundResponse {
    result: NoonRefundResponseResult,
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
                connector_refund_id: item.response.result.transaction.id,
                refund_status: enums::RefundStatus::from(item.response.result.transaction.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct NoonRefundResponseTransactions {
    id: String,
    status: RefundStatus,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct NoonRefundSyncResponseResult {
    transactions: Vec<NoonRefundResponseTransactions>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct RefundSyncResponse {
    result: NoonRefundSyncResponseResult,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let connector_refund_id = item.data.request.get_connector_refund_id()?;
        let noon_transaction: &NoonRefundResponseTransactions = item
            .response
            .result
            .transactions
            .iter()
            .find(|transaction| transaction.id == connector_refund_id)
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: noon_transaction.id.to_owned(),
                refund_status: enums::RefundStatus::from(noon_transaction.status.to_owned()),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoonErrorResponse {
    pub result_code: u32,
    pub message: String,
    pub class_description: String,
}
