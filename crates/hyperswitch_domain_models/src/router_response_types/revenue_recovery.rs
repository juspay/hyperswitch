use common_utils::types::MinorUnit;
use time::PrimitiveDateTime;
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BillingConnectorPaymentsSyncResponse {
    /// transaction amount against invoice, accepted in minor unit.
    pub amount: MinorUnit,
    /// currency of the transaction
    pub currency: common_enums::enums::Currency,
    /// merchant reference id at billing connector. ex: invoice_id
    pub merchant_reference_id: common_utils::id_type::PaymentReferenceId,
    /// transaction id reference at payment connector
    pub connector_transaction_id: Option<common_utils::types::ConnectorTransactionId>,
    /// error code sent by billing connector.
    pub error_code: Option<String>,
    /// error message sent by billing connector.
    pub error_message: Option<String>,
    /// mandate token at payment processor end.
    pub processor_payment_method_token: String,
    /// customer id at payment connector for which mandate is attached.
    pub connector_customer_id: String,
    /// Payment gateway identifier id at billing processor.
    pub connector_account_reference_id: String,
    /// timestamp at which transaction has been created at billing connector
    pub transaction_created_at: Option<PrimitiveDateTime>,
    /// transaction status at billing connector equivalent to payment attempt status.
    pub status: common_enums::enums::AttemptStatus,
    /// payment method of payment attempt.
    pub payment_method_type: common_enums::enums::PaymentMethod,
    /// payment method sub type of the payment attempt.
    pub payment_method_sub_type: common_enums::enums::PaymentMethodType,
    /// stripe specific id used to validate duplicate attempts.
    pub charge_id: Option<String>,
    /// card information
    pub card_info: api_models::payments::AdditionalCardInfo,
}

#[derive(Debug, Clone)]
pub struct InvoiceRecordBackResponse {
    pub merchant_reference_id: common_utils::id_type::PaymentReferenceId,
}

#[derive(Debug, Clone)]
pub struct BillingConnectorInvoiceSyncResponse {
    /// transaction amount against invoice, accepted in minor unit.
    pub amount: MinorUnit,
    /// currency of the transaction
    pub currency: common_enums::enums::Currency,
    /// merchant reference id at billing connector. ex: invoice_id
    pub merchant_reference_id: common_utils::id_type::PaymentReferenceId,
    /// No of attempts made against an invoice
    pub retry_count: Option<u16>,
    ///  Billing Address of the customer for Invoice
    pub billing_address: Option<api_models::payments::Address>,
    /// creation time of the invoice
    pub created_at: Option<PrimitiveDateTime>,
    /// Ending time of Invoice
    pub ends_at: Option<PrimitiveDateTime>,
}
