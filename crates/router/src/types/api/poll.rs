#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct PollId {
    pub poll_id: String,
    pub client_secret: Option<hyperswitch_masking::Secret<String>>,
}
