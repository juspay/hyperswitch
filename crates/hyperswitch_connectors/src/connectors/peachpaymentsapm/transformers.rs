use common_enums::enums;
use common_utils::{request::Method, types::MinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, PaymentMethodData, RealTimePaymentData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::Execute,
    router_request_types::{RefundsData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{types::ResponseRouterData, utils};

/// Router data wrapper for PeachPayments APM
pub struct PeachpaymentsapmRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PeachpaymentsapmRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

/// Authentication credentials for PeachPayments APM API
pub struct PeachpaymentsapmAuthType {
    pub entity_id: Secret<String>,
    pub username: Secret<String>,
    pub password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PeachpaymentsapmAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                entity_id: api_key.clone(),
                username: key1.clone(),
                password: api_secret.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

/// Supported payment brands for PeachPayments APM
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PeachpaymentsapmPaymentBrand {
    /// PayShap - Real-time EFT
    Payshap,
    /// Capitec Pay - Capitec bank instant payments
    #[serde(rename = "CAPITECPAY")]
    CapitecPay,
    /// Peach EFT - Standard EFT with redirect
    #[serde(rename = "PEACHEFT")]
    PeachEft,
    /// M-PESA - Mobile money (Kenya)
    Mpesa,
    /// 1Voucher - PIN-based voucher
    #[serde(rename = "1VOUCHER")]
    OneVoucher,
    /// Mobicred - Credit facility
    Mobicred,
    /// Payflex - BNPL
    Payflex,
    /// ZeroPay - BNPL
    #[serde(rename = "ZEROPAY")]
    ZeroPay,
}

/// Payment type - DB for debit (immediate charge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeachpaymentsapmPaymentType {
    /// Debit - immediate charge
    #[serde(rename = "DB")]
    Debit,
}

/// Virtual account type for Capitec Pay and PayShap
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VirtualAccountType {
    /// Mobile phone number (CELLPHONE for PayShap/CapitecPay)
    #[serde(rename = "CELLPHONE")]
    Cellphone,
    /// South African ID number
    #[serde(rename = "IDNUMBER")]
    IdNumber,
}

/// Payment request for PeachPayments APM (form-encoded)
#[derive(Debug, Clone, Serialize)]
pub struct PeachpaymentsapmPaymentsRequest {
    #[serde(rename = "authentication.entityId")]
    pub entity_id: Secret<String>,
    #[serde(rename = "authentication.userId")]
    pub user_id: Secret<String>,
    #[serde(rename = "authentication.password")]
    pub password: Secret<String>,
    pub amount: String,
    pub currency: String,
    #[serde(rename = "paymentType")]
    pub payment_type: PeachpaymentsapmPaymentType,
    #[serde(rename = "paymentBrand")]
    pub payment_brand: PeachpaymentsapmPaymentBrand,
    #[serde(rename = "merchantTransactionId")]
    pub merchant_transaction_id: String,
    #[serde(rename = "shopperResultUrl")]
    pub shopper_result_url: String,
    #[serde(rename = "virtualAccount.accountId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_account_id: Option<Secret<String>>,
    #[serde(rename = "virtualAccount.type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_account_type: Option<VirtualAccountType>,
    #[serde(rename = "virtualAccount.bank")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_account_bank: Option<String>,
}

impl TryFrom<&PeachpaymentsapmRouterData<&PaymentsAuthorizeRouterData>>
    for PeachpaymentsapmPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &PeachpaymentsapmRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = PeachpaymentsapmAuthType::try_from(&item.router_data.connector_auth_type)?;
        let return_url = item
            .router_data
            .request
            .router_return_url
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "router_return_url",
            })?;

        // Convert amount to string with decimal places (e.g., "100.00")
        let amount_str = format_amount(item.amount, item.router_data.request.currency)?;

        let (payment_brand, virtual_account_id, virtual_account_type, virtual_account_bank) =
            match &item.router_data.request.payment_method_data {
                PaymentMethodData::BankTransfer(bank_transfer) => {
                    get_bank_transfer_params(bank_transfer, item.router_data)?
                }
                PaymentMethodData::RealTimePayment(rtp_data) => {
                    get_rtp_params(rtp_data, item.router_data)?
                }
                _ => {
                    return Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("Peachpaymentsapm"),
                    )
                    .into())
                }
            };

        Ok(Self {
            entity_id: auth.entity_id,
            user_id: auth.username,
            password: auth.password,
            amount: amount_str,
            currency: item.router_data.request.currency.to_string(),
            payment_type: PeachpaymentsapmPaymentType::Debit,
            payment_brand,
            merchant_transaction_id: item.router_data.connector_request_reference_id.clone(),
            shopper_result_url: return_url,
            virtual_account_id,
            virtual_account_type,
            virtual_account_bank,
        })
    }
}

/// Format amount for PeachPayments API (e.g., 10000 cents -> "100.00")
fn format_amount(
    amount: MinorUnit,
    _currency: enums::Currency,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    // PeachPayments expects amount in major units with 2 decimal places (e.g., "100.00")
    // MinorUnit is already in cents, so divide by 100
    let minor_amount = amount.get_amount_as_i64();
    let whole = minor_amount / 100;
    let fraction = (minor_amount % 100).abs();
    Ok(format!("{}.{:02}", whole, fraction))
}

/// Parse bank_code which may be in format "BRAND" or "BRAND:BANK"
/// Returns (brand, optional_bank)
fn parse_bank_code(bank_code: Option<&str>) -> (&str, Option<&str>) {
    match bank_code {
        Some(code) if code.contains(':') => {
            let mut parts = code.splitn(2, ':');
            let brand = parts.next().unwrap_or("");
            let bank = parts.next();
            (brand, bank)
        }
        Some(code) => (code, None),
        None => ("", None),
    }
}

/// Extract payment parameters from BankTransfer data
fn get_bank_transfer_params(
    bank_transfer: &Box<BankTransferData>,
    router_data: &PaymentsAuthorizeRouterData,
) -> Result<
    (
        PeachpaymentsapmPaymentBrand,
        Option<Secret<String>>,
        Option<VirtualAccountType>,
        Option<String>,
    ),
    error_stack::Report<errors::ConnectorError>,
> {
    match bank_transfer.as_ref() {
        BankTransferData::LocalBankTransfer { bank_code } => {
            // Parse bank_code which may be in format "BRAND" or "BRAND:BANK"
            let (brand_str, bank_str) = parse_bank_code(bank_code.as_deref());

            // Map brand_str to payment brand
            let brand = match brand_str {
                "PAYSHAP" => PeachpaymentsapmPaymentBrand::Payshap,
                "CAPITECPAY" => PeachpaymentsapmPaymentBrand::CapitecPay,
                "PEACHEFT" | "PEACH_EFT" | "" => PeachpaymentsapmPaymentBrand::PeachEft,
                _ => PeachpaymentsapmPaymentBrand::PeachEft,
            };

            // For PAYSHAP and CAPITECPAY, extract virtualAccount params from billing
            let (virtual_account_id, virtual_account_type, virtual_account_bank) = match &brand {
                PeachpaymentsapmPaymentBrand::Payshap
                | PeachpaymentsapmPaymentBrand::CapitecPay => {
                    // Get phone number from billing and format for PeachPayments
                    let phone = get_peach_phone_number(router_data)?;
                    // For bank, use CAPITECBANK as default for CapitecPay
                    let bank = match &brand {
                        PeachpaymentsapmPaymentBrand::CapitecPay => {
                            Some("CAPITECBANK".to_string())
                        }
                        _ => bank_str.map(|s| s.to_string()),
                    };
                    (Some(phone), Some(VirtualAccountType::Cellphone), bank)
                }
                _ => (None, None, None),
            };

            Ok((brand, virtual_account_id, virtual_account_type, virtual_account_bank))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("Peachpaymentsapm"),
        )
        .into()),
    }
}

/// Get phone number formatted for PeachPayments API (+XX-XXXXXXXX)
fn get_peach_phone_number(
    router_data: &PaymentsAuthorizeRouterData,
) -> Result<Secret<String>, error_stack::Report<errors::ConnectorError>> {
    use crate::utils::RouterData as _;

    let phone = router_data
        .get_billing_phone()
        .map_err(|_| errors::ConnectorError::MissingRequiredField {
            field_name: "billing.phone",
        })?;

    let country_code = phone
        .country_code
        .as_ref()
        .ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: "billing.phone.country_code",
        })?;

    let number = phone
        .number
        .as_ref()
        .ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: "billing.phone.number",
        })?;

    // Format as +XX-XXXXXXXX (PeachPayments format)
    let formatted = format!("{}-{}", country_code, number.peek());
    Ok(Secret::new(formatted))
}

/// Extract payment parameters from RealTimePayment data
fn get_rtp_params(
    rtp_data: &Box<RealTimePaymentData>,
    router_data: &PaymentsAuthorizeRouterData,
) -> Result<
    (
        PeachpaymentsapmPaymentBrand,
        Option<Secret<String>>,
        Option<VirtualAccountType>,
        Option<String>,
    ),
    error_stack::Report<errors::ConnectorError>,
> {
    match rtp_data.as_ref() {
        RealTimePaymentData::Fps {} => {
            // PayShap is a fast payment system similar to FPS
            // Get phone number from billing for virtualAccount
            let phone = get_peach_phone_number(router_data)?;
            Ok((
                PeachpaymentsapmPaymentBrand::Payshap,
                Some(phone),
                Some(VirtualAccountType::Cellphone),
                None, // Bank can be provided via metadata if needed
            ))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("Peachpaymentsapm"),
        )
        .into()),
    }
}

/// Response from PeachPayments APM API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeachpaymentsapmPaymentsResponse {
    /// Transaction ID
    pub id: String,
    /// Result code and description
    pub result: PeachpaymentsapmResult,
    /// Redirect information (for async flows)
    #[serde(default)]
    pub redirect: Option<PeachpaymentsapmRedirect>,
    /// Timestamp
    #[serde(default)]
    pub timestamp: Option<String>,
    /// NDC (for debugging)
    #[serde(default)]
    pub ndc: Option<String>,
}

/// Result code and description
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeachpaymentsapmResult {
    /// Result code (e.g., "000.200.000" for pending)
    pub code: String,
    /// Human-readable description
    pub description: String,
}

/// Redirect information for async payment flows
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeachpaymentsapmRedirect {
    /// Redirect URL
    pub url: String,
    /// HTTP method for redirect (GET or POST)
    #[serde(default)]
    pub method: Option<String>,
    /// Form parameters for POST redirects
    #[serde(default)]
    pub parameters: Option<Vec<PeachpaymentsapmRedirectParam>>,
}

/// Redirect parameter for POST redirects
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeachpaymentsapmRedirectParam {
    pub name: String,
    pub value: String,
}

/// Map result code to AttemptStatus
pub fn map_result_code_to_status(code: &str) -> enums::AttemptStatus {
    match code {
        // Success patterns (000.000.xxx, 000.100.xxx, 000.300.xxx, 000.600.xxx)
        c if c.starts_with("000.000") => enums::AttemptStatus::Charged,
        c if c.starts_with("000.100") => enums::AttemptStatus::Charged,
        c if c.starts_with("000.300") => enums::AttemptStatus::Charged,
        c if c.starts_with("000.600") => enums::AttemptStatus::Charged,
        // 3DS success patterns
        c if c.starts_with("000.400.1") => enums::AttemptStatus::Charged,
        // Pending/Redirect patterns (000.200.xxx)
        c if c.starts_with("000.200") => enums::AttemptStatus::AuthenticationPending,
        // Pending review patterns (000.4xx.xxx excluding 000.400.1xx)
        c if c.starts_with("000.4") => enums::AttemptStatus::Pending,
        // Waiting for confirmation
        "800.400.500" | "100.400.500" => enums::AttemptStatus::Pending,
        // Cancelled by user
        "100.396.101" => enums::AttemptStatus::AuthenticationFailed,
        // Failure patterns (100.xxx, 200.xxx, 800.xxx, 900.xxx)
        c if c.starts_with("100.") => enums::AttemptStatus::Failure,
        c if c.starts_with("200.") => enums::AttemptStatus::Failure,
        c if c.starts_with("800.") => enums::AttemptStatus::Failure,
        c if c.starts_with("900.") => enums::AttemptStatus::Failure,
        // Default to pending for unknown codes
        _ => enums::AttemptStatus::Pending,
    }
}

/// Check if result code indicates success
fn is_success_code(code: &str) -> bool {
    matches!(
        map_result_code_to_status(code),
        enums::AttemptStatus::Charged
    )
}

/// Check if result code indicates pending/redirect
fn is_pending_code(code: &str) -> bool {
    matches!(
        map_result_code_to_status(code),
        enums::AttemptStatus::AuthenticationPending | enums::AttemptStatus::Pending
    )
}

impl<F, T> TryFrom<ResponseRouterData<F, PeachpaymentsapmPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsapmPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        let status = map_result_code_to_status(&response.result.code);

        // Handle redirect flows (000.200.xxx)
        let redirection_data = if is_pending_code(&response.result.code) {
            response.redirect.as_ref().map(|redirect| {
                let method = match redirect.method.as_deref() {
                    Some("POST") => Method::Post,
                    _ => Method::Get,
                };

                let form_fields = redirect
                    .parameters
                    .as_ref()
                    .map(|params| {
                        params
                            .iter()
                            .map(|p| (p.name.clone(), p.value.clone()))
                            .collect()
                    })
                    .unwrap_or_default();

                RedirectForm::Form {
                    endpoint: redirect.url.clone(),
                    method,
                    form_fields,
                }
            })
        } else {
            None
        };

        let payments_response = if is_success_code(&response.result.code)
            || is_pending_code(&response.result.code)
        {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(response.id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            })
        } else {
            Err(ErrorResponse {
                code: response.result.code.clone(),
                message: response.result.description.clone(),
                reason: Some(response.result.description.clone()),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(response.id.clone()),
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        };

        Ok(Self {
            status,
            response: payments_response,
            ..item.data
        })
    }
}

/// Sync response (same structure as payment response)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeachpaymentsapmSyncResponse {
    pub id: String,
    pub result: PeachpaymentsapmResult,
    #[serde(default)]
    pub timestamp: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, PeachpaymentsapmSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsapmSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        let status = map_result_code_to_status(&response.result.code);

        let payments_response = if is_success_code(&response.result.code)
            || is_pending_code(&response.result.code)
        {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(response.id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            })
        } else {
            Err(ErrorResponse {
                code: response.result.code.clone(),
                message: response.result.description.clone(),
                reason: Some(response.result.description.clone()),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(response.id.clone()),
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        };

        Ok(Self {
            status,
            response: payments_response,
            ..item.data
        })
    }
}

/// Refund request
#[derive(Debug, Clone, Serialize)]
pub struct PeachpaymentsapmRefundRequest {
    #[serde(rename = "authentication.entityId")]
    pub entity_id: Secret<String>,
    #[serde(rename = "authentication.userId")]
    pub user_id: Secret<String>,
    #[serde(rename = "authentication.password")]
    pub password: Secret<String>,
    pub amount: String,
    pub currency: String,
    #[serde(rename = "paymentType")]
    pub payment_type: String,
}

impl TryFrom<&RefundsRouterData<Execute>> for PeachpaymentsapmRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &RefundsRouterData<Execute>) -> Result<Self, Self::Error> {
        let auth = PeachpaymentsapmAuthType::try_from(&item.connector_auth_type)?;
        let amount_str = format_amount(item.request.minor_refund_amount, item.request.currency)?;

        Ok(Self {
            entity_id: auth.entity_id,
            user_id: auth.username,
            password: auth.password,
            amount: amount_str,
            currency: item.request.currency.to_string(),
            payment_type: "RF".to_string(), // RF = Refund
        })
    }
}

/// Refund response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeachpaymentsapmRefundResponse {
    pub id: String,
    pub result: PeachpaymentsapmResult,
}

/// Map result code to refund status
fn map_refund_status(code: &str) -> enums::RefundStatus {
    match code {
        c if c.starts_with("000.000") => enums::RefundStatus::Success,
        c if c.starts_with("000.100") => enums::RefundStatus::Success,
        c if c.starts_with("000.200") => enums::RefundStatus::Pending,
        _ => enums::RefundStatus::Failure,
    }
}

impl<F> TryFrom<ResponseRouterData<F, PeachpaymentsapmRefundResponse, RefundsData, RefundsResponseData>>
    for RouterData<F, RefundsData, RefundsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            F,
            PeachpaymentsapmRefundResponse,
            RefundsData,
            RefundsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        let refund_status = map_refund_status(&response.result.code);

        let response_data = if refund_status == enums::RefundStatus::Failure {
            Err(ErrorResponse {
                code: response.result.code.clone(),
                message: response.result.description.clone(),
                reason: Some(response.result.description.clone()),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(response.id.clone()),
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: response.id.clone(),
                refund_status,
            })
        };

        Ok(Self {
            response: response_data,
            ..item.data
        })
    }
}

/// Error response from PeachPayments API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeachpaymentsapmErrorResponse {
    pub result: PeachpaymentsapmResult,
    #[serde(default)]
    pub timestamp: Option<String>,
}

/// Webhook payload (after decryption)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsapmWebhookBody {
    pub id: String,
    pub payment_type: String,
    pub payment_brand: String,
    pub amount: String,
    pub currency: String,
    pub result: PeachpaymentsapmResult,
    #[serde(default)]
    pub merchant_transaction_id: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyperswitch_domain_models::router_data::ConnectorAuthType;
    use masking::PeekInterface;

    #[test]
    fn test_map_result_code_success() {
        // Test success codes (000.000.xxx)
        assert_eq!(
            map_result_code_to_status("000.000.000"),
            enums::AttemptStatus::Charged
        );
        assert_eq!(
            map_result_code_to_status("000.100.110"),
            enums::AttemptStatus::Charged
        );
        assert_eq!(
            map_result_code_to_status("000.300.000"),
            enums::AttemptStatus::Charged
        );
    }

    #[test]
    fn test_map_result_code_pending() {
        // Test pending/redirect codes (000.200.xxx)
        assert_eq!(
            map_result_code_to_status("000.200.000"),
            enums::AttemptStatus::AuthenticationPending
        );
        assert_eq!(
            map_result_code_to_status("000.200.100"),
            enums::AttemptStatus::AuthenticationPending
        );
    }

    #[test]
    fn test_map_result_code_3ds_success() {
        // Test 3DS success codes (000.400.xxx)
        assert_eq!(
            map_result_code_to_status("000.400.000"),
            enums::AttemptStatus::Charged
        );
        assert_eq!(
            map_result_code_to_status("000.400.100"),
            enums::AttemptStatus::Charged
        );
    }

    #[test]
    fn test_map_result_code_failure() {
        // Test failure codes (100.xxx, 200.xxx, 800.xxx, 900.xxx)
        assert_eq!(
            map_result_code_to_status("100.100.100"),
            enums::AttemptStatus::Failure
        );
        assert_eq!(
            map_result_code_to_status("200.100.101"),
            enums::AttemptStatus::Failure
        );
        assert_eq!(
            map_result_code_to_status("800.100.100"),
            enums::AttemptStatus::Failure
        );
        assert_eq!(
            map_result_code_to_status("900.100.100"),
            enums::AttemptStatus::Failure
        );
    }

    #[test]
    fn test_map_refund_status_success() {
        assert_eq!(
            map_refund_status("000.000.000"),
            enums::RefundStatus::Success
        );
        assert_eq!(
            map_refund_status("000.100.110"),
            enums::RefundStatus::Success
        );
    }

    #[test]
    fn test_map_refund_status_pending() {
        assert_eq!(
            map_refund_status("000.200.000"),
            enums::RefundStatus::Pending
        );
    }

    #[test]
    fn test_map_refund_status_failure() {
        assert_eq!(
            map_refund_status("100.100.100"),
            enums::RefundStatus::Failure
        );
        assert_eq!(
            map_refund_status("800.100.100"),
            enums::RefundStatus::Failure
        );
    }

    #[test]
    fn test_format_amount() {
        // Test amount formatting (minor units to major units string)
        assert_eq!(
            format_amount(MinorUnit::new(10000), enums::Currency::ZAR).unwrap(),
            "100.00"
        );
        assert_eq!(
            format_amount(MinorUnit::new(1), enums::Currency::ZAR).unwrap(),
            "0.01"
        );
        assert_eq!(
            format_amount(MinorUnit::new(100), enums::Currency::ZAR).unwrap(),
            "1.00"
        );
        assert_eq!(
            format_amount(MinorUnit::new(12345), enums::Currency::ZAR).unwrap(),
            "123.45"
        );
    }

    #[test]
    fn test_payment_response_deserialization() {
        let json = r#"{
            "id": "8ac7a4c88babd9b5018baf71f9b506df",
            "paymentType": "DB",
            "paymentBrand": "PAYSHAP",
            "amount": "100.00",
            "currency": "ZAR",
            "descriptor": "Test payment",
            "merchantTransactionId": "test_ref_123",
            "result": {
                "code": "000.200.000",
                "description": "Transaction pending"
            },
            "redirect": {
                "url": "https://test.peachpayments.com/redirect",
                "method": "GET"
            }
        }"#;

        let response: PeachpaymentsapmPaymentsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "8ac7a4c88babd9b5018baf71f9b506df");
        assert_eq!(response.result.code, "000.200.000");
        assert!(response.redirect.is_some());
        let redirect = response.redirect.unwrap();
        assert_eq!(redirect.method, Some("GET".to_string()));
    }

    #[test]
    fn test_sync_response_deserialization() {
        let json = r#"{
            "id": "8ac7a4c88babd9b5018baf71f9b506df",
            "paymentType": "DB",
            "paymentBrand": "PAYSHAP",
            "amount": "100.00",
            "currency": "ZAR",
            "result": {
                "code": "000.000.000",
                "description": "Transaction succeeded"
            }
        }"#;

        let response: PeachpaymentsapmSyncResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "8ac7a4c88babd9b5018baf71f9b506df");
        assert_eq!(response.result.code, "000.000.000");
    }

    #[test]
    fn test_error_response_deserialization() {
        let json = r#"{
            "result": {
                "code": "100.100.100",
                "description": "Invalid payment brand"
            }
        }"#;

        let response: PeachpaymentsapmErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.result.code, "100.100.100");
    }

    #[test]
    fn test_auth_type_parsing() {
        let auth = ConnectorAuthType::SignatureKey {
            api_key: "entity123".to_string().into(),
            key1: "user456".to_string().into(),
            api_secret: "pass789".to_string().into(),
        };

        let result = PeachpaymentsapmAuthType::try_from(&auth);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.entity_id.peek(), "entity123");
        assert_eq!(parsed.username.peek(), "user456");
        assert_eq!(parsed.password.peek(), "pass789");
    }

    #[test]
    fn test_auth_type_wrong_type() {
        let auth = ConnectorAuthType::HeaderKey {
            api_key: "key".to_string().into(),
        };

        let result = PeachpaymentsapmAuthType::try_from(&auth);
        assert!(result.is_err());
    }

    #[test]
    fn test_payment_brand_mapping() {
        // Test that payment brands are correctly identified
        let (brand, _, _, _) =
            get_bank_transfer_params(&Box::new(BankTransferData::LocalBankTransfer {
                bank_code: Some("PAYSHAP".to_string()),
            }))
            .unwrap();
        assert!(matches!(brand, PeachpaymentsapmPaymentBrand::Payshap));

        let (brand, _, _, _) =
            get_bank_transfer_params(&Box::new(BankTransferData::LocalBankTransfer {
                bank_code: Some("CAPITECPAY".to_string()),
            }))
            .unwrap();
        assert!(matches!(brand, PeachpaymentsapmPaymentBrand::CapitecPay));

        let (brand, _, _, _) =
            get_bank_transfer_params(&Box::new(BankTransferData::LocalBankTransfer {
                bank_code: Some("PEACH_EFT".to_string()),
            }))
            .unwrap();
        assert!(matches!(brand, PeachpaymentsapmPaymentBrand::PeachEft));
    }
}
