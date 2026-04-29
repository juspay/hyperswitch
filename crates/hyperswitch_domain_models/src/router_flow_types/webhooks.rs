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
