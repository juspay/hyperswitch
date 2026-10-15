use common_utils::events::ApiEventMetric;

use crate::entities_migration::EntitiesMigrationResponse;

impl ApiEventMetric for EntitiesMigrationResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::EntitiesMigration)
    }
}
