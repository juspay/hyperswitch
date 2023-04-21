use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressDetailsData, CardData, PaymentsAuthorizeRequestData, RouterData,
    },
    core::errors,
    pii::{self},
    types::{self, api, storage::enums, transformers::ForeignFrom},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct FortePaymentsRequest {
    action: String,
    authorization_amount: f64,
    subtotal_amount: f64,
    billing_address: BillingAddress,
    card: Card,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct BillingAddress {
    first_name: Secret<String>,
    last_name: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Card {
    card_type: ForteCardType,
    name_on_card: Secret<String>,
    account_number: Secret<String, pii::CardNumber>,
    expire_month: Secret<String>,
    expire_year: Secret<String>,
    card_verification_value: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
                payment_method: api::enums::PaymentMethod::Card.to_string(),
                connector: "Forte",
                payment_experience: api::enums::PaymentExperience::RedirectToUrl.to_string(),
            }
            .into()),
        }
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api_models::payments::PaymentMethodData::Card(ref ccard) => {
                let action = match item.request.is_auto_capture()? {
                    true => "sale",
                    false => "authorize",
                }
                .to_string();
                let card_issuer = ccard.get_card_issuer();

                let card_type = match card_issuer {
                    Ok(issuer) => ForteCardType::try_from(issuer)?,
                    Err(_) => Err(errors::ConnectorError::NotSupported {
                        payment_method: api::enums::PaymentMethod::Card.to_string(),
                        connector: "Forte",
                        payment_experience: api::enums::PaymentExperience::RedirectToUrl
                            .to_string(),
                    })?,
                };
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
                    utils::to_currency_base_unit(item.request.amount, item.request.currency)?
                        .parse::<f64>()
                        .ok()
                        .ok_or_else(|| errors::ConnectorError::RequestEncodingFailed)?;
                let subtotal_amount = authorization_amount;
                Ok(Self {
                    action,
                    authorization_amount,
                    subtotal_amount,
                    billing_address,
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Methods".to_string(),
            ))?,
        }
    }
}

// Auth Struct
pub struct ForteAuthType {
    pub(super) api_access_id: String,
    pub(super) organization_id: String,
    pub(super) location_id: String,
    pub(super) api_secret_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::MultiAuthKey {
            api_key,
            key1,
            api_secret,
            key2,
        } = auth_type
        {
            Ok(Self {
                api_access_id: api_key.to_string(),
                organization_id: format!("org_{}", key1),
                location_id: format!("loc_{}", key2),
                api_secret_key: api_secret.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
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
    fn foreign_from(item: (ForteResponseCode, ForteAction)) -> Self {
        match item.0 {
            ForteResponseCode::A01 => match item.1 {
                ForteAction::Authorize => Self::Authorized,
                ForteAction::Sale => Self::Pending,
            },
            ForteResponseCode::A05 | ForteResponseCode::A06 => Self::Authorizing,
            _ => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct CardResponse {
    name_on_card: String,
    last_4_account_number: String,
    masked_account_number: String,
    card_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ForteResponseCode {
    A01,
    A05,
    A06,
    U18,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseStatus {
    environment: String,
    response_type: String,
    response_code: ForteResponseCode,
    response_desc: String,
    authorization_code: String,
    avs_result: Option<String>,
    cvv_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ForteAction {
    Sale,
    Authorize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct FortePaymentsResponse {
    transaction_id: String,
    location_id: String,
    action: ForteAction,
    authorization_amount: Option<f64>,
    authorization_code: String,
    entered_by: String,
    billing_address: Option<BillingAddress>,
    card: Option<CardResponse>,
    response: ResponseStatus,
}

#[derive(Debug, Serialize, Default, Deserialize)]
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
            }),
            ..item.data
        })
    }
}

//PsyncResponse

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct FortePaymentsSyncResponse {
    transaction_id: String,
    location_id: String,
    status: FortePaymentStatus,
    action: ForteAction,
    authorization_amount: Option<f64>,
    authorization_code: String,
    entered_by: String,
    billing_address: Option<BillingAddress>,
    card: Option<CardResponse>,
    response: ResponseStatus,
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
            }),
            ..item.data
        })
    }
}

// Capture

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForteCaptureRequest {
    action: String,
    transaction_id: String,
    authorization_code: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for ForteCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let trn_id = &item.request.connector_transaction_id;
        let connector_auth_id: ForteMeta =
            utils::to_connector_meta(item.request.connector_meta.clone())?;
        let auth_code = connector_auth_id.auth_id;
        Ok(Self {
            action: "capture".to_string(),
            transaction_id: trn_id.to_string(),
            authorization_code: auth_code,
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CaptureResponseStatus {
    environment: String,
    response_type: String,
    response_code: String,
    response_desc: String,
    authorization_code: String,
}
// Capture Response
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForteCaptureResponse {
    transaction_id: String,
    original_transaction_id: String,
    entered_by: String,
    authorization_code: String,
    response: CaptureResponseStatus,
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<ForteCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<ForteCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let capture_status = match item.response.response.response_code.as_str() {
            "A01" => FortePaymentStatus::Complete,
            _ => FortePaymentStatus::Failed,
        };
        Ok(Self {
            status: enums::AttemptStatus::from(capture_status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(serde_json::json!(ForteMeta {
                    auth_id: item.response.authorization_code,
                })),
            }),
            amount_captured: None,
            ..item.data
        })
    }
}

//Cancel

#[derive(Default, Debug, Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CancelResponseStatus {
    environment: String,
    response_type: String,
    response_code: String,
    response_desc: String,
    authorization_code: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ForteCancelResponse {
    transaction_id: String,
    location_id: String,
    action: String,
    authorization_code: String,
    entered_by: String,
    response: CancelResponseStatus,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, ForteCancelResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, ForteCancelResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let payment_status = match item.response.response.response_code.as_str() {
            "A01" => FortePaymentStatus::Voided,
            _ => FortePaymentStatus::Failed,
        };
        let transaction_id = &item.response.transaction_id;
        Ok(Self {
            status: enums::AttemptStatus::from(payment_status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(transaction_id.to_string()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(serde_json::json!(ForteMeta {
                    auth_id: item.response.authorization_code,
                })),
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
        let trn_id = &item.request.connector_transaction_id;
        let connector_auth_id: ForteMeta =
            utils::to_connector_meta(item.request.connector_metadata.clone())?;
        let auth_code = connector_auth_id.auth_id;
        let authorization_amount =
            utils::to_currency_base_unit(item.request.amount, item.request.currency)?
                .parse::<f64>()
                .ok()
                .ok_or_else(|| errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            action: "reverse".to_string(),
            authorization_amount,
            original_transaction_id: trn_id.to_string(),
            authorization_code: auth_code,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RefundStatus {
    Ready,
    Pending,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Ready => Self::Success,
            RefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    transaction_id: String,
    original_transaction_id: String,
    action: String,
    authorization_amount: Option<f64>,
    authorization_code: String,
    response: ResponseStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.response.response_code {
            ForteResponseCode::A01 => RefundStatus::Ready,
            _ => RefundStatus::Pending,
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status: enums::RefundStatus::from(refund_status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RefundSyncResponse {
    transaction_id: String,
    response: ResponseStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.response.response_code {
            ForteResponseCode::A01 => RefundStatus::Ready,
            _ => RefundStatus::Pending,
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status: enums::RefundStatus::from(refund_status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ErrorResponseStatus {
    pub environment: String,
    pub response_type: Option<String>,
    pub response_code: Option<String>,
    pub response_desc: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ForteErrorResponse {
    pub response: ErrorResponseStatus,
}
