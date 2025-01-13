use cards::CardNumber;
use common_enums::enums;
use common_utils::types::FloatMajorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{PaymentsCaptureResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, AddressDetailsData, CardData as _PaymentsAuthorizeRequestData,
        PaymentsAuthorizeRequestData, RouterData as _,
    },
};

#[derive(Debug, Serialize)]
pub struct ForteRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for ForteRouterData<T> {
    fn from((amount, router_data): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FortePaymentsRequest {
    action: ForteAction,
    authorization_amount: FloatMajorUnit,
    billing_address: BillingAddress,
    card: Card,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct BillingAddress {
    first_name: Secret<String>,
    last_name: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct Card {
    card_type: ForteCardType,
    name_on_card: Secret<String>,
    account_number: CardNumber,
    expire_month: Secret<String>,
    expire_year: Secret<String>,
    card_verification_value: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForteCardType {
    Visa,
    MasterCard,
    Amex,
    Discover,
    DinersClub,
    Jcb,
}

impl TryFrom<utils::CardIssuer> for ForteCardType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(issuer: utils::CardIssuer) -> Result<Self, Self::Error> {
        match issuer {
            utils::CardIssuer::AmericanExpress => Ok(Self::Amex),
            utils::CardIssuer::Master => Ok(Self::MasterCard),
            utils::CardIssuer::Discover => Ok(Self::Discover),
            utils::CardIssuer::Visa => Ok(Self::Visa),
            utils::CardIssuer::DinersClub => Ok(Self::DinersClub),
            utils::CardIssuer::JCB => Ok(Self::Jcb),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Forte"),
            )
            .into()),
        }
    }
}

impl TryFrom<&ForteRouterData<&types::PaymentsAuthorizeRouterData>> for FortePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item_data: &ForteRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let item = item_data.router_data;
        if item.request.currency != enums::Currency::USD {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Forte"),
            ))?
        }
        match item.request.payment_method_data {
            PaymentMethodData::Card(ref ccard) => {
                let action = match item.request.is_auto_capture()? {
                    true => ForteAction::Sale,
                    false => ForteAction::Authorize,
                };
                let card_type = ForteCardType::try_from(ccard.get_card_issuer()?)?;
                let address = item.get_billing_address()?;
                let card = Card {
                    card_type,
                    name_on_card: item
                        .get_optional_billing_full_name()
                        .unwrap_or(Secret::new("".to_string())),
                    account_number: ccard.card_number.clone(),
                    expire_month: ccard.card_exp_month.clone(),
                    expire_year: ccard.card_exp_year.clone(),
                    card_verification_value: ccard.card_cvc.clone(),
                };
                let first_name = address.get_first_name()?;
                let billing_address = BillingAddress {
                    first_name: first_name.clone(),
                    last_name: address.get_last_name().unwrap_or(first_name).clone(),
                };
                let authorization_amount = item_data.amount;
                Ok(Self {
                    action,
                    authorization_amount,
                    billing_address,
                    card,
                })
            }
            PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment {}
            | PaymentMethodData::Reward {}
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
                    utils::get_unimplemented_payment_method_error_message("Forte"),
                ))?
            }
        }
    }
}

// Auth Struct
pub struct ForteAuthType {
    pub(super) api_access_id: Secret<String>,
    pub(super) organization_id: Secret<String>,
    pub(super) location_id: Secret<String>,
    pub(super) api_secret_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for ForteAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Ok(Self {
                api_access_id: api_key.to_owned(),
                organization_id: Secret::new(format!("org_{}", key1.peek())),
                location_id: Secret::new(format!("loc_{}", key2.peek())),
                api_secret_key: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
// PaymentsResponse
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FortePaymentStatus {
    Complete,
    Failed,
    Authorized,
    Ready,
    Voided,
    Settled,
}

impl From<FortePaymentStatus> for enums::AttemptStatus {
    fn from(item: FortePaymentStatus) -> Self {
        match item {
            FortePaymentStatus::Complete | FortePaymentStatus::Settled => Self::Charged,
            FortePaymentStatus::Failed => Self::Failure,
            FortePaymentStatus::Ready => Self::Pending,
            FortePaymentStatus::Authorized => Self::Authorized,
            FortePaymentStatus::Voided => Self::Voided,
        }
    }
}

fn get_status(response_code: ForteResponseCode, action: ForteAction) -> enums::AttemptStatus {
    match response_code {
        ForteResponseCode::A01 => match action {
            ForteAction::Authorize => enums::AttemptStatus::Authorized,
            ForteAction::Sale => enums::AttemptStatus::Pending,
            ForteAction::Verify => enums::AttemptStatus::Charged,
        },
        ForteResponseCode::A05 | ForteResponseCode::A06 => enums::AttemptStatus::Authorizing,
        _ => enums::AttemptStatus::Failure,
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CardResponse {
    pub name_on_card: Secret<String>,
    pub last_4_account_number: String,
    pub masked_account_number: String,
    pub card_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ForteResponseCode {
    A01,
    A05,
    A06,
    U13,
    U14,
    U18,
    U20,
}

impl From<ForteResponseCode> for enums::AttemptStatus {
    fn from(item: ForteResponseCode) -> Self {
        match item {
            ForteResponseCode::A01 | ForteResponseCode::A05 | ForteResponseCode::A06 => {
                Self::Pending
            }
            _ => Self::Failure,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseStatus {
    pub environment: String,
    pub response_type: String,
    pub response_code: ForteResponseCode,
    pub response_desc: String,
    pub authorization_code: String,
    pub avs_result: Option<String>,
    pub cvv_result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForteAction {
    Sale,
    Authorize,
    Verify,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FortePaymentsResponse {
    pub transaction_id: String,
    pub location_id: Secret<String>,
    pub action: ForteAction,
    pub authorization_amount: Option<FloatMajorUnit>,
    pub authorization_code: String,
    pub entered_by: String,
    pub billing_address: Option<BillingAddress>,
    pub card: Option<CardResponse>,
    pub response: ResponseStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForteMeta {
    pub auth_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, FortePaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FortePaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response_code = item.response.response.response_code;
        let action = item.response.action;
        let transaction_id = &item.response.transaction_id;
        Ok(Self {
            status: get_status(response_code, action),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id.to_string()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: Some(serde_json::json!(ForteMeta {
                    auth_id: item.response.authorization_code,
                })),
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id.to_string()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

//PsyncResponse

#[derive(Debug, Deserialize, Serialize)]
pub struct FortePaymentsSyncResponse {
    pub transaction_id: String,
    pub location_id: Secret<String>,
    pub status: FortePaymentStatus,
    pub action: ForteAction,
    pub authorization_amount: Option<FloatMajorUnit>,
    pub authorization_code: String,
    pub entered_by: String,
    pub billing_address: Option<BillingAddress>,
    pub card: Option<CardResponse>,
    pub response: ResponseStatus,
}

impl<F, T> TryFrom<ResponseRouterData<F, FortePaymentsSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FortePaymentsSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = &item.response.transaction_id;
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id.to_string()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: Some(serde_json::json!(ForteMeta {
                    auth_id: item.response.authorization_code,
                })),
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id.to_string()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// Capture

#[derive(Debug, Serialize)]
pub struct ForteCaptureRequest {
    action: String,
    transaction_id: String,
    authorization_code: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for ForteCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let trn_id = item.request.connector_transaction_id.clone();
        let connector_auth_id: ForteMeta =
            utils::to_connector_meta(item.request.connector_meta.clone())?;
        let auth_code = connector_auth_id.auth_id;
        Ok(Self {
            action: "capture".to_string(),
            transaction_id: trn_id,
            authorization_code: auth_code,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CaptureResponseStatus {
    pub environment: String,
    pub response_type: String,
    pub response_code: ForteResponseCode,
    pub response_desc: String,
    pub authorization_code: String,
}
// Capture Response
#[derive(Debug, Deserialize, Serialize)]
pub struct ForteCaptureResponse {
    pub transaction_id: String,
    pub original_transaction_id: String,
    pub entered_by: String,
    pub authorization_code: String,
    pub response: CaptureResponseStatus,
}

impl TryFrom<PaymentsCaptureResponseRouterData<ForteCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<ForteCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = &item.response.transaction_id;
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.response.response_code),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: Some(serde_json::json!(ForteMeta {
                    auth_id: item.response.authorization_code,
                })),
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id.to_string()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            amount_captured: None,
            ..item.data
        })
    }
}

//Cancel

#[derive(Debug, Serialize)]
pub struct ForteCancelRequest {
    action: String,
    authorization_code: String,
}

impl TryFrom<&types::PaymentsCancelRouterData> for ForteCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let action = "void".to_string();
        let connector_auth_id: ForteMeta =
            utils::to_connector_meta(item.request.connector_meta.clone())?;
        let authorization_code = connector_auth_id.auth_id;
        Ok(Self {
            action,
            authorization_code,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CancelResponseStatus {
    pub response_type: String,
    pub response_code: ForteResponseCode,
    pub response_desc: String,
    pub authorization_code: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ForteCancelResponse {
    pub transaction_id: String,
    pub location_id: Secret<String>,
    pub action: String,
    pub authorization_code: String,
    pub entered_by: String,
    pub response: CancelResponseStatus,
}

impl<F, T> TryFrom<ResponseRouterData<F, ForteCancelResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, ForteCancelResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = &item.response.transaction_id;
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.response.response_code),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id.to_string()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: Some(serde_json::json!(ForteMeta {
                    auth_id: item.response.authorization_code,
                })),
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id.to_string()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
#[derive(Default, Debug, Serialize)]
pub struct ForteRefundRequest {
    action: String,
    authorization_amount: FloatMajorUnit,
    original_transaction_id: String,
    authorization_code: String,
}

impl<F> TryFrom<&ForteRouterData<&types::RefundsRouterData<F>>> for ForteRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item_data: &ForteRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let item = item_data.router_data;
        let trn_id = item.request.connector_transaction_id.clone();
        let connector_auth_id: ForteMeta =
            utils::to_connector_meta(item.request.connector_metadata.clone())?;
        let auth_code = connector_auth_id.auth_id;
        let authorization_amount = item_data.amount;
        Ok(Self {
            action: "reverse".to_string(),
            authorization_amount,
            original_transaction_id: trn_id,
            authorization_code: auth_code,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RefundStatus {
    Complete,
    Ready,
    Failed,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Complete => Self::Success,
            RefundStatus::Ready => Self::Pending,
            RefundStatus::Failed => Self::Failure,
        }
    }
}
impl From<ForteResponseCode> for enums::RefundStatus {
    fn from(item: ForteResponseCode) -> Self {
        match item {
            ForteResponseCode::A01 | ForteResponseCode::A05 | ForteResponseCode::A06 => {
                Self::Pending
            }
            _ => Self::Failure,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RefundResponse {
    pub transaction_id: String,
    pub original_transaction_id: String,
    pub action: String,
    pub authorization_amount: Option<FloatMajorUnit>,
    pub authorization_code: String,
    pub response: ResponseStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status: enums::RefundStatus::from(item.response.response.response_code),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RefundSyncResponse {
    status: RefundStatus,
    transaction_id: String,
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundSyncResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponseStatus {
    pub environment: String,
    pub response_type: Option<String>,
    pub response_code: Option<String>,
    pub response_desc: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ForteErrorResponse {
    pub response: ErrorResponseStatus,
}
