#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "type", content = "info", rename_all = "snake_case")]
pub enum CurrencyConversionError {
    #[error("Currency Conversion isn't possible")]
    DecimalMultiplicationFailed,
    #[error("Currency not supported: '{0}'")]
    ConversionNotSupported(String),
}
