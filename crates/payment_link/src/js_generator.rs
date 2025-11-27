use api_models::payments::PaymentLinkData;
use error_stack::{Result, ResultExt};

#[derive(Debug, thiserror::Error)]
pub enum PaymentLinkError {
    #[error("Failed to serialize payment link data")]
    SerializationFailed,
}

pub fn get_js_script(payment_details: &PaymentLinkData) -> Result<String, PaymentLinkError> {
    let payment_details_str = serde_json::to_string(payment_details)
        .change_context(PaymentLinkError::SerializationFailed)
        .attach_printable("Failed to serialize PaymentLinkData")?;
    let url_encoded_str = urlencoding::encode(&payment_details_str);
    Ok(format!("window.__PAYMENT_DETAILS = '{url_encoded_str}';"))
}
