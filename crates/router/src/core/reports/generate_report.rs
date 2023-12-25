
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PaymentAttemptDimensions {
    AttemptId,
    Status,
    Connector,
    ConnectorTransactionId,
    AmountToCapture,
    CreatedAt,
    ErrorMessage,
    CaptureMethod,
    AuthenticationType,
    MandateId,
    PaymentMethod,
    PaymentMethodType,
    BusinessSubLabel
}

#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PaymentIntentDimensions {
    PaymentId,
    Amount,
    Currency,
    CustomerId,
    OrderDetails,
    Metadata,
    SetupFutureUsage,
    StatementDescriptorName,
    Description,
    OffSession,
    BusinessCountry,
    BusinessLabel,
    AllowedPaymentMethodTypes
}

pub async fn generate_report(
    payload: &GenerateReportRequest,
) -> CustomResult<(), AnalyticsError> {
    

    Ok(())
}
