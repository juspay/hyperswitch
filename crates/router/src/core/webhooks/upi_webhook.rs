//! UPI Webhook Handler
//!
//! Handles asynchronous status updates from UPI gateways (Razorpay, ICICI, EaseBuzz)
//! for UPI In-App payments and mandate transactions.

use api_models::webhooks as webhook_types;
use common_enums::enums;
use error_stack::{ResultExt, report};
use hyperswitch_domain_models::payment_method_data::UpiData;
use router_env::{instrument, logger, tracing};

use crate::{
    core::{
        errors::{RouterResponse, RouterResult, ApiErrorResponse},
        mandate,
        payments::helpers,
    },
    db::StorageInterface,
    routes::SessionState,
    services,
    types::{
        self,
        api,
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

/// Handle UPI webhook for payment status updates
/// Equivalent to Haskell's `callGetStatusResponseForWebhook`
#[instrument(skip(state))]
pub async fn handle_upi_webhook(
    state: &SessionState,
    webhook: &webhook_types::IncomingWebhook,
    merchant_account: &domain::MerchantAccount,
    connector_name: &str,
    profile_id: &str,
) -> RouterResponse<services::ApplicationResponse<()>> {
    logger::info!("Handling UPI webhook from connector: {}", connector_name);

    // Parse webhook payload based on connector
    let webhook_data = match connector_name.to_uppercase().as_str() {
        #[cfg(feature = "connector_razorpay")]
        "RAZORPAY" => parse_razorpay_webhook(webhook)?,
        #[cfg(feature = "connector_icici")]
        "ICICI" => parse_icici_webhook(webhook)?,
        #[cfg(feature = "connector_easebuzz")]
        "EASEBUZZ" => parse_easebuzz_webhook(webhook)?,
        _ => return Err(errors::ApiErrorResponse::UnsupportedConnector.into()),
    };

    // Get transaction details from webhook
    let (payment_id, mandate_id) = extract_transaction_ids(&webhook_data)?;

    // Retrieve payment details
    let payment_attempt = state
        .store
        .find_payment_attempt_by_payment_id(&payment_id, profile_id, merchant_account.storage_scheme)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    // Determine if this is a mandate transaction
    let is_mandate_transaction = mandate_id.is_some() || payment_attempt.mandate_id.is_some();
    let is_otm = check_if_otm(&state, mandate_id.as_ref(), merchant_account).await?;

    // Interpret transaction status
    let txn_status = interpret_transaction_status(
        &webhook_data,
        &payment_attempt,
        is_mandate_transaction,
        is_otm,
    )?;

    // Update transaction based on status
    update_payment_from_webhook(
        state,
        &payment_attempt,
        &webhook_data,
        txn_status,
        is_mandate_transaction,
        merchant_account.storage_scheme,
    )
    .await?;

    Ok(services::ApplicationResponse::StatusOk)
}

/// UPI webhook data structure
#[derive(Debug, Clone)]
pub struct UpiWebhookData {
    /// Gateway transaction ID
    pub gateway_transaction_id: String,
    /// Transaction status from gateway
    pub status: String,
    /// Amount
    pub amount: i64,
    /// Currency
    pub currency: enums::Currency,
    /// VPA used for payment
    pub vpa: Option<String>,
    /// Gateway-specific response codes
    pub gateway_response_code: Option<String>,
    /// Gateway-specific response message
    pub gateway_response_message: Option<String>,
    /// Mandate reference ID (for mandate transactions)
    pub mandate_reference_id: Option<String>,
}

/// Parse Razorpay webhook payload
#[cfg(feature = "connector_razorpay")]
#[instrument(skip(webhook))]
fn parse_razorpay_webhook(
    webhook: &webhook_types::IncomingWebhook,
) -> RouterResult<UpiWebhookData> {
    let body = webhook
        .request
        .body
        .as_ref()
        .get_required_value("webhook_body")?;

    let parsed: serde_json::Value = serde_json::from_str(body)
        .change_context(errors::ApiErrorResponse::InvalidWebhookData)?;

    // Extract Razorpay-specific fields
    let event = parsed
        .get("event")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let payload = parsed.get("payload").and_then(|v| v.get("payment")).ok_or_else(|| {
        report!(errors::ApiErrorResponse::InvalidWebhookData)
            .attach_printable("Missing payment payload in webhook")
    })?;

    let entity = payload.get("entity").and_then(|v| v.as_str()).unwrap_or_default();

    if entity != "payment" {
        return Err(errors::ApiErrorResponse::InvalidWebhookData.into());
    }

    // Parse payment details
    let notes = payload.get("notes").and_then(|v| v.as_object());
    let upi_transaction_id = notes
        .and_then(|n| n.get("upi_transaction_id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    Ok(UpiWebhookData {
        gateway_transaction_id: upi_transaction_id.to_string(),
        status: payload
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string(),
        amount: payload
            .get("amount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        currency: enums::Currency::INR, // Default, should be parsed from payload
        vpa: payload.get("vpa").and_then(|v| v.as_str()).map(|s| s.to_string()),
        gateway_response_code: None,
        gateway_response_message: None,
        mandate_reference_id: None,
    })
}

/// Parse ICICI webhook payload
#[cfg(feature = "connector_icici")]
#[instrument(skip(webhook))]
fn parse_icici_webhook(
    webhook: &webhook_types::IncomingWebhook,
) -> RouterResult<UpiWebhookData> {
    // TODO: Parse ICICI-specific webhook format
    // ICICI webhooks are typically encrypted/decrypted
    Err(errors::ApiErrorResponse::NotImplemented.into())
}

/// Parse EaseBuzz webhook payload
#[cfg(feature = "connector_easebuzz")]
#[instrument(skip(webhook))]
fn parse_easebuzz_webhook(
    webhook: &webhook_types::IncomingWebhook,
) -> RouterResult<UpiWebhookData> {
    // TODO: Parse EaseBuzz-specific webhook format
    Err(errors::ApiErrorResponse::NotImplemented.into())
}

/// Extract transaction IDs from webhook data
fn extract_transaction_ids(
    webhook_data: &UpiWebhookData,
) -> RouterResult<(String, Option<String>)> {
    // Extract payment ID from gateway transaction ID
    // Format: mdtreg_<payment_id> for mandate registrations, or <payment_id> for direct payments
    let payment_id = if webhook_data.gateway_transaction_id.starts_with("mdtreg_") {
        webhook_data.gateway_transaction_id.strip_prefix("mdtreg_").unwrap_or_default()
    } else {
        &webhook_data.gateway_transaction_id
    }.to_string();

    // Extract mandate ID if present
    let mandate_id = webhook_data.mandate_reference_id.clone();

    Ok((payment_id, mandate_id))
}

/// Check if this is a One-Time Mandate (OTM)
async fn check_if_otm(
    state: &SessionState,
    mandate_id: Option<&String>,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<bool> {
    let Some(mandate_id) = mandate_id else {
        return Ok(false);
    };

    let mandate = state
        .store
        .find_mandate_by_merchant_id_mandate_id(
            &merchant_account.merchant_id,
            mandate_id,
            merchant_account.storage_scheme,
        )
        .await
        .ok();

    Ok(mandate.as_ref().map_or(false, |m| {
        matches!(m.mandate_type, storage_enums::MandateType::SingleUse)
    }))
}

/// Interpret transaction status based on gateway response
/// Gateway-specific logic per Haskell documentation
fn interpret_transaction_status(
    webhook_data: &UpiWebhookData,
    payment_attempt: &hyperswitch_domain_models::payments::PaymentAttempt,
    is_mandate_transaction: bool,
    is_otm: bool,
) -> RouterResult<storage_enums::TxnStatus> {
    // Special handling for OTM (One-Time Mandate)
    if is_mandate_transaction && is_otm {
        match webhook_data.status.to_uppercase().as_str() {
            "CREATE-INITIATED" => return Ok(storage_enums::TxnStatus::Authorized),
            "SUCCESS" => return Ok(storage_enums::TxnStatus::Succeeded),
            "FAILED" => return Ok(storage_enums::TxnStatus::Failed),
            _ => {}
        }
    }

    // General transaction status interpretation
    let normalized_status = webhook_data.status.to_uppercase();

    match normalized_status.as_str() {
        // Success states
        "SUCCESS" | "COMPLETED" | "CAPTURED" => Ok(storage_enums::TxnStatus::Succeeded),

        // Pending states
        "PENDING" | "INITIATED" | "PROCESSING" | "AUTHORIZED" => Ok(storage_enums::TxnStatus::Processing),

        // Failure states
        "FAILED" | "DECLINED" | "CANCELLED" => Ok(storage_enums::TxnStatus::Failed),

        // Not found (for polling)
        "NOT_FOUND" | "INVALID" => Ok(storage_enums::TxnStatus::Failed),

        _ => {
            logger::warn!("Unknown UPI transaction status: {}", webhook_data.status);
            Ok(storage_enums::TxnStatus::Processing)
        }
    }
}

/// Update payment from webhook status
#[instrument(skip(state))]
async fn update_payment_from_webhook(
    state: &SessionState,
    payment_attempt: &hyperswitch_domain_models::payments::PaymentAttempt,
    webhook_data: &UpiWebhookData,
    txn_status: storage_enums::TxnStatus,
    is_mandate_transaction: bool,
    storage_scheme: common_enums::StorageScheme,
) -> RouterResult<()> {
    match txn_status {
        storage_enums::TxnStatus::Succeeded => {
            // Update payment attempt status
            let update_attempt = storage::PaymentAttemptUpdate::StatusUpdate {
                status: storage_enums::AttemptStatus::Charged,
                updated_at: common_utils::date_time::now(),
            };

            state
                .store
                .update_payment_attempt(payment_attempt.clone(), update_attempt)
                .await
                .change_context(errors::ApiErrorResponse::PaymentUpdateFailed)?;

            // For mandate transactions, update mandate status
            if is_mandate_transaction && payment_attempt.mandate_id.is_some() {
                update_mandate_from_webhook(
                    state,
                    &payment_attempt.mandate_id.clone().unwrap(),
                    txn_status,
                    storage_scheme,
                )
                .await?;
            }
        }

        storage_enums::TxnStatus::Failed => {
            // Update payment attempt status
            let update_attempt = storage::PaymentAttemptUpdate::StatusUpdate {
                status: storage_enums::AttemptStatus::Failure,
                updated_at: common_utils::date_time::now(),
            };

            state
                .store
                .update_payment_attempt(payment_attempt.clone(), update_attempt)
                .await
                .change_context(errors::ApiErrorResponse::PaymentUpdateFailed)?;

            // For mandate transactions, update mandate status
            if is_mandate_transaction && payment_attempt.mandate_id.is_some() {
                update_mandate_from_webhook(
                    state,
                    &payment_attempt.mandate_id.clone().unwrap(),
                    txn_status,
                    storage_scheme,
                )
                .await?;
            }
        }

        storage_enums::TxnStatus::Processing | storage_enums::TxnStatus::Authorized => {
            // Update payment attempt status to pending
            let update_attempt = storage::PaymentAttemptUpdate::StatusUpdate {
                status: storage_enums::AttemptStatus::Pending,
                updated_at: common_utils::date_time::now(),
            };

            state
                .store
                .update_payment_attempt(payment_attempt.clone(), update_attempt)
                .await
                .change_context(errors::ApiErrorResponse::PaymentUpdateFailed)?;
        }

        _ => {
            logger::info!("Ignoring webhook status update: {:?}", txn_status);
        }
    }

    // Trigger outgoing webhook to merchant
    trigger_outgoing_webhook(state, payment_attempt, webhook_data, txn_status).await?;

    Ok(())
}

/// Update mandate status from webhook
#[instrument(skip(state))]
async fn update_mandate_from_webhook(
    state: &SessionState,
    mandate_id: &String,
    txn_status: storage_enums::TxnStatus,
    storage_scheme: common_enums::StorageScheme,
) -> RouterResult<()> {
    let db = state.store.as_ref();

    // Get current mandate
    let merchant_id = db
        .find_mandate_by_merchant_id_mandate_id(&common_utils::id_type::MerchantId::default(), mandate_id, storage_scheme)
        .await
        .ok(); // We need merchant_id but don't have it here

    // TODO: Get merchant_id properly

    // Determine new mandate status
    let new_mandate_status = match txn_status {
        storage_enums::TxnStatus::Succeeded => storage_enums::MandateStatus::Active,
        storage_enums::TxnStatus::Failed => storage_enums::MandateStatus::Inactive,
        storage_enums::TxnStatus::Processing => storage_enums::MandateStatus::Pending,
        _ => return Ok(()),
    };

    // Update mandate
    // Note: Need proper merchant_id
    /*
    db.update_mandate_by_merchant_id_mandate_id(
        merchant_id,
        mandate_id,
        MandateUpdate::StatusUpdate {
            mandate_status: new_mandate_status,
        },
        mandate,
        storage_scheme,
    )
    .await
    .change_context(errors::ApiErrorResponse::MandateUpdateFailed)?;
    */

    Ok(())
}

/// Trigger outgoing webhook to merchant
#[instrument(skip(state))]
async fn trigger_outgoing_webhook(
    state: &SessionState,
    payment_attempt: &hyperswitch_domain_models::payments::PaymentAttempt,
    webhook_data: &UpiWebhookData,
    txn_status: storage_enums::TxnStatus,
) -> RouterResult<()> {
    // TODO: Implement outgoing webhook triggering
    // This should notify the merchant about the payment status update
    Ok(())
}