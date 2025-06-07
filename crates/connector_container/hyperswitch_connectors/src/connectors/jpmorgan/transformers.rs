use common_enums::enums::CaptureMethod;
use common_utils::types::MinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsCancelData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefreshTokenRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        get_unimplemented_payment_method_error_message, CardData, RouterData as OtherRouterData,
    },
};
pub struct JpmorganRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for JpmorganRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JpmorganAuthUpdateRequest {
    pub grant_type: String,
    pub scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JpmorganAuthUpdateResponse {
    pub access_token: Secret<String>,
    pub scope: String,
    pub token_type: String,
    pub expires_in: i64,
}

impl TryFrom<&RefreshTokenRouterData> for JpmorganAuthUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &RefreshTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            grant_type: String::from("client_credentials"),
            scope: String::from("jpm:payments:sandbox"),
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, JpmorganAuthUpdateResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, JpmorganAuthUpdateResponse, T, AccessToken>,
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

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPaymentsRequest {
    capture_method: CapMethod,
    amount: MinorUnit,
    currency: common_enums::Currency,
    merchant: JpmorganMerchant,
    payment_method_type: JpmorganPaymentMethodType,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganCard {
    account_number: Secret<String>,
    expiry: Expiry,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPaymentMethodType {
    card: JpmorganCard,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expiry {
    month: Secret<String>,
    year: Secret<String>,
}

#[derive(Serialize, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganMerchantSoftware {
    company_name: Secret<String>,
    product_name: Secret<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganMerchant {
    merchant_software: JpmorganMerchantSoftware,
}

fn map_capture_method(
    capture_method: CaptureMethod,
) -> Result<CapMethod, error_stack::Report<errors::ConnectorError>> {
    match capture_method {
        CaptureMethod::Automatic => Ok(CapMethod::Now),
        CaptureMethod::Manual => Ok(CapMethod::Manual),
        CaptureMethod::Scheduled
        | CaptureMethod::ManualMultiple
        | CaptureMethod::SequentialAutomatic => {
            Err(errors::ConnectorError::NotImplemented("Capture Method".to_string()).into())
        }
    }
}

impl TryFrom<&JpmorganRouterData<&PaymentsAuthorizeRouterData>> for JpmorganPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &JpmorganRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                if item.router_data.is_three_ds() {
                    return Err(errors::ConnectorError::NotSupported {
                        message: "3DS payments".to_string(),
                        connector: "Jpmorgan",
                    }
                    .into());
                }

                let capture_method =
                    map_capture_method(item.router_data.request.capture_method.unwrap_or_default());

                let merchant_software = JpmorganMerchantSoftware {
                    company_name: String::from("JPMC").into(),
                    product_name: String::from("Hyperswitch").into(),
                };

                let merchant = JpmorganMerchant { merchant_software };

                let expiry: Expiry = Expiry {
                    month: req_card.card_exp_month.clone(),
                    year: req_card.get_expiry_year_4_digit(),
                };

                let account_number = Secret::new(req_card.card_number.to_string());

                let card = JpmorganCard {
                    account_number,
                    expiry,
                };

                let payment_method_type = JpmorganPaymentMethodType { card };

                Ok(Self {
                    capture_method: capture_method?,
                    currency: item.router_data.request.currency,
                    amount: item.amount,
                    merchant,
                    payment_method_type,
                })
            }
            PaymentMethodData::CardDetailsForNetworkTransactionId(_)
            | PaymentMethodData::CardRedirect(_)
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
            | PaymentMethodData::NetworkToken(_) => Err(errors::ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("jpmorgan"),
            )
            .into()),
        }
    }
}

//JP Morgan uses access token only due to which we aren't reading the fields in this struct
#[derive(Debug)]
pub struct JpmorganAuthType {
    pub(super) _api_key: Secret<String>,
    pub(super) _key1: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for JpmorganAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                _api_key: api_key.to_owned(),
                _key1: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum JpmorganTransactionStatus {
    Success,
    Denied,
    Error,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum JpmorganTransactionState {
    Closed,
    Authorized,
    Voided,
    #[default]
    Pending,
    Declined,
    Error,
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPaymentsResponse {
    transaction_id: String,
    request_id: String,
    transaction_state: JpmorganTransactionState,
    response_status: String,
    response_code: String,
    response_message: String,
    payment_method_type: PaymentMethodType,
    capture_method: Option<CapMethod>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Merchant {
    merchant_id: Option<String>,
    merchant_software: MerchantSoftware,
    merchant_category_code: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantSoftware {
    company_name: Secret<String>,
    product_name: Secret<String>,
    version: Option<Secret<String>>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodType {
    card: Option<Card>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    expiry: Option<ExpiryResponse>,
    card_type: Option<Secret<String>>,
    card_type_name: Option<Secret<String>>,
    masked_account_number: Option<Secret<String>>,
    card_type_indicators: Option<CardTypeIndicators>,
    network_response: Option<NetworkResponse>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkResponse {
    address_verification_result: Option<Secret<String>>,
    address_verification_result_code: Option<Secret<String>>,
    card_verification_result_code: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpiryResponse {
    month: Option<Secret<i32>>,
    year: Option<Secret<i32>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardTypeIndicators {
    issuance_country_code: Option<Secret<String>>,
    is_durbin_regulated: Option<bool>,
    card_product_types: Secret<Vec<String>>,
}

pub fn attempt_status_from_transaction_state(
    transaction_state: JpmorganTransactionState,
) -> common_enums::AttemptStatus {
    match transaction_state {
        JpmorganTransactionState::Authorized => common_enums::AttemptStatus::Authorized,
        JpmorganTransactionState::Closed => common_enums::AttemptStatus::Charged,
        JpmorganTransactionState::Declined | JpmorganTransactionState::Error => {
            common_enums::AttemptStatus::Failure
        }
        JpmorganTransactionState::Pending => common_enums::AttemptStatus::Pending,
        JpmorganTransactionState::Voided => common_enums::AttemptStatus::Voided,
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, JpmorganPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, JpmorganPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let transaction_state = match item.response.transaction_state {
            JpmorganTransactionState::Closed => match item.response.capture_method {
                Some(CapMethod::Now) => JpmorganTransactionState::Closed,
                _ => JpmorganTransactionState::Authorized,
            },
            JpmorganTransactionState::Authorized => JpmorganTransactionState::Authorized,
            JpmorganTransactionState::Voided => JpmorganTransactionState::Voided,
            JpmorganTransactionState::Pending => JpmorganTransactionState::Pending,
            JpmorganTransactionState::Declined => JpmorganTransactionState::Declined,
            JpmorganTransactionState::Error => JpmorganTransactionState::Error,
        };
        let status = attempt_status_from_transaction_state(transaction_state);

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganCaptureRequest {
    capture_method: Option<CapMethod>,
    amount: MinorUnit,
    currency: Option<common_enums::Currency>,
}

#[derive(Debug, Default, Copy, Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum CapMethod {
    #[default]
    Now,
    Delayed,
    Manual,
}

impl TryFrom<&JpmorganRouterData<&PaymentsCaptureRouterData>> for JpmorganCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &JpmorganRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let capture_method = Some(map_capture_method(
            item.router_data.request.capture_method.unwrap_or_default(),
        )?);
        Ok(Self {
            capture_method,
            amount: item.amount,
            currency: Some(item.router_data.request.currency),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganCaptureResponse {
    pub transaction_id: String,
    pub request_id: String,
    pub transaction_state: JpmorganTransactionState,
    pub response_status: JpmorganTransactionStatus,
    pub response_code: String,
    pub response_message: String,
    pub payment_method_type: PaymentMethodTypeCapRes,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodTypeCapRes {
    pub card: Option<CardCapRes>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardCapRes {
    pub card_type: Option<Secret<String>>,
    pub card_type_name: Option<Secret<String>>,
    unmasked_account_number: Option<Secret<String>>,
}

impl<F, T> TryFrom<ResponseRouterData<F, JpmorganCaptureResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, JpmorganCaptureResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = attempt_status_from_transaction_state(item.response.transaction_state);
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPSyncResponse {
    transaction_id: String,
    transaction_state: JpmorganTransactionState,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum JpmorganResponseStatus {
    Success,
    Denied,
    Error,
}

impl<F, PaymentsSyncData>
    TryFrom<ResponseRouterData<F, JpmorganPSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, JpmorganPSyncResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = attempt_status_from_transaction_state(item.response.transaction_state);
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionData {
    payment_type: Option<Secret<String>>,
    status_code: Secret<i32>,
    txn_secret: Option<Secret<String>>,
    tid: Option<Secret<i64>>,
    test_mode: Option<Secret<i8>>,
    status: Option<JpmorganTransactionStatus>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganRefundRequest {
    pub merchant: MerchantRefundReq,
    pub amount: MinorUnit,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantRefundReq {
    pub merchant_software: MerchantSoftware,
}

impl<F> TryFrom<&JpmorganRouterData<&RefundsRouterData<F>>> for JpmorganRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &JpmorganRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented("Refunds".to_string()).into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganRefundResponse {
    pub transaction_id: Option<String>,
    pub request_id: String,
    pub transaction_state: JpmorganTransactionState,
    pub amount: MinorUnit,
    pub currency: common_enums::Currency,
    pub response_status: JpmorganResponseStatus,
    pub response_code: String,
    pub response_message: String,
    pub transaction_reference_id: Option<String>,
    pub remaining_refundable_amount: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for common_enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

pub fn refund_status_from_transaction_state(
    transaction_state: JpmorganTransactionState,
) -> common_enums::RefundStatus {
    match transaction_state {
        JpmorganTransactionState::Voided | JpmorganTransactionState::Closed => {
            common_enums::RefundStatus::Success
        }
        JpmorganTransactionState::Declined | JpmorganTransactionState::Error => {
            common_enums::RefundStatus::Failure
        }
        JpmorganTransactionState::Pending | JpmorganTransactionState::Authorized => {
            common_enums::RefundStatus::Pending
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, JpmorganRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, JpmorganRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item
                    .response
                    .transaction_id
                    .clone()
                    .ok_or(errors::ConnectorError::ResponseHandlingFailed)?,
                refund_status: refund_status_from_transaction_state(
                    item.response.transaction_state,
                ),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganRefundSyncResponse {
    transaction_id: String,
    request_id: String,
    transaction_state: JpmorganTransactionState,
    amount: MinorUnit,
    currency: common_enums::Currency,
    response_status: JpmorganResponseStatus,
    response_code: String,
}

impl TryFrom<RefundsResponseRouterData<RSync, JpmorganRefundSyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, JpmorganRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id.clone(),
                refund_status: refund_status_from_transaction_state(
                    item.response.transaction_state,
                ),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganCancelRequest {
    pub amount: Option<i64>,
    pub is_void: Option<bool>,
    pub reversal_reason: Option<String>,
}

impl TryFrom<JpmorganRouterData<&PaymentsCancelRouterData>> for JpmorganCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: JpmorganRouterData<&PaymentsCancelRouterData>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.router_data.request.amount,
            is_void: Some(true),
            reversal_reason: item.router_data.request.cancellation_reason.clone(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganCancelResponse {
    transaction_id: String,
    request_id: String,
    response_status: JpmorganResponseStatus,
    response_code: String,
    response_message: String,
    payment_method_type: JpmorganPaymentMethodTypeCancelResponse,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganPaymentMethodTypeCancelResponse {
    pub card: CardCancelResponse,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardCancelResponse {
    pub card_type: Secret<String>,
    pub card_type_name: Secret<String>,
}

impl<F>
    TryFrom<ResponseRouterData<F, JpmorganCancelResponse, PaymentsCancelData, PaymentsResponseData>>
    for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            JpmorganCancelResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = match item.response.response_status {
            JpmorganResponseStatus::Success => common_enums::AttemptStatus::Voided,
            JpmorganResponseStatus::Denied | JpmorganResponseStatus::Error => {
                common_enums::AttemptStatus::Failure
            }
        };
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganValidationErrors {
    pub code: Option<String>,
    pub message: Option<String>,
    pub entity: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganErrorInformation {
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JpmorganErrorResponse {
    pub response_status: JpmorganTransactionStatus,
    pub response_code: String,
    pub response_message: Option<String>,
}
