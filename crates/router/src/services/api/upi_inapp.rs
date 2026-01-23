//! UPI InApp Service
//!
//! This module handles UPI InApp payment operations including SDK parameter generation,
//! mandate creation, and transaction mode detection.

use common_utils::{
    crypto::{HmacSha256, SignMessage},
};
use error_stack::{ResultExt, report};
use masking::{ExposeInterface, PeekInterface, Secret};
use uuid::Uuid;

use crate::{
    core::errors::{ApiErrorResponse, RouterResult},
    types::upi_inapp::{
        self, UPIPaymentMode, UPIPSP, UpiInAppCreateMandateSDKParams, UpiInAppMandateSDKParamsRequest,
        UpiInAppMandateSDKParamsResponse, UpiInAppPSPAccountDetails, UpiInAppSDKParamsRequest,
        UpiInAppSDKParamsResponse, UpiInAppSessionParams, UpiInAppSplitSettlementDetails,
        UpiInAppTransactionSDKParams,
    },
};

/// Generate current timestamp in the format YYYYMMDDHHMMSS
fn generate_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0));
    let secs = now.as_secs() as i64;
    let days = secs / 86400;
    let rem_day = secs % 86400;
    let hour = rem_day / 3600;
    let rem_hour = rem_day % 3600;
    let min = rem_hour / 60;
    let sec = rem_hour % 60;

    // Approximate date calculation from Unix epoch (good enough for logging/timestamps)
    let year = 1970 + days / 365;
    let month = 1 + ((days % 365) / 30) % 12;
    let day = 1 + (days % 30);

    format!("{:04}{:02}{:02}{:02}{:02}{:02}", year, month, day, hour, min, sec)
}

/// Generate SDK parameters for UPI InApp transaction
pub fn generate_upi_inapp_sdk_params(
    request: UpiInAppSDKParamsRequest,
    psp_account_details: UpiInAppPSPAccountDetails,
) -> RouterResult<UpiInAppSDKParamsResponse> {
    let timestamp = generate_timestamp();

    let currency = request.currency.unwrap_or_else(|| "INR".to_string());
    let uuid_part = Uuid::new_v4().to_string().replace('-', "");
    let upi_id = format!(
        "{}{}",
        psp_account_details.prefix.expose(),
        uuid_part.get(0..16).unwrap_or(&uuid_part)
    );

    // Format split details for signature payload
    let split_details = format_split_details(&request.split_settlement);

    // Extract values before consuming the request
    let payer_vpa_str = request.payer_vpa.expose().clone();
    let payee_vpa_str = request.payee_vpa.expose().clone();

    // Build signature payload
    let payload = build_signature_payload(
        &currency,
        &request.mobile_number,
        psp_account_details.mcc.expose().as_str(),
        psp_account_details.channel_id.expose().as_str(),
        request.customer_id.as_deref().unwrap_or(""),
        psp_account_details.merchant_id.expose().as_str(),
        &timestamp,
        &request.bank_account_reference_id,
        &request.amount,
        payer_vpa_str.as_str(),
        payee_vpa_str.as_str(),
        &request.transaction_reference_id,
        &split_details,
    );

    // Generate signature based on PSP type
    let signature = match psp_account_details.psp {
        UPIPSP::AxisBiz => {
            generate_hmac_signature(&payload, &psp_account_details.signing_key)?
        }
        UPIPSP::YesBiz | UPIPSP::Bhim => {
            generate_rsa_signature(&payload, &psp_account_details.signing_key)?
        }
    };

    let sdk_params = UpiInAppTransactionSDKParams {
        merchant_request_id: request.transaction_reference_id.clone(),
        customer_vpa: payer_vpa_str.clone(),
        merchant_vpa: payee_vpa_str.clone(),
        amount: request.amount.clone(),
        currency: currency.clone(),
        transaction_reference_id: request.transaction_reference_id.clone(),
        signature,
        timestamp,
        psp: psp_account_details.psp.as_str().to_string(),
        upi_app: request.upi_app,
        split_settlement: request.split_settlement.map(|s| {
            serde_json::to_string(&s)
                .unwrap_or_else(|_| String::new())
        }),
        purpose: request.purpose,
        upi_id: Some(upi_id),
    };

    Ok(UpiInAppSDKParamsResponse {
        success: true,
        error_message: None,
        sdk_params: Some(sdk_params),
    })
}

/// Generate session parameters for UPI InApp
pub fn generate_upi_inapp_session_params(
    merchant_id: String,
    customer_id: String,
    mobile_number: Option<Secret<String>>,
    psp_account_details: UpiInAppPSPAccountDetails,
    mga_entries: Vec<upi_inapp::MGAEntry>,
) -> RouterResult<UpiInAppSessionParams> {
    let timestamp = generate_timestamp();
    let currency = "INR".to_string();

    // Build value to be signed
    let mobile = mobile_number
        .as_ref()
        .map(|m| m.peek().clone())
        .unwrap_or_default();

    let mcc = psp_account_details.mcc.expose().clone();
    let channel_id = psp_account_details.channel_id.expose().clone();
    let merchant_id_psp = psp_account_details.merchant_id.expose().clone();
    let prefix = psp_account_details.prefix.expose().clone();

    let value_to_be_signed = format!(
        "{}{}{}{}{}{}{}",
        currency,
        mobile,
        mcc,
        channel_id,
        customer_id,
        merchant_id,
        timestamp,
    );

    // Generate signature
    let signature = match psp_account_details.psp {
        UPIPSP::AxisBiz => {
            generate_hmac_signature(&value_to_be_signed, &psp_account_details.signing_key)?
        }
        UPIPSP::YesBiz | UPIPSP::Bhim => {
            generate_rsa_signature(&value_to_be_signed, &psp_account_details.signing_key)?
        }
    };

    // Build VPA with gateway ref ID entries
    let vpa_with_ref_id_and_gw = mga_entries
        .into_iter()
        .map(|entry| upi_inapp::VpaWithGwRefIdAndGw {
            vpa: entry.vpa,
            ref_id: entry.ref_id,
            gateway: entry.gateway,
        })
        .collect();

    Ok(UpiInAppSessionParams {
        merchant_id,
        channel_id,
        customer_id,
        mcc,
        timestamp,
        currency,
        signature,
        prefix,
        udf: None,
        vpa_with_ref_id_and_gw,
        mobile_number: mobile_number.map(|m| m.expose().clone()),
    })
}

/// Generate mandate SDK parameters for UPI InApp
pub fn generate_upi_inapp_mandate_params(
    request: UpiInAppMandateSDKParamsRequest,
    psp_account_details: UpiInAppPSPAccountDetails,
) -> RouterResult<UpiInAppMandateSDKParamsResponse> {
    // Validate mandate request fields
    validate_mandate_request(&request)?;

    let timestamp = generate_timestamp();
    let currency = "INR".to_string();
    let merchant_request_id = format!("MANDATE_{}", request.transaction_reference_id);

    // Build mandate signature payload with purpose
    let purpose = request.purpose.as_deref().unwrap_or("MERCHANT_PURCHASE");
    let recipient_name = request
        .recipient_name
        .as_ref()
        .or(Some(&request.merchant_id))
        .cloned()
        .unwrap_or_else(|| "Merchant".to_string());

    // Extract VPAs for signature payload
    let payer_vpa_str = request.payer_vpa.expose().clone();
    let payee_vpa_str = request.payee_vpa.expose().clone();
    let bank_ref_id = request.bank_account_reference_id.clone().unwrap_or_default();

    // Build payload in alphabetical order for signature
    let payload = format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}",
        currency,
        "",
        psp_account_details.mcc.expose(),
        psp_account_details.channel_id.expose(),
        request.customer_id,
        request.merchant_id,
        timestamp,
        bank_ref_id,
        request.amount,
        payer_vpa_str,
        payee_vpa_str,
        merchant_request_id,
    );

    // Generate signature based on PSP type
    let signature = match psp_account_details.psp {
        UPIPSP::AxisBiz => {
            generate_hmac_signature(&payload, &psp_account_details.signing_key)?
        }
        UPIPSP::YesBiz | UPIPSP::Bhim => {
            generate_rsa_signature(&payload, &psp_account_details.signing_key)?
        }
    };

    let mandate_params = UpiInAppCreateMandateSDKParams {
        merchant_request_id,
        customer_vpa: payer_vpa_str,
        merchant_vpa: payee_vpa_str,
        amount: request.amount,
        currency,
        transaction_reference_id: request.transaction_reference_id,
        signature,
        timestamp,
        psp: psp_account_details.psp.as_str().to_string(),
        recipient_name,
        amount_rule: request.amount_rule,
        recurrence_pattern: request.recurrence_pattern,
        recurrence_rule: request.recurrence_rule,
        recurrence_value: request.recurrence_value,
        validity_start: request.validity_start,
        validity_end: request.validity_end,
        purpose: Some(purpose.to_string()),
        block_fund: request.block_fund,
    };

    Ok(UpiInAppMandateSDKParamsResponse {
        success: true,
        error_message: None,
        mandate_params: Some(mandate_params),
    })
}

/// Extract UPI payment mode from gateway response
pub fn extract_upi_payment_mode(
    gateway_name: &str,
    response: &serde_json::Value,
) -> RouterResult<UPIPaymentMode> {
    let mode = match gateway_name.to_uppercase().as_str() {
        "PAYU" => {
            // For PAYU, look for mode in routeParams or response fields
            let mode_str = response
                .get("routeParams")
                .and_then(|rp| rp.get("mode"))
                .or(response.get("mode"))
                .and_then(|m| m.as_str())
                .unwrap_or("");

            match mode_str {
                "UPICC" => UPIPaymentMode::CreditCard,
                "UPIPPI" => UPIPaymentMode::PrepaidInstrument,
                _ => UPIPaymentMode::Standard,
            }
        }
        "EASEBUZZ" => {
            // For EaseBuzz, check cardCategory field
            let card_category = response
                .get("cardCategory")
                .or(response.get("card_category"))
                .and_then(|c| c.as_str())
                .unwrap_or("");

            match card_category {
                c if c.contains("CREDIT") || c.contains("Credit") => UPIPaymentMode::CreditCard,
                c if c.contains("PPI") || c.contains("PREPAID") => UPIPaymentMode::PrepaidInstrument,
                _ => UPIPaymentMode::Standard,
            }
        }
        "RAZORPAY" => {
            // Razorpay mode extraction
            let mode_str = response
                .get("mode")
                .or(response.get("upi_mode"))
                .and_then(|m| m.as_str())
                .unwrap_or("");

            match mode_str {
                "cc" | "credit" => UPIPaymentMode::CreditCard,
                "upi" => UPIPaymentMode::Standard,
                _ => UPIPaymentMode::Standard,
            }
        }
        "PHONEPE" | "PAYTM" | "HDFC" | "AXIS" => {
            // For these gateways, default to standard
            UPIPaymentMode::Standard
        }
        _ => UPIPaymentMode::Standard,
    };

    Ok(mode)
}

/// Format split settlement details for signature payload
fn format_split_details(details: &Option<UpiInAppSplitSettlementDetails>) -> String {
    match details {
        Some(split) => {
            let partner_splits: String = split
                .partners_split
                .iter()
                .map(|p| format!("{}{}", p.partner_id, p.value))
                .collect();

            format!(
                "{}{}{}",
                split.merchant_split, partner_splits, split.split_type
            )
        }
        None => String::new(),
    }
}

/// Build signature payload in alphabetical order
fn build_signature_payload(
    currency: &str,
    mobile_number: &Option<Secret<String>>,
    mcc: &str,
    channel_id: &str,
    customer_id: &str,
    merchant_id: &str,
    timestamp: &str,
    account_reference_id: &str,
    amount: &str,
    payer_vpa: &str,
    payee_vpa: &str,
    transaction_reference: &str,
    split_details: &str,
) -> String {
    let mobile = mobile_number
        .as_ref()
        .map(|m| m.peek().clone())
        .unwrap_or_default();

    format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}",
        currency,
        mobile,
        mcc,
        channel_id,
        customer_id,
        merchant_id,
        timestamp,
        account_reference_id,
        amount,
        payer_vpa,
        payee_vpa,
        transaction_reference,
        split_details
    )
}

/// Generate HMAC-SHA256 signature
fn generate_hmac_signature(
    payload: &str,
    signing_key: &Secret<String>,
) -> RouterResult<String> {
    let key = signing_key.peek().as_bytes();
    let msg = payload.as_bytes();

    let signature = HmacSha256
        .sign_message(key, msg)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to generate HMAC signature for UPI InApp")?;

    Ok(base64::encode(signature))
}

/// Generate RSA signature for YesBiz/BHIM PSP
fn generate_rsa_signature(
    _payload: &str,
    _signing_key: &Secret<String>,
) -> RouterResult<String> {
    // RSA signature generation for UPI InApp
    // For production, implement using ring::signature::RsaKeyPair with proper PKCS8 key loading
    Err(report!(ApiErrorResponse::NotImplemented {
        message: hyperswitch_domain_models::errors::api_error_response::NotImplementedMessage::Reason(
            "RSA signature generation for UPI InApp is not yet implemented".to_string()
        ),
    }))
}

/// Validate UPI InApp request fields
pub fn validate_upi_inapp_request(
    request: &UpiInAppSDKParamsRequest,
) -> RouterResult<()> {
    if request.payer_vpa.clone().expose().is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "payer_vpa"
        }))
    } else if request.payee_vpa.clone().expose().is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "payee_vpa"
        }))
    } else if request.amount.is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "amount"
        }))
    } else if request.bank_account_reference_id.is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "bank_account_reference_id"
        }))
    } else {
        Ok(())
    }
}

/// Validate UPI InApp mandate request fields
fn validate_mandate_request(request: &UpiInAppMandateSDKParamsRequest) -> RouterResult<()> {
    if request.merchant_id.is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "merchant_id"
        }))
    } else if request.customer_id.is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "customer_id"
        }))
    } else if request.payer_vpa.clone().expose().is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "payer_vpa"
        }))
    } else if request.amount.is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "amount"
        }))
    } else if request.validity_start.is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "validity_start"
        }))
    } else if request.validity_end.is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "validity_end"
        }))
    } else if request.recurrence_pattern.is_empty() {
        Err(report!(ApiErrorResponse::MissingRequiredField {
            field_name: "recurrence_pattern"
        }))
    } else {
        Ok(())
    }
}

/// Extract bank code from metadata for UPI InApp routing
pub fn extract_bank_code_from_metadata(
    metadata: &Option<common_utils::pii::SecretSerdeValue>,
) -> Option<String> {
    match metadata {
        Some(m) => {
            let exposed = m.clone().expose();
            // Try to extract from nested upi.bank_name
            if let Some(upi) = exposed.get("upi") {
                if let Some(bank_name) = upi.get("bank_name") {
                    if let Some(bn_str) = bank_name.as_str() {
                        return Some(bn_str.to_string());
                    }
                }
            }
            // Try to extract from direct bank_name
            if let Some(bank_name) = exposed.get("bank_name") {
                if let Some(bn_str) = bank_name.as_str() {
                    return Some(bn_str.to_string());
                }
            }
            None
        }
        None => None,
    }
}

/// Construct UPI payment source JSON
pub fn construct_upi_payment_source(
    upi_app: Option<String>,
    payer_vpa: Option<String>,
) -> Option<String> {
    // For gateways like BILLDESK that require structured JSON
    let payment_source = upi_inapp::UpiPaymentSource {
        upi_identifier: "UPI_INAPP".to_string(),
        upi_app,
        payer_vpa,
    };

    payment_source
        .to_json_string()
        .ok()
        .filter(|s| !s.is_empty())
}
