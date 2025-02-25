use serde::Serialize;

#[derive(Clone, Debug)]
pub struct VerifyWebhookSource;

#[derive(Debug, Clone, Serialize)]
pub struct ConnectorMandateDetails {
    pub connector_mandate_id: masking::Secret<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectorNetworkTxnId(masking::Secret<String>);

impl ConnectorNetworkTxnId {
    pub fn new(txn_id: masking::Secret<String>) -> Self {
        Self(txn_id)
    }
    pub fn get_id(&self) -> &masking::Secret<String> {
        &self.0
    }
}
