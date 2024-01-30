#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum QueryType {
    Payment {
        payment_id: String,
    },
    Refund {
        payment_id: String,
        refund_id: String,
    },
    Dispute {
        dispute_id: String,
    },
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ConnectorEventsRequest {
    #[serde(flatten)]
    pub query_param: QueryType,
}
