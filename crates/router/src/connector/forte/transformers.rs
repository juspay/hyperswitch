use cards::CardNumber;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressDetailsData, CardData, PaymentsAuthorizeRequestData, RouterData,
    },
    core::errors,
    types::{self, api, storage::enums, transformers::ForeignFrom},
};

#[derive(Debug, Serialize)]
pub struct FortePaymentsRequest {
    action: ForteAction,
    authorization_amount: f64,
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
            _ => Err(errors::ConnectorError::NotSupported {
                message: issuer.to_string(),
                connector: "Forte",
            }
            .into()),
        }
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        if item.request.currency != enums::Currency::USD {
            Err(errors::ConnectorError::NotSupported {
                message: item.request.currency.to_string(),
                connector: "Forte",
            })?
        }
        match item.request.payment_method_data {
            api_models::payments::PaymentMethodData::Card(ref ccard) => {
                let action = match item.request.is_auto_capture()? {
                    true => ForteAction::Sale,
                    false => ForteAction::Authorize,
                };
                let card_type = ForteCardType::try_from(ccard.get_card_issuer()?)?;
                let address = item.get_billing_address()?;
                let card = Card {
                    card_type,
                    name_on_card: ccard.card_holder_name.clone(),
                    account_number: ccard.card_number.clone(),
                    expire_month: ccard.card_exp_month.clone(),
                    expire_year: ccard.card_exp_year.clone(),
                    card_verification_value: ccard.card_cvc.clone(),
                };
                let billing_address = BillingAddress {
                    first_name: address.get_first_name()?.to_owned(),
                    last_name: address.get_last_name()?.to_owned(),
                };
                let authorization_amount =
                    utils::to_currency_base_unit_asf64(item.request.amount, item.request.currency)?;
                Ok(Self {
                    action,
                    authorization_amount,
                    billing_address,
                    card,
                })
            }
            api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::Wallet(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment {}
            | api_models::payments::PaymentMethodData::Reward {}
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Forte",
                })?
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

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::MultiAuthKey {
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
#[derive(Debug, Deserialize)]
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

impl ForeignFrom<(ForteResponseCode, ForteAction)> for enums::AttemptStatus {
    fn foreign_from((response_code, action): (ForteResponseCode, ForteAction)) -> Self {
        match response_code {
            ForteResponseCode::A01 => match action {
                ForteAction::Authorize => Self::Authorized,
                ForteAction::Sale => Self::Pending,
                ForteAction::Verify => Self::Charged,
            },
            ForteResponseCode::A05 | ForteResponseCode::A06 => Self::Authorizing,
            _ => Self::Failure,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CardResponse {
    pub name_on_card: Secret<String>,
    pub last_4_account_number: String,
    pub masked_account_number: String,
    pub card_type: String,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct FortePaymentsResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub action: ForteAction,
    pub authorization_amount: Option<f64>,
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

impl<F, T>
    TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response_code = item.response.response.response_code;
        let action = item.response.action;
        let transaction_id = &item.response.transaction_id;
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((response_code, action)),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(transaction_id.to_string()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(serde_json::json!(ForteMeta {
                    auth_id: item.response.authorization_code,
                })),
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id.to_string()),
            }),
            ..item.data
        })
    }
}

//PsyncResponse

#[derive(Debug, Deserialize)]
pub struct FortePaymentsSyncResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub status: FortePaymentStatus,
    pub action: ForteAction,
    pub authorization_amount: Option<f64>,
    pub authorization_code: String,
    pub entered_by: String,
    pub billing_address: Option<BillingAddress>,
    pub card: Option<CardResponse>,
    pub response: ResponseStatus,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, FortePaymentsSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            FortePaymentsSyncResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let transaction_id = &item.response.transaction_id;
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(transaction_id.to_string()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(serde_json::json!(ForteMeta {
                    auth_id: item.response.authorization_code,
                })),
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id.to_string()),
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

#[derive(Debug, Deserialize)]
pub struct CaptureResponseStatus {
    pub environment: String,
    pub response_type: String,
    pub response_code: ForteResponseCode,
    pub response_desc: String,
    pub authorization_code: String,
}
// Capture Response
#[derive(Debug, Deserialize)]
pub struct ForteCaptureResponse {
    pub transaction_id: String,
    pub original_transaction_id: String,
    pub entered_by: String,
    pub authorization_code: String,
    pub response: CaptureResponseStatus,
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<ForteCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<ForteCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = &item.response.transaction_id;
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.response.response_code),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(transaction_id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(serde_json::json!(ForteMeta {
                    auth_id: item.response.authorization_code,
                })),
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id.to_string()),
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

#[derive(Debug, Deserialize)]
pub struct CancelResponseStatus {
    pub response_type: String,
    pub response_code: ForteResponseCode,
    pub response_desc: String,
    pub authorization_code: String,
}

#[derive(Debug, Deserialize)]
pub struct ForteCancelResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub action: String,
    pub authorization_code: String,
    pub entered_by: String,
    pub response: CancelResponseStatus,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, ForteCancelResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, ForteCancelResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = &item.response.transaction_id;
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.response.response_code),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(transaction_id.to_string()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(serde_json::json!(ForteMeta {
                    auth_id: item.response.authorization_code,
                })),
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id.to_string()),
            }),
            ..item.data
        })
    }
}

// REFUND :
#[derive(Default, Debug, Serialize)]
pub struct ForteRefundRequest {
    action: String,
    authorization_amount: f64,
    original_transaction_id: String,
    authorization_code: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ForteRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let trn_id = item.request.connector_transaction_id.clone();
        let connector_auth_id: ForteMeta =
            utils::to_connector_meta(item.request.connector_metadata.clone())?;
        let auth_code = connector_auth_id.auth_id;
        let authorization_amount =
            utils::to_currency_base_unit_asf64(item.request.refund_amount, item.request.currency)?;
        Ok(Self {
            action: "reverse".to_string(),
            authorization_amount,
            original_transaction_id: trn_id,
            authorization_code: auth_code,
        })
    }
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct RefundResponse {
    pub transaction_id: String,
    pub original_transaction_id: String,
    pub action: String,
    pub authorization_amount: Option<f64>,
    pub authorization_code: String,
    pub response: ResponseStatus,
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
                connector_refund_id: item.response.transaction_id,
                refund_status: enums::RefundStatus::from(item.response.response.response_code),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct RefundSyncResponse {
    status: RefundStatus,
    transaction_id: String,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponseStatus {
    pub environment: String,
    pub response_type: Option<String>,
    pub response_code: Option<String>,
    pub response_desc: String,
}

#[derive(Debug, Deserialize)]
pub struct ForteErrorResponse {
    pub response: ErrorResponseStatus,
}
