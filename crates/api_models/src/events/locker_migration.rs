use common_utils::events::ApiEventMetric;

use crate::locker_migration::MigrateCardResponse;

impl ApiEventMetric for MigrateCardResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::RustLocker)
    }
}
