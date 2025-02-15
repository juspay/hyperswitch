use std::collections::HashMap;

use api_models::payments::{self, AdditionalPaymentData};
use common_enums::enums;
use common_utils::{pii::Email, request::Method, types::MinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsAuthorizeData, ResponseId, SetupMandateRequestData},
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types,
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        PaymentsSyncResponseRouterData, RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        get_unimplemented_payment_method_error_message, AdditionalCardInfo, CardData as _,
        PaymentsAuthorizeRequestData, RouterData as _,
    },
};

const TRANSACTION_ALREADY_CANCELLED: &str = "transaction already canceled";
const TRANSACTION_ALREADY_SETTLED: &str = "already settled";
const REDIRECTION_SBX_URL: &str = "https://pay.sandbox.datatrans.com";
const REDIRECTION_PROD_URL: &str = "https://pay.datatrans.com";

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DatatransErrorResponse {
    pub error: DatatransError,
}
pub struct DatatransAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) passcode: Secret<String>,
}

pub struct DatatransRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DatatransPaymentsRequest {
    pub amount: Option<MinorUnit>,
    pub currency: enums::Currency,
    pub card: DataTransPaymentDetails,
    pub refno: String,
    pub auto_settle: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect: Option<RedirectUrls>,
    pub option: Option<DataTransCreateAlias>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataTransCreateAlias {
    pub create_alias: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RedirectUrls {
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
    pub error_url: Option<String>,
}
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Payment,
    Credit,
    CardCheck,
}
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Initialized,
    Authenticated,
    Authorized,
    Settled,
    Canceled,
    Transmitted,
    Failed,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
pub enum DatatransSyncResponse {
    Error(DatatransError),
    Response(SyncResponse),
}
#[derive(Debug, Deserialize, Serialize)]
pub enum DataTransCaptureResponse {
    Error(DatatransError),
    Empty,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum DataTransCancelResponse {
    Error(DatatransError),
    Empty,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    pub transaction_id: String,
    #[serde(rename = "type")]
    pub res_type: TransactionType,
    pub status: TransactionStatus,
    pub detail: SyncDetails,
    pub card: Option<SyncCardDetails>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncCardDetails {
    pub alias: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncDetails {
    fail: Option<FailDetails>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FailDetails {
    reason: Option<String>,
    message: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum DataTransPaymentDetails {
    Cards(PlainCardDetails),
    Mandate(MandateDetails),
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlainCardDetails {
    #[serde(rename = "type")]
    pub res_type: String,
    pub number: cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvv: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "3D")]
    pub three_ds: Option<ThreeDSecureData>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MandateDetails {
    #[serde(rename = "type")]
    pub res_type: String,
    pub alias: String,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ThreedsInfo {
    cardholder: CardHolder,
}

#[derive(Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum ThreeDSecureData {
    Cardholder(ThreedsInfo),
    Authentication(ThreeDSData),
}
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSData {
    #[serde(rename = "threeDSTransactionId")]
    pub three_ds_transaction_id: Secret<String>,
    pub cavv: Secret<String>,
    pub eci: Option<String>,
    pub xid: Option<Secret<String>>,
    #[serde(rename = "threeDSVersion")]
    pub three_ds_version: String,
    #[serde(rename = "authenticationResponse")]
    pub authentication_response: String,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardHolder {
    cardholder_name: Secret<String>,
    email: Email,
}

#[derive(Debug, Clone, Serialize, Default, Deserialize)]
pub struct DatatransError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DatatransResponse {
    TransactionResponse(DatatransSuccessResponse),
    ErrorResponse(DatatransError),
    ThreeDSResponse(Datatrans3DSResponse),
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatatransSuccessResponse {
    pub transaction_id: String,
    pub acquirer_authorization_code: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DatatransRefundsResponse {
    Success(DatatransSuccessResponse),
    Error(DatatransError),
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Datatrans3DSResponse {
    pub transaction_id: String,
    #[serde(rename = "3D")]
    pub three_ds_enrolled: ThreeDSEnolled,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSEnolled {
    pub enrolled: bool,
}

#[derive(Default, Debug, Serialize)]
pub struct DatatransRefundRequest {
    pub amount: MinorUnit,
    pub currency: enums::Currency,
    pub refno: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DataPaymentCaptureRequest {
    pub amount: MinorUnit,
    pub currency: enums::Currency,
    pub refno: String,
}

impl<T> TryFrom<(MinorUnit, T)> for DatatransRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (MinorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

impl TryFrom<&types::SetupMandateRouterData> for DatatransPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SetupMandateRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => Ok(Self {
                amount: None,
                currency: item.request.currency,
                card: DataTransPaymentDetails::Cards(PlainCardDetails {
                    res_type: "PLAIN".to_string(),
                    number: req_card.card_number.clone(),
                    expiry_month: req_card.card_exp_month.clone(),
                    expiry_year: req_card.get_card_expiry_year_2_digit()?,
                    cvv: req_card.card_cvc.clone(),
                    three_ds: Some(ThreeDSecureData::Cardholder(ThreedsInfo {
                        cardholder: CardHolder {
                            cardholder_name: item.get_billing_full_name()?,
                            email: item.get_billing_email()?,
                        },
                    })),
                }),
                refno: item.connector_request_reference_id.clone(),
                auto_settle: true, // zero auth doesn't support manual capture
                option: Some(DataTransCreateAlias { create_alias: true }),
                redirect: Some(RedirectUrls {
                    success_url: item.request.router_return_url.clone(),
                    cancel_url: item.request.router_return_url.clone(),
                    error_url: item.request.router_return_url.clone(),
                }),
            }),
            PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("Datatrans"),
                ))?
            }
        }
    }
}

impl TryFrom<&DatatransRouterData<&types::PaymentsAuthorizeRouterData>>
    for DatatransPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DatatransRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let is_mandate_payment = item.router_data.request.is_mandate_payment();
                let option =
                    is_mandate_payment.then_some(DataTransCreateAlias { create_alias: true });
                // provides return url for only mandate payment(CIT) or 3ds through datatrans
                let redirect = if is_mandate_payment
                    || (item.router_data.is_three_ds()
                        && item.router_data.request.authentication_data.is_none())
                {
                    Some(RedirectUrls {
                        success_url: item.router_data.request.router_return_url.clone(),
                        cancel_url: item.router_data.request.router_return_url.clone(),
                        error_url: item.router_data.request.router_return_url.clone(),
                    })
                } else {
                    None
                };
                Ok(Self {
                    amount: Some(item.amount),
                    currency: item.router_data.request.currency,
                    card: create_card_details(item, &req_card)?,
                    refno: item.router_data.connector_request_reference_id.clone(),
                    auto_settle: item.router_data.request.is_auto_capture()?,
                    option,
                    redirect,
                })
            }
            PaymentMethodData::MandatePayment => {
                let additional_payment_data = match item
                    .router_data
                    .request
                    .additional_payment_method_data
                    .clone()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "additional_payment_method_data",
                    })? {
                    AdditionalPaymentData::Card(card) => *card,
                    _ => Err(errors::ConnectorError::NotSupported {
                        message: "Payment Method Not Supported".to_string(),
                        connector: "DataTrans",
                    })?,
                };
                Ok(Self {
                    amount: Some(item.amount),
                    currency: item.router_data.request.currency,
                    card: create_mandate_details(item, &additional_payment_data)?,
                    refno: item.router_data.connector_request_reference_id.clone(),
                    auto_settle: item.router_data.request.is_auto_capture()?,
                    option: None,
                    redirect: None,
                })
            }
            PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("Datatrans"),
                ))?
            }
        }
    }
}
impl TryFrom<&ConnectorAuthType> for DatatransAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                merchant_id: key1.clone(),
                passcode: api_key.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

fn get_status(item: &DatatransResponse, is_auto_capture: bool) -> enums::AttemptStatus {
    match item {
        DatatransResponse::ErrorResponse(_) => enums::AttemptStatus::Failure,
        DatatransResponse::TransactionResponse(_) => {
            if is_auto_capture {
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Authorized
            }
        }
        DatatransResponse::ThreeDSResponse(_) => enums::AttemptStatus::AuthenticationPending,
    }
}

fn create_card_details(
    item: &DatatransRouterData<&types::PaymentsAuthorizeRouterData>,
    card: &Card,
) -> Result<DataTransPaymentDetails, error_stack::Report<errors::ConnectorError>> {
    let mut details = PlainCardDetails {
        res_type: "PLAIN".to_string(),
        number: card.card_number.clone(),
        expiry_month: card.card_exp_month.clone(),
        expiry_year: card.get_card_expiry_year_2_digit()?,
        cvv: card.card_cvc.clone(),
        three_ds: None,
    };

    if let Some(auth_data) = &item.router_data.request.authentication_data {
        details.three_ds = Some(ThreeDSecureData::Authentication(ThreeDSData {
            three_ds_transaction_id: Secret::new(auth_data.threeds_server_transaction_id.clone()),
            cavv: Secret::new(auth_data.cavv.clone()),
            eci: auth_data.eci.clone(),
            xid: auth_data.ds_trans_id.clone().map(Secret::new),
            three_ds_version: auth_data.message_version.to_string(),
            authentication_response: "Y".to_string(),
        }));
    } else if item.router_data.is_three_ds() {
        details.three_ds = Some(ThreeDSecureData::Cardholder(ThreedsInfo {
            cardholder: CardHolder {
                cardholder_name: item.router_data.get_billing_full_name()?,
                email: item.router_data.get_billing_email()?,
            },
        }));
    }
    Ok(DataTransPaymentDetails::Cards(details))
}

fn create_mandate_details(
    item: &DatatransRouterData<&types::PaymentsAuthorizeRouterData>,
    additional_card_details: &payments::AdditionalCardInfo,
) -> Result<DataTransPaymentDetails, error_stack::Report<errors::ConnectorError>> {
    let alias = item.router_data.request.get_connector_mandate_id()?;
    Ok(DataTransPaymentDetails::Mandate(MandateDetails {
        res_type: "ALIAS".to_string(),
        alias,
        expiry_month: additional_card_details.card_exp_month.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "card_exp_month",
            },
        )?,
        expiry_year: additional_card_details.get_card_expiry_year_2_digit()?,
    }))
}

impl From<SyncResponse> for enums::AttemptStatus {
    fn from(item: SyncResponse) -> Self {
        match item.res_type {
            TransactionType::Payment => match item.status {
                TransactionStatus::Authorized => Self::Authorized,
                TransactionStatus::Settled | TransactionStatus::Transmitted => Self::Charged,
                TransactionStatus::Canceled => Self::Voided,
                TransactionStatus::Failed => Self::Failure,
                TransactionStatus::Initialized | TransactionStatus::Authenticated => Self::Pending,
            },
            TransactionType::CardCheck => match item.status {
                TransactionStatus::Settled
                | TransactionStatus::Transmitted
                | TransactionStatus::Authorized => Self::Charged,
                TransactionStatus::Canceled => Self::Voided,
                TransactionStatus::Failed => Self::Failure,
                TransactionStatus::Initialized | TransactionStatus::Authenticated => Self::Pending,
            },
            TransactionType::Credit => Self::Failure,
        }
    }
}

impl From<SyncResponse> for enums::RefundStatus {
    fn from(item: SyncResponse) -> Self {
        match item.res_type {
            TransactionType::Credit => match item.status {
                TransactionStatus::Settled | TransactionStatus::Transmitted => Self::Success,
                TransactionStatus::Initialized
                | TransactionStatus::Authenticated
                | TransactionStatus::Authorized
                | TransactionStatus::Canceled
                | TransactionStatus::Failed => Self::Failure,
            },
            TransactionType::Payment | TransactionType::CardCheck => Self::Failure,
        }
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, DatatransResponse, PaymentsAuthorizeData, PaymentsResponseData>>
    for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, DatatransResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = get_status(&item.response, item.data.request.is_auto_capture()?);
        let response = match &item.response {
            DatatransResponse::ErrorResponse(error) => Err(ErrorResponse {
                code: error.code.clone(),
                message: error.message.clone(),
                reason: Some(error.message.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
            }),
            DatatransResponse::TransactionResponse(response) => {
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        response.transaction_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            }
            DatatransResponse::ThreeDSResponse(response) => {
                let redirection_link = match item.data.test_mode {
                    Some(true) => format!("{}/v1/start", REDIRECTION_SBX_URL),
                    Some(false) | None => format!("{}/v1/start", REDIRECTION_PROD_URL),
                };
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        response.transaction_id.clone(),
                    ),
                    redirection_data: Box::new(Some(RedirectForm::Form {
                        endpoint: format!("{}/{}", redirection_link, response.transaction_id),
                        method: Method::Get,
                        form_fields: HashMap::new(),
                    })),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, DatatransResponse, SetupMandateRequestData, PaymentsResponseData>>
    for RouterData<F, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            DatatransResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        // zero auth doesn't support manual capture
        let status = get_status(&item.response, true);
        let response = match &item.response {
            DatatransResponse::ErrorResponse(error) => Err(ErrorResponse {
                code: error.code.clone(),
                message: error.message.clone(),
                reason: Some(error.message.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
            }),
            DatatransResponse::TransactionResponse(response) => {
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        response.transaction_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            }
            DatatransResponse::ThreeDSResponse(response) => {
                let redirection_link = match item.data.test_mode {
                    Some(true) => format!("{}/v1/start", REDIRECTION_SBX_URL),
                    Some(false) | None => format!("{}/v1/start", REDIRECTION_PROD_URL),
                };
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        response.transaction_id.clone(),
                    ),
                    redirection_data: Box::new(Some(RedirectForm::Form {
                        endpoint: format!("{}/{}", redirection_link, response.transaction_id),
                        method: Method::Get,
                        form_fields: HashMap::new(),
                    })),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<F> TryFrom<&DatatransRouterData<&types::RefundsRouterData<F>>> for DatatransRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DatatransRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
            currency: item.router_data.request.currency,
            refno: item.router_data.request.refund_id.clone(),
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, DatatransRefundsResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, DatatransRefundsResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            DatatransRefundsResponse::Error(error) => Ok(Self {
                response: Err(ErrorResponse {
                    code: error.code.clone(),
                    message: error.message.clone(),
                    reason: Some(error.message),
                    attempt_status: None,
                    connector_transaction_id: None,
                    status_code: item.http_code,
                }),
                ..item.data
            }),
            DatatransRefundsResponse::Success(response) => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: response.transaction_id,
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            }),
        }
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, DatatransSyncResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, DatatransSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item.response {
            DatatransSyncResponse::Error(error) => Err(ErrorResponse {
                code: error.code.clone(),
                message: error.message.clone(),
                reason: Some(error.message),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
            }),
            DatatransSyncResponse::Response(response) => Ok(RefundsResponseData {
                connector_refund_id: response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(response),
            }),
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<DatatransSyncResponse>>
    for types::PaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSyncResponseRouterData<DatatransSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            DatatransSyncResponse::Error(error) => {
                let response = Err(ErrorResponse {
                    code: error.code.clone(),
                    message: error.message.clone(),
                    reason: Some(error.message),
                    attempt_status: None,
                    connector_transaction_id: None,
                    status_code: item.http_code,
                });
                Ok(Self {
                    response,
                    ..item.data
                })
            }
            DatatransSyncResponse::Response(sync_response) => {
                let status = enums::AttemptStatus::from(sync_response.clone());
                let response = if status == enums::AttemptStatus::Failure {
                    let (code, message) = match sync_response.detail.fail {
                        Some(fail_details) => (
                            fail_details.reason.unwrap_or(NO_ERROR_CODE.to_string()),
                            fail_details.message.unwrap_or(NO_ERROR_MESSAGE.to_string()),
                        ),
                        None => (NO_ERROR_CODE.to_string(), NO_ERROR_MESSAGE.to_string()),
                    };
                    Err(ErrorResponse {
                        code,
                        message: message.clone(),
                        reason: Some(message),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                    })
                } else {
                    let mandate_reference =
                        sync_response.card.as_ref().map(|card| MandateReference {
                            connector_mandate_id: Some(card.alias.clone()),
                            payment_method_id: None,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: None,
                        });
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            sync_response.transaction_id.to_string(),
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(mandate_reference),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<&DatatransRouterData<&types::PaymentsCaptureRouterData>>
    for DataPaymentCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DatatransRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            currency: item.router_data.request.currency,
            refno: item.router_data.connector_request_reference_id.clone(),
        })
    }
}

impl TryFrom<PaymentsCaptureResponseRouterData<DataTransCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: PaymentsCaptureResponseRouterData<DataTransCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let status = match item.response {
            DataTransCaptureResponse::Error(error) => {
                if error.message == *TRANSACTION_ALREADY_SETTLED {
                    common_enums::AttemptStatus::Charged
                } else {
                    common_enums::AttemptStatus::Failure
                }
            }
            // Datatrans http code 204 implies Successful Capture
            //https://api-reference.datatrans.ch/#tag/v1transactions/operation/settle
            DataTransCaptureResponse::Empty => {
                if item.http_code == 204 {
                    common_enums::AttemptStatus::Charged
                } else {
                    common_enums::AttemptStatus::Failure
                }
            }
        };
        Ok(Self {
            status,
            ..item.data
        })
    }
}

impl TryFrom<PaymentsCancelResponseRouterData<DataTransCancelResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: PaymentsCancelResponseRouterData<DataTransCancelResponse>,
    ) -> Result<Self, Self::Error> {
        let status = match item.response {
            // Datatrans http code 204 implies Successful Cancellation
            //https://api-reference.datatrans.ch/#tag/v1transactions/operation/cancel
            DataTransCancelResponse::Empty => {
                if item.http_code == 204 {
                    common_enums::AttemptStatus::Voided
                } else {
                    common_enums::AttemptStatus::Failure
                }
            }
            DataTransCancelResponse::Error(error) => {
                if error.message == *TRANSACTION_ALREADY_CANCELLED {
                    common_enums::AttemptStatus::Voided
                } else {
                    common_enums::AttemptStatus::Failure
                }
            }
        };
        Ok(Self {
            status,
            ..item.data
        })
    }
}
