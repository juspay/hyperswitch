use common_utils::events::ApiEventMetric;

use crate::apple_pay_certificates_migration::ApplePayCertificatesMigrationResponse;

impl ApiEventMetric for ApplePayCertificatesMigrationResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::ApplePayCertificatesMigration)
    }
}
