use std::collections::HashMap;

use common_utils::{errors::CustomResult, ext_traits::OptionExt};
use diesel_models::process_tracker::ProcessTracker;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::errors;

#[derive(Debug, Clone)]
pub struct ProcessTrackerBatch {
    pub id: String,
    pub group_name: String,
    pub stream_name: String,
    pub connection_name: String,
    pub created_time: PrimitiveDateTime,
    pub rule: String,                  // is it required?
    pub trackers: Vec<ProcessTracker>, /* FIXME: Add sized also here,  list */
}

impl ProcessTrackerBatch {
    pub fn to_redis_field_value_pairs(
        &self,
    ) -> CustomResult<Vec<(&str, String)>, errors::ProcessTrackerError> {
        Ok(vec![
            ("id", self.id.to_string()),
            ("group_name", self.group_name.to_string()),
            ("stream_name", self.stream_name.to_string()),
            ("connection_name", self.connection_name.to_string()),
            (
                "created_time",
                self.created_time.assume_utc().unix_timestamp().to_string(),
            ),
            ("rule", self.rule.to_string()),
            (
                "trackers",
                serde_json::to_string(&self.trackers)
                    .change_context(errors::ProcessTrackerError::SerializationFailed)
                    .attach_printable_lazy(|| {
                        format!("Unable to stringify trackers: {:?}", self.trackers)
                    })?,
            ),
        ])
    }

    pub fn from_redis_stream_entry(
        entry: HashMap<String, Option<String>>,
    ) -> CustomResult<Self, errors::ProcessTrackerError> {
        let mut entry = entry;
        let id = entry
            .remove("id")
            .flatten()
            .get_required_value("id")
            .change_context(errors::ProcessTrackerError::MissingRequiredField)?;
        let group_name = entry
            .remove("group_name")
            .flatten()
            .get_required_value("group_name")
            .change_context(errors::ProcessTrackerError::MissingRequiredField)?;
        let stream_name = entry
            .remove("stream_name")
            .flatten()
            .get_required_value("stream_name")
            .change_context(errors::ProcessTrackerError::MissingRequiredField)?;
        let connection_name = entry
            .remove("connection_name")
            .flatten()
            .get_required_value("connection_name")
            .change_context(errors::ProcessTrackerError::MissingRequiredField)?;
        let created_time = entry
            .remove("created_time")
            .flatten()
            .get_required_value("created_time")
            .change_context(errors::ProcessTrackerError::MissingRequiredField)?;

        //make it parser error
        let created_time = {
            let offset_date_time = time::OffsetDateTime::from_unix_timestamp(
                created_time
                    .as_str()
                    .parse::<i64>()
                    .change_context(errors::ParsingError::UnknownError)
                    .change_context(errors::ProcessTrackerError::DeserializationFailed)?,
            )
            .attach_printable_lazy(|| format!("Unable to parse time {}", &created_time))
            .change_context(errors::ProcessTrackerError::MissingRequiredField)?;
            PrimitiveDateTime::new(offset_date_time.date(), offset_date_time.time())
        };

        let rule = entry
            .remove("rule")
            .flatten()
            .get_required_value("rule")
            .change_context(errors::ProcessTrackerError::MissingRequiredField)?;
        let trackers = entry
            .remove("trackers")
            .flatten()
            .get_required_value("trackers")
            .change_context(errors::ProcessTrackerError::MissingRequiredField)?;

        let trackers = serde_json::from_str::<Vec<ProcessTracker>>(trackers.as_str())
            .change_context(errors::ParsingError::UnknownError)
            .attach_printable_lazy(|| {
                format!("Unable to parse trackers from JSON string: {trackers:?}")
            })
            .change_context(errors::ProcessTrackerError::DeserializationFailed)
            .attach_printable("Error parsing ProcessTracker from redis stream entry")?;

        Ok(Self {
            id,
            group_name,
            stream_name,
            connection_name,
            created_time,
            rule,
            trackers,
        })
    }
}
