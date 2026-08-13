use std::collections::HashMap;

use common_enums::{enums, Currency};
use common_utils::{
    crypto::{self, GenerateDigest},
    date_time,
    request::Method,
    types::StringMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, PaymentMethodData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsSyncRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{consts, errors};
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, RouterData as _},
};

/// TDayPay service name header (documented as `api.pay`).
pub const SERVICE_NAME: &str = "api.pay";
/// TDayPay sign algorithm header.
pub const SIGN_TYPE: &str = "SHA512";
/// Successful API `resultCode`.
pub const SUCCESS_RESULT_CODE: &str = "000000";

pub struct TdaypayRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for TdaypayRouterData<T> {
    fn from((amount, router_data): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

/// BodyKey credentials:
/// - `api_key` = merchant secret key (used only for SHA512 signing)
/// - `key1` = mchId
pub struct TdaypayAuthType {
    pub(super) merchant_key: Secret<String>,
    pub(super) mch_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for TdaypayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                merchant_key: api_key.to_owned(),
                mch_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

/// Request sign: SHA512(mchId + serviceName + method + timestamp + signType + rawBody + merchantKey)
pub fn sign_request(
    mch_id: &str,
    method: &str,
    timestamp: &str,
    raw_body: &str,
    merchant_key: &str,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let payload =
        format!("{mch_id}{SERVICE_NAME}{method}{timestamp}{SIGN_TYPE}{raw_body}{merchant_key}");
    let digest = crypto::Sha512
        .generate_digest(payload.as_bytes())
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
    Ok(hex::encode(digest))
}

pub fn current_timestamp() -> String {
    date_time::now_unix_timestamp().to_string()
}

/// mchOrderId limits: 5–32 alphanumeric characters.
fn sanitize_mch_order_id(reference: &str) -> String {
    let cleaned: String = reference
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let id = if cleaned.len() >= 5 {
        cleaned
    } else {
        format!("ord{cleaned}{}", date_time::now_unix_timestamp())
    };
    id.chars().take(32).collect()
}

/// Optional string from payment metadata (no currency allowlist).
/// Supports keys: `tdaypay_payment_type`, `payment_type`, `paymentType`.
fn metadata_string(metadata: Option<&serde_json::Value>, keys: &[&str]) -> Option<String> {
    let meta = metadata?;
    let obj = meta.as_object()?;
    for key in keys {
        if let Some(v) = obj.get(*key) {
            if let Some(s) = v.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

/// Map Hyperswitch bank_transfer PMT → TDayPay `paymentType` (PM-driven, not currency list).
/// Override anytime via payment metadata without recompiling.
fn resolve_payment_type(
    bank: &BankTransferData,
    metadata: Option<&serde_json::Value>,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    if let Some(explicit) = metadata_string(
        metadata,
        &["tdaypay_payment_type", "payment_type", "paymentType"],
    ) {
        return Ok(explicit.to_uppercase());
    }

    match bank {
        BankTransferData::Pix { .. } | BankTransferData::PixEmv {} | BankTransferData::PixQr {} => {
            Ok("PIX".to_string())
        }
        BankTransferData::Pse {} => Ok("PSE".to_string()),
        BankTransferData::LocalBankTransfer { bank_code } => {
            // Prefer bank_code when it carries the gateway paymentType (SPEI, CASH, TRANSFER, …)
            if let Some(code) = bank_code
                .as_ref()
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
            {
                Ok(code.to_uppercase())
            } else {
                // Generic default — override via metadata `payment_type` / `tdaypay_payment_type`
                // so new TDayPay markets work without recompiling this connector.
                Ok("TRANSFER".to_string())
            }
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("tdaypay"),
        )
        .into()),
    }
}

/// Pay request body fields (camelCase as used by TDayPay gateway).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TdaypayPaymentsRequest {
    mch_order_id: String,
    amount: StringMajorUnit,
    currency: String,
    productinfo: String,
    firstname: String,
    lastname: String,
    email: String,
    phone: String,
    callback_url: String,
    redirect_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    beneficiary_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    beneficiary_id: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    doc_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    doc_number: Option<Secret<String>>,
}

impl TryFrom<&TdaypayRouterData<&PaymentsAuthorizeRouterData>> for TdaypayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TdaypayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let req = item.router_data;
        // Currency is forwarded as-is — no hardcoded allowlist (MCA/profile controls availability).
        let currency = req.request.currency;

        let firstname = req
            .get_optional_billing_first_name()
            .map(|s| s.expose())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Customer".to_string());
        let lastname = req
            .get_optional_billing_last_name()
            .map(|s| s.expose())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "User".to_string());
        let email = req
            .get_optional_billing_email()
            .map(|e| e.expose().expose())
            .or_else(|| req.request.email.as_ref().map(|e| e.peek().to_string()))
            .unwrap_or_else(|| "customer@example.com".to_string());
        let phone = req
            .get_optional_billing_phone_number()
            .map(|p| p.expose())
            .unwrap_or_else(|| "+10000000000".to_string());

        let bank = match &req.request.payment_method_data {
            PaymentMethodData::BankTransfer(bank) => bank.as_ref(),
            _ => {
                return Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("tdaypay"),
                )
                .into());
            }
        };

        let payment_type = resolve_payment_type(bank, req.request.metadata.as_ref())?;

        // Optional PIX tax document from payment method data.
        let (doc_type, doc_number) = match bank {
            BankTransferData::Pix { cpf, cnpj, .. } => {
                if let Some(cpf) = cpf {
                    (Some("CPF".to_string()), Some(cpf.clone()))
                } else if let Some(cnpj) = cnpj {
                    (Some("CNPJ".to_string()), Some(cnpj.clone()))
                } else {
                    (None, None)
                }
            }
            _ => (None, None),
        };

        // Optional beneficiary fields (e.g. CASH/RUT) via metadata — not currency-locked.
        let beneficiary_type = metadata_string(
            req.request.metadata.as_ref(),
            &[
                "tdaypay_beneficiary_type",
                "beneficiary_type",
                "beneficiaryType",
            ],
        );
        let beneficiary_id = metadata_string(
            req.request.metadata.as_ref(),
            &["tdaypay_beneficiary_id", "beneficiary_id", "beneficiaryId"],
        )
        .map(Secret::new);

        // When paymentType is CASH and no beneficiaryType was provided, default to RUT
        // (documented for Chile CASH flows) without locking the currency list.
        let beneficiary_type = beneficiary_type.or_else(|| {
            if payment_type.eq_ignore_ascii_case("CASH") {
                Some("RUT".to_string())
            } else {
                None
            }
        });

        let payment_type = Some(payment_type);

        let callback_url = req.request.get_webhook_url().unwrap_or_else(|_| {
            req.request
                .router_return_url
                .clone()
                .unwrap_or_else(|| "https://example.com/tdaypay/callback".to_string())
        });
        let redirect_url = req
            .request
            .router_return_url
            .clone()
            .unwrap_or_else(|| callback_url.clone());

        Ok(Self {
            mch_order_id: sanitize_mch_order_id(&req.connector_request_reference_id),
            amount: item.amount.clone(),
            currency: currency.to_string(),
            productinfo: format!("Payment {}", req.connector_request_reference_id),
            firstname: firstname.chars().take(50).collect(),
            lastname: lastname.chars().take(50).collect(),
            email,
            phone,
            callback_url,
            redirect_url,
            payment_type,
            beneficiary_type,
            beneficiary_id,
            doc_type,
            doc_number,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TdaypaySyncRequest {
    order_id: String,
}

impl TryFrom<&PaymentsSyncRouterData> for TdaypaySyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let order_id = item
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(Self { order_id })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TdaypayOrderStatus {
    Success,
    Paying,
    Failed,
    Refund,
    Reversed,
    #[serde(other)]
    Unknown,
}

impl From<TdaypayOrderStatus> for enums::AttemptStatus {
    fn from(item: TdaypayOrderStatus) -> Self {
        match item {
            TdaypayOrderStatus::Success => Self::Charged,
            TdaypayOrderStatus::Failed | TdaypayOrderStatus::Refund => Self::Failure,
            TdaypayOrderStatus::Reversed => Self::Voided,
            TdaypayOrderStatus::Paying | TdaypayOrderStatus::Unknown => Self::AuthenticationPending,
        }
    }
}

impl From<TdaypayOrderStatus> for api_models::webhooks::IncomingWebhookEvent {
    fn from(item: TdaypayOrderStatus) -> Self {
        match item {
            TdaypayOrderStatus::Success => Self::PaymentIntentSuccess,
            TdaypayOrderStatus::Failed | TdaypayOrderStatus::Refund => Self::PaymentIntentFailure,
            TdaypayOrderStatus::Reversed => Self::PaymentIntentCancelled,
            TdaypayOrderStatus::Paying | TdaypayOrderStatus::Unknown => {
                Self::PaymentIntentProcessing
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TdaypayPaymentData {
    pub order_id: Option<String>,
    pub mch_order_id: Option<String>,
    pub checkout_url: Option<String>,
    pub order_status: Option<TdaypayOrderStatus>,
    pub amount: Option<String>,
    pub real_amount: Option<String>,
    pub currency: Option<String>,
    pub payment_type: Option<String>,
    pub beneficiary_account_number: Option<String>,
    pub beneficiary_bank_name: Option<String>,
    pub beneficiary_name: Option<String>,
    pub ref_number: Option<String>,
    pub expiration_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TdaypayResponse {
    pub result_code: Option<String>,
    pub error_code: Option<String>,
    pub error_msg: Option<String>,
    #[serde(default)]
    pub data: Option<TdaypayPaymentDataOrList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TdaypayPaymentDataOrList {
    Object(TdaypayPaymentData),
    List(Vec<TdaypayPaymentData>),
}

impl TdaypayPaymentDataOrList {
    pub fn first(self) -> Option<TdaypayPaymentData> {
        match self {
            Self::Object(d) => Some(d),
            Self::List(mut list) => list.drain(..).next(),
        }
    }
    pub fn as_first_ref(&self) -> Option<&TdaypayPaymentData> {
        match self {
            Self::Object(d) => Some(d),
            Self::List(list) => list.first(),
        }
    }
}

impl TdaypayResponse {
    pub fn is_success(&self) -> bool {
        self.result_code.as_deref() == Some(SUCCESS_RESULT_CODE)
    }
    pub fn payment_data(&self) -> Option<&TdaypayPaymentData> {
        self.data.as_ref().and_then(|d| d.as_first_ref())
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, TdaypayResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, TdaypayResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        if !item.response.is_success() {
            let code = item
                .response
                .error_code
                .clone()
                .or_else(|| item.response.result_code.clone())
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string());
            let message = item
                .response
                .error_msg
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string());
            return Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    status_code: item.http_code,
                    code,
                    message: message.clone(),
                    reason: Some(message),
                    attempt_status: Some(enums::AttemptStatus::Failure),
                    connector_transaction_id: item
                        .response
                        .payment_data()
                        .and_then(|d| d.order_id.clone()),
                    connector_response_reference_id: item
                        .response
                        .payment_data()
                        .and_then(|d| d.mch_order_id.clone()),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            });
        }

        let data = item
            .response
            .data
            .clone()
            .and_then(|d| d.first())
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

        let order_id = data
            .order_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?;

        let status = data
            .order_status
            .clone()
            .map(enums::AttemptStatus::from)
            .unwrap_or(enums::AttemptStatus::AuthenticationPending);

        let redirection_data = data.checkout_url.as_ref().map(|url| RedirectForm::Form {
            endpoint: url.clone(),
            method: Method::Get,
            form_fields: HashMap::new(),
        });

        let connector_metadata = serde_json::to_value(&data).ok();

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(order_id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: data.mch_order_id.or(Some(order_id)),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct TdaypayRefundRequest {
    pub amount: StringMajorUnit,
}

impl<F> TryFrom<&TdaypayRouterData<&RefundsRouterData<F>>> for TdaypayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &TdaypayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: String,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::Pending,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::Pending,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TdaypayWebhookBody {
    pub order_id: Option<String>,
    pub mch_order_id: Option<String>,
    pub order_status: Option<TdaypayOrderStatus>,
    pub amount: Option<String>,
    pub currency: Option<String>,
    pub result_code: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TdaypayErrorResponse {
    pub result_code: Option<String>,
    pub error_code: Option<String>,
    pub error_msg: Option<String>,
}
