use std::collections::HashMap;

use api_models::payments;
use cards::CardNumber;
use common_enums::{enums, CaptureMethod, Currency};
use common_utils::{
    crypto::GenerateDigest,
    errors::CustomResult,
    ext_traits::Encode,
    request::Method,
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, RealTimePaymentData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsAuthorizeData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use strum::Display;
use url::Url;

use crate::{
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        PaymentsSyncResponseRouterData, RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{PaymentsAuthorizeRequestData, QrImage, RouterData as _},
};

pub struct FiuuRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for FiuuRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub struct FiuuAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) verify_key: Secret<String>,
    pub(super) secret_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for FiuuAuthType {
    type Error = Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                merchant_id: key1.to_owned(),
                verify_key: api_key.to_owned(),
                secret_key: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
enum TxnType {
    Sals,
    Auts,
}

impl TryFrom<Option<CaptureMethod>> for TxnType {
    type Error = Report<errors::ConnectorError>;
    fn try_from(capture_method: Option<CaptureMethod>) -> Result<Self, Self::Error> {
        match capture_method {
            Some(CaptureMethod::Automatic) => Ok(Self::Sals),
            Some(CaptureMethod::Manual) => Ok(Self::Auts),
            _ => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Display, Debug)]
#[serde(rename_all = "UPPERCASE")]
enum TxnChannel {
    #[serde(rename = "CREDITAN")]
    #[strum(serialize = "CREDITAN")]
    Creditan,
    #[serde(rename = "DuitNowSQR")]
    #[strum(serialize = "DuitNowSQR")]
    DuitNowSqr,
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "PascalCase")]
pub enum FiuuPaymentsRequest {
    QRPaymentRequest(FiuuQRPaymentRequest),
    CardPaymentRequest(FiuuCardPaymentRequest),
}

#[derive(Serialize, Debug, Deserialize)]
pub struct FiuuQRPaymentRequest {
    #[serde(rename = "merchantID")]
    merchant_id: Secret<String>,
    channel: TxnChannel,
    orderid: String,
    currency: Currency,
    amount: StringMajorUnit,
    checksum: Secret<String>,
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuCardPaymentRequest {
    #[serde(rename = "MerchantID")]
    merchant_id: Secret<String>,
    reference_no: String,
    txn_type: TxnType,
    txn_channel: TxnChannel,
    txn_currency: Currency,
    txn_amount: StringMajorUnit,
    signature: Secret<String>,
    #[serde(rename = "CC_PAN")]
    cc_pan: CardNumber,
    #[serde(rename = "CC_CVV2")]
    cc_cvv2: Secret<String>,
    #[serde(rename = "CC_MONTH")]
    cc_month: Secret<String>,
    #[serde(rename = "CC_YEAR")]
    cc_year: Secret<String>,
    #[serde(rename = "non_3DS")]
    non_3ds: i32,
    #[serde(rename = "ReturnURL")]
    return_url: Option<String>,
}

pub fn calculate_signature(
    signature_data: String,
) -> Result<Secret<String>, Report<errors::ConnectorError>> {
    let message = signature_data.as_bytes();
    let encoded_data = hex::encode(
        common_utils::crypto::Md5
            .generate_digest(message)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?,
    );
    Ok(Secret::new(encoded_data))
}

impl TryFrom<&FiuuRouterData<&PaymentsAuthorizeRouterData>> for FiuuPaymentsRequest {
    type Error = Report<errors::ConnectorError>;
    fn try_from(item: &FiuuRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        let auth = FiuuAuthType::try_from(&item.router_data.connector_auth_type)?;
        let merchant_id = auth.merchant_id.peek().to_string();
        let txn_currency = item.router_data.request.currency;
        let txn_amount = item.amount.clone();
        let reference_no = item.router_data.connector_request_reference_id.clone();
        let verify_key = auth.verify_key.peek().to_string();
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let signature = calculate_signature(format!(
                    "{}{merchant_id}{reference_no}{verify_key}",
                    txn_amount.get_amount_as_string()
                ))?;

                Ok(Self::CardPaymentRequest(FiuuCardPaymentRequest {
                    merchant_id: auth.merchant_id,
                    reference_no,
                    txn_type: match item.router_data.request.is_auto_capture()? {
                        true => TxnType::Sals,
                        false => TxnType::Auts,
                    },
                    txn_channel: TxnChannel::Creditan,
                    txn_currency,
                    txn_amount,
                    signature,
                    cc_pan: req_card.card_number,
                    cc_cvv2: req_card.card_cvc,
                    cc_month: req_card.card_exp_month,
                    cc_year: req_card.card_exp_year,
                    non_3ds: match item.router_data.is_three_ds() {
                        false => 1,
                        true => 0,
                    },
                    return_url: item.router_data.request.router_return_url.clone(),
                }))
            }
            PaymentMethodData::RealTimePayment(real_time_payment_data) => {
                match *real_time_payment_data {
                    RealTimePaymentData::DuitNow {} => {
                        Ok(Self::QRPaymentRequest(FiuuQRPaymentRequest {
                            merchant_id: auth.merchant_id,
                            channel: TxnChannel::DuitNowSqr,
                            orderid: reference_no.clone(),
                            currency: txn_currency,
                            amount: txn_amount.clone(),
                            checksum: calculate_signature(format!(
                                "{merchant_id}{}{reference_no}{txn_currency}{}{verify_key}",
                                TxnChannel::DuitNowSqr,
                                txn_amount.get_amount_as_string()
                            ))?,
                        }))
                    }
                    RealTimePaymentData::Fps {}
                    | RealTimePaymentData::PromptPay {}
                    | RealTimePaymentData::VietQr {} => Err(
                        errors::ConnectorError::NotImplemented("Payment methods".to_string())
                            .into(),
                    ),
                }
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CardPaymentsResponse {
    pub reference_no: String,
    #[serde(rename = "TxnID")]
    pub txn_id: String,
    pub txn_type: String,
    pub txn_currency: String,
    pub txn_amount: String,
    pub txn_channel: String,
    pub txn_data: TxnData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FiuuPaymentsResponse {
    CardPaymentResponse(Box<CardPaymentsResponse>),
    QRPaymentResponse(Box<DuitNowQrCodeResponse>),
    Error(FiuuErrorResponse),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TxnData {
    #[serde(rename = "RequestURL")]
    pub request_url: String,
    pub request_type: RequestType,
    pub request_data: RequestData,
    pub request_method: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RequestType {
    Redirect,
    Response,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreeDSResponseData {
    #[serde(rename = "paRes")]
    pa_res: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestData {
    NonThreeDS(NonThreeDSResponseData),
    ThreeDS(ThreeDSResponseData),
}
#[derive(Debug, Serialize, Deserialize)]
pub struct NonThreeDSResponseData {
    #[serde(rename = "tranID")]
    pub tran_id: String,
    pub status: String,
    pub error_code: Option<String>,
    pub error_desc: Option<String>,
}

impl<F>
    TryFrom<
        ResponseRouterData<F, FiuuPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            FiuuPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            FiuuPaymentsResponse::QRPaymentResponse(response) => Ok(Self {
                status: match response.status {
                    false => enums::AttemptStatus::Failure,
                    true => enums::AttemptStatus::AuthenticationPending,
                },
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: get_qr_metadata(&response)?,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
            FiuuPaymentsResponse::Error(error) => Ok(Self {
                response: Err(ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_desc.clone(),
                    reason: Some(error.error_desc),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            }),
            FiuuPaymentsResponse::CardPaymentResponse(data) => match data.txn_data.request_data {
                RequestData::ThreeDS(three_ds_data) => {
                    let form_fields = {
                        let mut map = HashMap::new();
                        map.insert("paRes".to_string(), three_ds_data.pa_res.clone());
                        map
                    };
                    let redirection_data = Some(RedirectForm::Form {
                        endpoint: data.txn_data.request_url.to_string(),
                        method: if data.txn_data.request_method.as_str() == "POST" {
                            Method::Post
                        } else {
                            Method::Get
                        },
                        form_fields,
                    });
                    Ok(Self {
                        status: enums::AttemptStatus::AuthenticationPending,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(data.txn_id),
                            redirection_data,
                            mandate_reference: None,
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: None,
                            incremental_authorization_allowed: None,
                            charge_id: None,
                        }),
                        ..item.data
                    })
                }

                RequestData::NonThreeDS(non_threeds_data) => {
                    let status = match non_threeds_data.status.as_str() {
                        "00" => {
                            if item.data.request.is_auto_capture()? {
                                Ok(enums::AttemptStatus::Charged)
                            } else {
                                Ok(enums::AttemptStatus::Authorized)
                            }
                        }
                        "11" => Ok(enums::AttemptStatus::Failure),
                        "22" => Ok(enums::AttemptStatus::Pending),
                        other => Err(errors::ConnectorError::UnexpectedResponseError(
                            bytes::Bytes::from(other.to_owned()),
                        )),
                    }?;
                    let response = if status == enums::AttemptStatus::Failure {
                        Err(ErrorResponse {
                            code: non_threeds_data
                                .error_code
                                .clone()
                                .unwrap_or_else(|| "NO_ERROR_CODE".to_string()),
                            message: non_threeds_data
                                .error_desc
                                .clone()
                                .unwrap_or_else(|| "NO_ERROR_MESSAGE".to_string()),
                            reason: non_threeds_data.error_desc.clone(),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: None,
                        })
                    } else {
                        Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(data.txn_id),
                            redirection_data: None,
                            mandate_reference: None,
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: None,
                            incremental_authorization_allowed: None,
                            charge_id: None,
                        })
                    };
                    Ok(Self {
                        status,
                        response,
                        ..item.data
                    })
                }
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuRefundRequest {
    pub refund_type: RefundType,
    #[serde(rename = "MerchantID")]
    pub merchant_id: Secret<String>,
    #[serde(rename = "RefID")]
    pub ref_id: String,
    #[serde(rename = "TxnID")]
    pub txn_id: String,
    pub amount: StringMajorUnit,
    pub signature: Secret<String>,
}
#[derive(Debug, Serialize, Display)]
pub enum RefundType {
    #[serde(rename = "P")]
    #[strum(serialize = "P")]
    Partial,
}

impl TryFrom<&FiuuRouterData<&RefundsRouterData<Execute>>> for FiuuRefundRequest {
    type Error = Report<errors::ConnectorError>;
    fn try_from(item: &FiuuRouterData<&RefundsRouterData<Execute>>) -> Result<Self, Self::Error> {
        let auth: FiuuAuthType = FiuuAuthType::try_from(&item.router_data.connector_auth_type)?;
        let merchant_id = auth.merchant_id.peek().to_string();
        let txn_amount = item.amount.clone();
        let reference_no = item.router_data.connector_request_reference_id.clone();
        let txn_id = item.router_data.request.connector_transaction_id.clone();
        let secret_key = auth.secret_key.peek().to_string();
        Ok(Self {
            refund_type: RefundType::Partial,
            merchant_id: auth.merchant_id,
            ref_id: reference_no.clone(),
            txn_id: txn_id.clone(),
            amount: txn_amount.clone(),
            signature: calculate_signature(format!(
                "{}{merchant_id}{reference_no}{txn_id}{}{secret_key}",
                RefundType::Partial,
                txn_amount.get_amount_as_string()
            ))?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuRefundSuccessResponse {
    #[serde(rename = "RefundID")]
    refund_id: i64,
    status: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FiuuRefundResponse {
    Success(FiuuRefundSuccessResponse),
    Error(FiuuErrorResponse),
}
impl TryFrom<RefundsResponseRouterData<Execute, FiuuRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, FiuuRefundResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            FiuuRefundResponse::Error(error) => Ok(Self {
                response: Err(ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_desc.clone(),
                    reason: Some(error.error_desc),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            }),
            FiuuRefundResponse::Success(refund_data) => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: refund_data.refund_id.to_string(),
                    refund_status: match refund_data.status.as_str() {
                        "00" => Ok(enums::RefundStatus::Success),
                        "11" => Ok(enums::RefundStatus::Failure),
                        "22" => Ok(enums::RefundStatus::Pending),
                        other => Err(errors::ConnectorError::UnexpectedResponseError(
                            bytes::Bytes::from(other.to_owned()),
                        )),
                    }?,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FiuuErrorResponse {
    pub error_code: String,
    pub error_desc: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FiuuPaymentSyncRequest {
    amount: StringMajorUnit,
    #[serde(rename = "txID")]
    tx_id: String,
    domain: String,
    skey: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuPaymentSyncResponse {
    stat_code: String,
    stat_name: StatName,
    #[serde(rename = "TranID")]
    tran_id: String,
    error_code: String,
    error_desc: String,
    #[serde(rename = "miscellaneous")]
    miscellaneous: Option<HashMap<String, Secret<String>>>,
}

#[derive(Debug, Serialize, Deserialize, Display, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StatName {
    Captured,
    Settled,
    Authorized,
    Failed,
    Cancelled,
    Chargeback,
    Release,
    #[serde(rename = "reject/hold")]
    RejectHold,
    Blocked,
    #[serde(rename = "ReqCancel")]
    ReqCancel,
    #[serde(rename = "ReqChargeback")]
    ReqChargeback,
    #[serde(rename = "Pending")]
    Pending,
    #[serde(rename = "Unknown")]
    Unknown,
}
impl TryFrom<&PaymentsSyncRouterData> for FiuuPaymentSyncRequest {
    type Error = Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = FiuuAuthType::try_from(&item.connector_auth_type)?;
        let txn_id = item
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        let merchant_id = auth.merchant_id.peek().to_string();
        let verify_key = auth.verify_key.peek().to_string();
        let amount = StringMajorUnitForConnector
            .convert(item.request.amount, item.request.currency)
            .change_context(errors::ConnectorError::AmountConversionFailed)?;
        Ok(Self {
            amount: amount.clone(),
            tx_id: txn_id.clone(),
            domain: merchant_id.clone(),
            skey: calculate_signature(format!(
                "{txn_id}{merchant_id}{verify_key}{}",
                amount.get_amount_as_string()
            ))?,
        })
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<FiuuPaymentSyncResponse>> for PaymentsSyncRouterData {
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSyncResponseRouterData<FiuuPaymentSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let stat_name = item.response.stat_name;
        let status = match item.response.stat_code.as_str() {
            "00" => {
                if stat_name == StatName::Captured || stat_name == StatName::Settled {
                    Ok(enums::AttemptStatus::Charged)
                } else {
                    Ok(enums::AttemptStatus::Authorized)
                }
            }
            "22" => Ok(enums::AttemptStatus::Pending),
            "11" => Ok(enums::AttemptStatus::Failure),
            other => Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(other.to_owned()),
            )),
        }?;
        let error_response = if status == enums::AttemptStatus::Failure {
            Some(ErrorResponse {
                status_code: item.http_code,
                code: item.response.stat_code.as_str().to_owned(),
                message: item.response.stat_name.clone().to_string(),
                reason: Some(item.response.stat_name.clone().to_string()),
                attempt_status: Some(enums::AttemptStatus::Failure),
                connector_transaction_id: None,
            })
        } else {
            None
        };
        let payments_response_data = PaymentsResponseData::TransactionResponse {
            resource_id: item.data.request.connector_transaction_id.clone(),
            redirection_data: None,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charge_id: None,
        };
        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response_data), Err),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PaymentCaptureRequest {
    domain: String,
    #[serde(rename = "tranID")]
    tran_id: String,
    amount: StringMajorUnit,
    #[serde(rename = "RefID")]
    ref_id: String,
    skey: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PaymentCaptureResponse {
    #[serde(rename = "TranID")]
    tran_id: String,
    stat_code: String,
}

impl TryFrom<&FiuuRouterData<&PaymentsCaptureRouterData>> for PaymentCaptureRequest {
    type Error = Report<errors::ConnectorError>;
    fn try_from(item: &FiuuRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth = FiuuAuthType::try_from(&item.router_data.connector_auth_type)?;
        let merchant_id = auth.merchant_id.peek().to_string();
        let amount = item.amount.clone();
        let txn_id = item.router_data.request.connector_transaction_id.clone();
        let verify_key = auth.verify_key.peek().to_string();
        let signature = calculate_signature(format!(
            "{txn_id}{}{merchant_id}{verify_key}",
            amount.get_amount_as_string()
        ))?;
        Ok(Self {
            domain: merchant_id,
            tran_id: txn_id,
            amount,
            ref_id: item.router_data.connector_request_reference_id.clone(),
            skey: signature,
        })
    }
}
fn capture_status_codes() -> HashMap<&'static str, &'static str> {
    [
        ("00", "Capture successful"),
        ("11", "Capture failed"),
        ("12", "Invalid or unmatched security hash string"),
        ("13", "Not a credit card transaction"),
        ("15", "Requested day is on settlement day"),
        ("16", "Forbidden transaction"),
        ("17", "Transaction not found"),
        ("18", "Missing required parameter"),
        ("19", "Domain not found"),
        ("20", "Temporary out of service"),
        ("21", "Authorization expired"),
        ("23", "Partial capture not allowed"),
        ("24", "Transaction already captured"),
        ("25", "Requested amount exceeds available capture amount"),
        ("99", "General error (contact payment gateway support)"),
    ]
    .into_iter()
    .collect()
}

impl TryFrom<PaymentsCaptureResponseRouterData<PaymentCaptureResponse>>
    for PaymentsCaptureRouterData
{
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<PaymentCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let status_code = item.response.stat_code;

        let status = match status_code.as_str() {
            "00" => Ok(enums::AttemptStatus::Charged),
            "22" => Ok(enums::AttemptStatus::Pending),
            "11" | "12" | "13" | "15" | "16" | "17" | "18" | "19" | "20" | "21" | "23" | "24"
            | "25" | "99" => Ok(enums::AttemptStatus::Failure),
            other => Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(other.to_owned()),
            )),
        }?;
        let capture_message_status = capture_status_codes();
        let error_response = if status == enums::AttemptStatus::Failure {
            Some(ErrorResponse {
                status_code: item.http_code,
                code: status_code.to_owned(),
                message: capture_message_status
                    .get(status_code.as_str())
                    .unwrap_or(&"NO_ERROR_MESSAGE")
                    .to_string(),
                reason: Some(
                    capture_message_status
                        .get(status_code.as_str())
                        .unwrap_or(&"NO_ERROR_REASON")
                        .to_string(),
                ),
                attempt_status: None,
                connector_transaction_id: None,
            })
        } else {
            None
        };
        let payments_response_data = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.tran_id.to_string()),
            redirection_data: None,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charge_id: None,
        };
        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response_data), Err),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FiuuPaymentCancelRequest {
    #[serde(rename = "txnID")]
    txn_id: String,
    domain: String,
    skey: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuPaymentCancelResponse {
    #[serde(rename = "TranID")]
    tran_id: String,
    stat_code: String,
    #[serde(rename = "miscellaneous")]
    miscellaneous: Option<HashMap<String, Secret<String>>>,
}

impl TryFrom<&PaymentsCancelRouterData> for FiuuPaymentCancelRequest {
    type Error = Report<errors::ConnectorError>;

    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = FiuuAuthType::try_from(&item.connector_auth_type)?;
        let txn_id = item.request.connector_transaction_id.clone();
        let merchant_id = auth.merchant_id.peek().to_string();
        let secret_key = auth.secret_key.peek().to_string();
        Ok(Self {
            txn_id: txn_id.clone(),
            domain: merchant_id.clone(),
            skey: calculate_signature(format!("{txn_id}{merchant_id}{secret_key}"))?,
        })
    }
}

fn void_status_codes() -> HashMap<&'static str, &'static str> {
    [
        ("00", "Success (will proceed the request)"),
        ("11", "Failure"),
        ("12", "Invalid or unmatched security hash string"),
        ("13", "Not a refundable transaction"),
        ("14", "Transaction date more than 180 days"),
        ("15", "Requested day is on settlement day"),
        ("16", "Forbidden transaction"),
        ("17", "Transaction not found"),
        ("18", "Duplicate partial refund request"),
        ("19", "Merchant not found"),
        ("20", "Missing required parameter"),
        (
            "21",
            "Transaction must be in authorized/captured/settled status",
        ),
    ]
    .into_iter()
    .collect()
}
impl TryFrom<PaymentsCancelResponseRouterData<FiuuPaymentCancelResponse>>
    for PaymentsCancelRouterData
{
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<FiuuPaymentCancelResponse>,
    ) -> Result<Self, Self::Error> {
        let status_code = item.response.stat_code;
        let status = match status_code.as_str() {
            "00" => Ok(enums::AttemptStatus::Voided),
            "11" | "12" | "13" | "14" | "15" | "16" | "17" | "18" | "19" | "20" | "21" => {
                Ok(enums::AttemptStatus::VoidFailed)
            }
            other => Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(other.to_owned()),
            )),
        }?;
        let void_message_status = void_status_codes();
        let error_response = if status == enums::AttemptStatus::VoidFailed {
            Some(ErrorResponse {
                status_code: item.http_code,
                code: status_code.to_owned(),
                message: void_message_status
                    .get(status_code.as_str())
                    .unwrap_or(&"NO_ERROR_MESSAGE")
                    .to_string(),
                reason: Some(
                    void_message_status
                        .get(status_code.as_str())
                        .unwrap_or(&"NO_ERROR_REASON")
                        .to_string(),
                ),
                attempt_status: None,
                connector_transaction_id: None,
            })
        } else {
            None
        };
        let payments_response_data = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.tran_id.to_string()),
            redirection_data: None,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charge_id: None,
        };
        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response_data), Err),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuRefundSyncRequest {
    #[serde(rename = "TxnID")]
    txn_id: String,
    #[serde(rename = "MerchantID")]
    merchant_id: Secret<String>,
    signature: Secret<String>,
}

impl TryFrom<&RefundSyncRouterData> for FiuuRefundSyncRequest {
    type Error = Report<errors::ConnectorError>;

    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth = FiuuAuthType::try_from(&item.connector_auth_type)?;
        let (txn_id, merchant_id, verify_key) = (
            item.request.connector_transaction_id.clone(),
            auth.merchant_id.peek().to_string(),
            auth.verify_key.peek().to_string(),
        );
        let signature = calculate_signature(format!("{txn_id}{merchant_id}{verify_key}"))?;
        Ok(Self {
            txn_id,
            merchant_id: auth.merchant_id,
            signature,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FiuuRefundSyncResponse {
    Success(Vec<RefundData>),
    Error(FiuuErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefundData {
    #[serde(rename = "RefundID")]
    refund_id: String,
    status: RefundStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RefundStatus {
    Success,
    Pending,
    Rejected,
    Processing,
}

impl TryFrom<RefundsResponseRouterData<RSync, FiuuRefundSyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = Report<errors::ConnectorError>;

    fn try_from(
        item: RefundsResponseRouterData<RSync, FiuuRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            FiuuRefundSyncResponse::Error(error) => Ok(Self {
                response: Err(ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_desc.clone(),
                    reason: Some(error.error_desc),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            }),
            FiuuRefundSyncResponse::Success(refund_data) => {
                let refund = refund_data
                    .iter()
                    .find(|refund| {
                        Some(refund.refund_id.clone()) == item.data.request.connector_refund_id
                    })
                    .ok_or_else(|| errors::ConnectorError::MissingConnectorRefundID)?;
                Ok(Self {
                    response: Ok(RefundsResponseData {
                        connector_refund_id: refund.refund_id.clone(),
                        refund_status: enums::RefundStatus::from(refund.status.clone()),
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Pending => Self::Pending,
            RefundStatus::Success => Self::Success,
            RefundStatus::Rejected => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DuitNowQrCodeResponse {
    status: bool,
    qrcode_data: Secret<String>,
}

pub fn get_qr_metadata(
    response: &DuitNowQrCodeResponse,
) -> CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let image_data = QrImage::new_from_data(response.qrcode_data.peek().clone())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let image_data_url = Url::parse(image_data.data.clone().as_str()).ok();
    let display_to_timestamp = None;

    if let Some(image_data_url) = image_data_url {
        let qr_code_info = payments::QrCodeInformation::QrDataUrl {
            image_data_url,
            display_to_timestamp,
        };

        Some(qr_code_info.encode_to_value())
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    } else {
        Ok(None)
    }
}
