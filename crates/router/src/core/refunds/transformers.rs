pub struct SplitRefundInput {
    pub refund_request: Option<common_types::refunds::SplitRefund>,
    pub payment_charges: Option<common_types::payments::ConnectorChargeResponseData>,
    pub split_payment_request: Option<common_types::payments::SplitPaymentsRequest>,
    pub charge_id: Option<String>,
}
