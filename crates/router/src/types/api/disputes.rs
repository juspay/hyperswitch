use masking::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct DisputeId {
    pub dispute_id: String,
}
