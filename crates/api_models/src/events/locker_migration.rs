use common_utils::events::ApiEventMetric;

use crate::locker_migration::MigrateCardResponse;

impl ApiEventMetric for MigrateCardResponse {
        /// This method returns the API event type, wrapped in an Option. 
    /// If the API event type is available, it returns Some(ApiEventsType::RustLocker),
    /// otherwise it returns None.
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::RustLocker)
    }
}
