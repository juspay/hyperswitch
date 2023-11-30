use reqwest::Url;

pub struct OutgoingRequest {
    pub url: Url,
    pub latency: u128,
}

// impl super::KafkaMessage for OutgoingRequest {
//     fn key(&self) -> String {
//         format!(
//             "{}_{}",

//         )
//     }

//     fn creation_timestamp(&self) -> Option<i64> {
//         Some(self.created_at.unix_timestamp())
//     }
// }
