use serde;

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct PollId {
    pub poll_id: String,
}
