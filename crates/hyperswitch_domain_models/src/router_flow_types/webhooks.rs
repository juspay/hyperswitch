use serde::Serialize;

#[derive(Clone, Debug)]
pub struct VerifyWebhookSource;

#[derive(Debug, Clone, Serialize)]
pub struct ConnectorMandateDetails {
    pub connector_mandate_id: hyperswitch_masking::Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectorNetworkTxnId(hyperswitch_masking::Secret<String>);

impl ConnectorNetworkTxnId {
    pub fn new(txn_id: hyperswitch_masking::Secret<String>) -> Self {
        Self(txn_id)
    }
    pub fn get_id(&self) -> &hyperswitch_masking::Secret<String> {
        &self.0
    }
}

/// Associated data about a payment shared by a connector over a webhook.
/// Fields are grouped by the record they are written to, so that support for further records
/// (payment methods, for instance) can be added without changing the flow's signature.
#[derive(Debug, Clone, Serialize)]
pub struct WebhookAssociatedData {
    pub payment_attempt: Option<PaymentAttemptAssociatedData>,
}

/// Associated data written to the payment attempt.
#[derive(Debug, Clone, Serialize)]
pub struct PaymentAttemptAssociatedData {
    pub sender_payment_instrument_id: Option<hyperswitch_masking::Secret<String>>,
}
