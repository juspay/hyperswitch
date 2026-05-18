use common_enums::enums;
use common_utils::{request::Method, types::StringMajorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, PaymentMethodData},
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{AccessTokenRequestData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

pub struct BidvRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for BidvRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// Account type configured per connector instance
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BidvAccountType {
    Business,
    Personal,
}

impl BidvAccountType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "personal" => Self::Personal,
            _ => Self::Business,
        }
    }
}

// Auth Struct — SignatureKey (merchant fills in at connector setup time):
//   api_key    = client_id           (OAuth2 client_id)
//   api_secret = client_secret       (OAuth2 client_secret)
//   key1       = client_certificate  (X-Client-Certificate header value, issued by BIDV)
pub struct BidvAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
    pub(super) client_certificate: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BidvAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                client_id: api_key.to_owned(),
                client_secret: api_secret.to_owned(),
                client_certificate: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// OAuth2 Client Credentials token request (form-encoded)
// Sent with Authorization: Basic base64(client_id:client_secret) header
#[derive(Debug, Serialize)]
pub struct BidvTokenRequest {
    grant_type: String,
    scope: String,
}

impl TryFrom<&types::RefreshTokenRouterData> for BidvTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_req: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            grant_type: "client_credentials".to_owned(),
            scope: "ewallet".to_owned(),
        })
    }
}

// OAuth2 token response
#[derive(Debug, Deserialize, Serialize)]
pub struct BidvTokenResponse {
    pub access_token: Secret<String>,
    pub expires_in: i64,
    pub token_type: String,
    pub scope: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, BidvTokenResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BidvTokenResponse, T, AccessToken>,
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

// ---- Business (Corp) payment request ----
// POST /open-banking/paygate/inittranscorpgw/v1
#[derive(Debug, Serialize)]
pub struct BidvCorpPaymentsRequest {
    pub body: BidvCorpPaymentsRequestBody,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BidvCorpPaymentsRequestBody {
    pub service_id: String,
    pub merchant_id: String,
    pub merchant_name: String,
    pub channel_id: String,
    pub root_request_id: String,
    pub root_request_date: String,
    #[serde(rename = "appAcct", skip_serializing_if = "Option::is_none")]
    pub app_acct: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_info1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_info2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_info3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_info4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_info5: Option<String>,
}

impl TryFrom<&BidvRouterData<&types::PaymentsAuthorizeRouterData>> for BidvCorpPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BidvRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let metadata = item.router_data.request.metadata.as_ref();
        let merchant_id = metadata
            .and_then(|m| m.get("merchant_id"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "metadata.merchant_id",
            })?;
        let merchant_name = metadata
            .and_then(|m| m.get("merchant_name"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "metadata.merchant_name",
            })?;
        let channel_id = metadata
            .and_then(|m| m.get("channel_id"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "metadata.channel_id",
            })?;
        let service_id = metadata
            .and_then(|m| m.get("service_id"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| "000003".to_owned());
        let now = OffsetDateTime::now_utc();
        let root_request_date = format!("{:02}{:02}{:02}", now.year() % 100, now.month() as u8, now.day());
        Ok(Self {
            body: BidvCorpPaymentsRequestBody {
                service_id,
                merchant_id,
                merchant_name,
                channel_id,
                root_request_id: item.router_data.connector_request_reference_id.clone(),
                root_request_date,
                app_acct: metadata.and_then(|m| m.get("account_number")).and_then(|v| v.as_str()).map(str::to_owned),
                extra_info1: metadata.and_then(|m| m.get("extra_info1")).and_then(|v| v.as_str()).map(str::to_owned),
                extra_info2: metadata.and_then(|m| m.get("extra_info2")).and_then(|v| v.as_str()).map(str::to_owned),
                extra_info3: metadata.and_then(|m| m.get("extra_info3")).and_then(|v| v.as_str()).map(str::to_owned),
                extra_info4: metadata.and_then(|m| m.get("extra_info4")).and_then(|v| v.as_str()).map(str::to_owned),
                extra_info5: metadata.and_then(|m| m.get("extra_info5")).and_then(|v| v.as_str()).map(str::to_owned),
            },
        })
    }
}

// Corp payment creation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidvCorpPaymentsResponse {
    pub msg: BidvCorpMsg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidvCorpMsg {
    pub header: BidvCorpMsgHeader,
    pub body: BidvCorpMsgBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidvCorpMsgHeader {
    pub request_id: Option<i64>,
    pub error_code: String,
    pub error_desc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidvCorpMsgBody {
    pub tran_date: Option<String>,
    pub bank_trans_id: Option<String>,
    pub redirect_url: Option<String>,
    pub extra_info1: Option<String>,
    pub extra_info2: Option<String>,
    pub extra_info3: Option<String>,
    pub extra_info4: Option<String>,
    pub extra_info5: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, BidvCorpPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BidvCorpPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let header = &item.response.msg.header;
        if header.error_code != "000" {
            return Ok(Self {
                response: Err(ErrorResponse {
                    status_code: item.http_code,
                    code: header.error_code.clone(),
                    message: header.error_desc.clone(),
                    reason: Some(header.error_desc.clone()),
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            });
        }
        let body = &item.response.msg.body;
        let trans_id = body.bank_trans_id.clone().unwrap_or_default();
        let redirect_url = body.redirect_url.clone();
        let redirection_data = redirect_url
            .as_deref()
            .map(|url| RedirectForm::Form {
                endpoint: url.to_owned(),
                method: Method::Get,
                form_fields: HashMap::new(),
            });
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(trans_id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(trans_id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// ---- Personal (eWallet) payment request ----
// POST /open-banking/ewallet/init-tran/v1
#[derive(Debug, Serialize)]
pub struct BidvEwalletPaymentsRequest {
    pub body: BidvEwalletPaymentsRequestBody,
}

#[derive(Debug, Serialize)]
pub struct BidvEwalletPaymentsRequestBody {
    #[serde(rename = "serviceId")]
    pub service_id: String,
    #[serde(rename = "merchantId")]
    pub merchant_id: String,
    #[serde(rename = "merchantName")]
    pub merchant_name: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
    #[serde(rename = "Trandate")]
    pub tran_date: String,
    #[serde(rename = "Trans_Id")]
    pub trans_id: String,
    #[serde(rename = "Trans_Desc")]
    pub trans_desc: String,
    #[serde(rename = "Amount")]
    pub amount: StringMajorUnit,
    #[serde(rename = "Curr")]
    pub curr: String,
    #[serde(rename = "Payer_Id")]
    pub payer_id: Secret<String>,
    #[serde(rename = "Payer_Name")]
    pub payer_name: Secret<String>,
    #[serde(rename = "Payer_Addr", skip_serializing_if = "Option::is_none")]
    pub payer_addr: Option<String>,
    #[serde(rename = "Type")]
    pub transaction_type: String,
    #[serde(rename = "Custmer_Id", skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    #[serde(rename = "Customer_Name", skip_serializing_if = "Option::is_none")]
    pub customer_name: Option<String>,
    #[serde(rename = "IssueDate", skip_serializing_if = "Option::is_none")]
    pub issue_date: Option<String>,
    #[serde(rename = "App_Acct", skip_serializing_if = "Option::is_none")]
    pub app_acct: Option<String>,
}

impl TryFrom<&BidvRouterData<&types::PaymentsAuthorizeRouterData>>
    for BidvEwalletPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BidvRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let metadata = item.router_data.request.metadata.as_ref();
        let merchant_id = metadata
            .and_then(|m| m.get("merchant_id"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "metadata.merchant_id",
            })?;
        let merchant_name = metadata
            .and_then(|m| m.get("merchant_name"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "metadata.merchant_name",
            })?;
        let channel_id = metadata
            .and_then(|m| m.get("channel_id"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "metadata.channel_id",
            })?;
        let service_id = metadata
            .and_then(|m| m.get("service_id"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| "000003".to_owned());
        let transaction_type = metadata
            .and_then(|m| m.get("transaction_type"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| "809".to_owned());
        let payer_id = metadata
            .and_then(|m| m.get("payer_id"))
            .and_then(|v| v.as_str())
            .map(|s| Secret::new(s.to_owned()))
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "metadata.payer_id",
            })?;
        let payer_name = item
            .router_data
            .request
            .customer_name
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "customer_name",
            })?;
        let now = OffsetDateTime::now_utc();
        let tran_date = format!("{:02}{:02}{:02}", now.year() % 100, now.month() as u8, now.day());
        Ok(Self {
            body: BidvEwalletPaymentsRequestBody {
                service_id,
                merchant_id,
                merchant_name,
                channel_id,
                tran_date,
                trans_id: item.router_data.connector_request_reference_id.clone(),
                trans_desc: item.router_data.request.statement_descriptor.clone().unwrap_or_default(),
                amount: item.amount.clone(),
                curr: item.router_data.request.currency.to_string(),
                payer_id,
                payer_name,
                payer_addr: metadata.and_then(|m| m.get("payer_addr")).and_then(|v| v.as_str()).map(str::to_owned),
                transaction_type,
                customer_id: metadata.and_then(|m| m.get("customer_id")).and_then(|v| v.as_str()).map(str::to_owned),
                customer_name: metadata.and_then(|m| m.get("customer_name")).and_then(|v| v.as_str()).map(str::to_owned),
                issue_date: metadata.and_then(|m| m.get("issue_date")).and_then(|v| v.as_str()).map(str::to_owned),
                app_acct: metadata.and_then(|m| m.get("account_number")).and_then(|v| v.as_str()).map(str::to_owned),
            },
        })
    }
}

// eWallet payment creation response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidvEwalletPaymentsResponse {
    pub body: Option<BidvEwalletResponseBody>,
    pub error_response: Option<BidvEwalletErrorResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidvEwalletResponseBody {
    pub service_id: Option<String>,
    pub merchant_id: Option<String>,
    pub tran_date: Option<String>,
    pub error_code: Option<String>,
    pub error_desc: Option<String>,
    pub redirect_url: Option<String>,
    pub extra_info1: Option<String>,
    pub extra_info2: Option<String>,
    pub extra_info3: Option<String>,
    pub extra_info4: Option<String>,
    pub extra_info5: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidvEwalletErrorResponse {
    pub metadata: Option<BidvEwalletErrorMetadata>,
    pub additional_info: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidvEwalletErrorMetadata {
    pub status: Option<BidvEwalletErrorStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidvEwalletErrorStatus {
    pub code: Option<String>,
    pub desc: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, BidvEwalletPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BidvEwalletPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Check for top-level error_response
        if let Some(err) = &item.response.error_response {
            let (code, message) = err
                .metadata
                .as_ref()
                .and_then(|m| m.status.as_ref())
                .map(|s| (
                    s.code.clone().unwrap_or_default(),
                    s.desc.clone().unwrap_or_default(),
                ))
                .unwrap_or_default();
            if !code.is_empty() {
                return Ok(Self {
                    response: Err(ErrorResponse {
                        status_code: item.http_code,
                        code,
                        message: message.clone(),
                        reason: Some(message),
                        attempt_status: None,
                        connector_transaction_id: None,
                        connector_response_reference_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                });
            }
        }

        let body = item
            .response
            .body
            .as_ref()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

        // Non-zero error_code in body means failure
        let error_code = body.error_code.as_deref().unwrap_or("0");
        if error_code != "0" && !error_code.is_empty() {
            let message = body.error_desc.clone().unwrap_or_default();
            return Ok(Self {
                response: Err(ErrorResponse {
                    status_code: item.http_code,
                    code: error_code.to_owned(),
                    message: message.clone(),
                    reason: Some(message),
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            });
        }

        let trans_id = body.service_id.clone().unwrap_or_default();
        let redirect_url = body.redirect_url.clone();
        let redirection_data = redirect_url
            .as_deref()
            .map(|url| RedirectForm::Form {
                endpoint: url.to_owned(),
                method: Method::Get,
                form_fields: HashMap::new(),
            });
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(trans_id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(trans_id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// Payment sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidvPaymentsSyncResponse {
    pub response_code: String,
    pub response_message: String,
    pub status: Option<BidvPaymentStatus>,
    pub transaction_id: Option<String>,
    pub order_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BidvPaymentStatus {
    Pending,
    Paid,
    Failed,
    Cancelled,
    Expired,
}

impl From<BidvPaymentStatus> for enums::AttemptStatus {
    fn from(item: BidvPaymentStatus) -> Self {
        match item {
            BidvPaymentStatus::Paid => Self::Charged,
            BidvPaymentStatus::Failed | BidvPaymentStatus::Expired => Self::Failure,
            BidvPaymentStatus::Cancelled => Self::Voided,
            BidvPaymentStatus::Pending => Self::AuthenticationPending,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, BidvPaymentsSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BidvPaymentsSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = item
            .response
            .transaction_id
            .clone()
            .unwrap_or_default();
        let status = item
            .response
            .status
            .clone()
            .map(enums::AttemptStatus::from)
            .unwrap_or(enums::AttemptStatus::AuthenticationPending);
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// Refund request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BidvRefundRequest {
    pub transaction_id: String,
    pub refund_amount: StringMajorUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_description: Option<String>,
}

impl<F> TryFrom<&BidvRouterData<&types::RefundsRouterData<F>>> for BidvRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BidvRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_id: item.router_data.request.connector_transaction_id.clone(),
            refund_amount: item.amount.to_owned(),
            refund_description: item.router_data.request.reason.clone(),
        })
    }
}

// Refund response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    pub response_code: String,
    pub response_message: String,
    pub refund_transaction_id: Option<String>,
    pub status: Option<BidvRefundStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BidvRefundStatus {
    Success,
    Failed,
    Pending,
}

impl From<BidvRefundStatus> for enums::RefundStatus {
    fn from(item: BidvRefundStatus) -> Self {
        match item {
            BidvRefundStatus::Success => Self::Success,
            BidvRefundStatus::Failed => Self::Failure,
            BidvRefundStatus::Pending => Self::Pending,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = item
            .response
            .status
            .clone()
            .map(enums::RefundStatus::from)
            .unwrap_or(enums::RefundStatus::Pending);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item
                    .response
                    .refund_transaction_id
                    .unwrap_or_else(|| item.response.response_code.clone()),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = item
            .response
            .status
            .clone()
            .map(enums::RefundStatus::from)
            .unwrap_or(enums::RefundStatus::Pending);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item
                    .response
                    .refund_transaction_id
                    .unwrap_or_else(|| item.response.response_code.clone()),
                refund_status,
            }),
            ..item.data
        })
    }
}

// Error response
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BidvErrorResponse {
    pub response_code: String,
    pub response_message: String,
    pub error_detail: Option<String>,
}
