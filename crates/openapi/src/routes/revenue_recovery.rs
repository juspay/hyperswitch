#[cfg(feature = "v2")]
/// Revenue Recovery - Retrieve
///
/// Retrieve the Revenue Recovery Payment Info
#[utoipa::path(
    get,
    path = "/v2/process-trackers/revenue-recovery-workflow/{revenue_recovery_id}",
    params(
        ("recovery_recovery_id" = String, Path, description = "The payment intent id"),
    ),
    responses(
        (status = 200, description = "Revenue Recovery Info Retrieved Successfully", body = RevenueRecoveryResponse),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Revenue Recovery",
   operation_id = "Retrieve Revenue Recovery Info",
   security(("jwt_key" = []))
)]
pub async fn revenue_recovery_pt_retrieve_api() {}

#[cfg(feature = "v2")]
/// Revenue Recovery - Create
///
/// Record a payment attempt made outside of Hyperswitch against a billing connector invoice,
/// and take the requested recovery action on it, such as scheduling a retry for a failed payment
/// or cancelling the invoice.
#[utoipa::path(
    post,
    path = "/v2/payments/recovery",
    request_body(
        content = RecoveryPaymentsCreate,
        examples(
            (
                "Record a failed payment attempt and schedule a retry" = (
                    value = json!({
                        "amount_details": {
                            "order_amount": 6540,
                            "currency": "USD"
                        },
                        "merchant_reference_id": "invoice_mbabizu24mvu3mela5njyh",
                        "billing_merchant_connector_id": "mca_billing_1234567890",
                        "payment_merchant_connector_id": "mca_payment_1234567890",
                        "transaction_status": "failure",
                        "payment_method_type": "card",
                        "payment_method_sub_type": "credit",
                        "connector_customer_id": "cust_12345",
                        "connector_transaction_id": "993672945374576J",
                        "transaction_created_at": "2022-09-10T10:11:12Z",
                        "error": {
                            "code": "card_declined",
                            "message": "The card was declined."
                        },
                        "payment_method_data": {
                            "primary_processor_payment_method_token": "token_1234",
                            "payment_method_metadata": {
                                "card_network": "Visa",
                                "card_type": "credit",
                                "last4": "4242"
                            }
                        },
                        "action": "schedule_failed_payment"
                    })
                )
            ),
        ),
    ),
    responses(
        (status = 200, description = "Revenue Recovery Payment Recorded Successfully", body = RecoveryPaymentsResponse),
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi),
        (status = 404, description = "Resource missing"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Revenue Recovery",
    operation_id = "Create a Revenue Recovery Payment",
    security(("api_key" = []))
)]
pub async fn revenue_recovery_payments_create() {}
