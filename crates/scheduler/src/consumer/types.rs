pub mod batch;
pub mod process_data;

pub use self::batch::ProcessTrackerBatch;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    strum::EnumString,
    strum::Display,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum PTRunner {
    PaymentsSyncWorkflow,
    RefundWorkflowRouter,
    DeleteTokenizeDataWorkflow,
    ApiKeyExpiryWorkflow,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use common_utils::ext_traits::StringExt;

    use super::PTRunner;

    #[test]
    fn test_enum_to_string() {
        let string_format = "PAYMENTS_SYNC_WORKFLOW".to_string();
        let enum_format: PTRunner = string_format.parse_enum("PTRunner").unwrap();
        assert_eq!(enum_format, PTRunner::PaymentsSyncWorkflow);
    }
}
