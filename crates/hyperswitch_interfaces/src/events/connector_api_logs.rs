//! Connector API logs interface

use common_utils::request::Method;
use router_env::RequestId;
use serde::Serialize;
use serde_json::json;
use time::OffsetDateTime;

/// struct ConnectorEvent
#[derive(Debug, Serialize)]
pub struct ConnectorEvent {
    tenant_id: common_utils::id_type::TenantId,
    connector_name: String,
    flow: String,
    request: String,
    masked_response: Option<String>,
    error: Option<String>,
    url: String,
    method: String,
    merchant_id: common_utils::id_type::MerchantId,
    created_at: i128,
    /// Connector Event Request ID
    pub request_id: String,
    latency: u128,
    status_code: u16,
    /// Whether this call went to the connector directly or to the Unified Connector Service.
    destination: common_enums::EventDestination,
    /// Primary (real) execution or shadow mirror — the two-state event projection of the
    /// routing `ExecutionMode` (see `common_enums::EventExecutionMode`).
    execution_mode: common_enums::EventExecutionMode,
    #[serde(flatten)]
    connector_event_type: common_utils::events::ConnectorEventsType,
}

impl ConnectorEvent {
    /// fn new ConnectorEvent
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: common_utils::id_type::TenantId,
        connector_name: String,
        flow: &str,
        request: serde_json::Value,
        url: String,
        method: Method,
        payment_id: String,
        merchant_id: common_utils::id_type::MerchantId,
        request_id: Option<&RequestId>,
        latency: u128,
        refund_id: Option<String>,
        dispute_id: Option<String>,
        payout_id: Option<String>,
        status_code: u16,
        destination: common_enums::EventDestination,
        execution_mode: common_enums::EventExecutionMode,
    ) -> Self {
        let connector_event_type = common_utils::events::ConnectorEventsType::new(
            payment_id, refund_id, payout_id, dispute_id,
        );
        Self {
            tenant_id,
            connector_name,
            flow: flow
                .rsplit_once("::")
                .map(|(_, s)| s)
                .unwrap_or(flow)
                .to_string(),
            request: request.to_string(),
            masked_response: None,
            error: None,
            url,
            method: method.to_string(),
            merchant_id,
            created_at: OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
            request_id: request_id
                .map(|i| i.to_string())
                .unwrap_or("NO_REQUEST_ID".to_string()),
            latency,
            status_code,
            destination,
            execution_mode,
            connector_event_type,
        }
    }

    /// fn set_response_body
    pub fn set_response_body<T: Serialize>(&mut self, response: &T) {
        match hyperswitch_masking::masked_serialize(response) {
            Ok(masked) => {
                self.masked_response = Some(masked.to_string());
            }
            Err(er) => self.set_error(json!({"error": er.to_string()})),
        }
    }

    /// fn set_error_response_body
    pub fn set_error_response_body<T: Serialize>(&mut self, response: &T) {
        match hyperswitch_masking::masked_serialize(response) {
            Ok(masked) => {
                self.error = Some(masked.to_string());
            }
            Err(er) => self.set_error(json!({"error": er.to_string()})),
        }
    }

    /// fn set_error
    pub fn set_error(&mut self, error: serde_json::Value) {
        self.error = Some(error.to_string());
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use common_enums::{EventDestination, EventExecutionMode, ExecutionMode};
    use common_utils::request::Method;

    use super::ConnectorEvent;

    /// Builds a connector event the way `emit_ucs_connector_event` does and returns its
    /// serialized form, routing the body by status code like the wrappers do.
    fn serialized_event(
        status_code: u16,
        destination: EventDestination,
        execution_mode: EventExecutionMode,
    ) -> serde_json::Value {
        let tenant_id =
            common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap();
        let merchant_id =
            common_utils::id_type::MerchantId::try_from(Cow::from("test_merchant")).unwrap();
        let mut event = ConnectorEvent::new(
            tenant_id,
            "stripe".to_string(),
            "Authorize",
            serde_json::json!({ "amount": 100 }),
            "grpc://unified-connector-service".to_string(),
            Method::Post,
            "pay_123".to_string(),
            merchant_id,
            None,
            42,
            None,
            None,
            None,
            status_code,
            destination,
            execution_mode,
        );
        let body = serde_json::json!({ "status": "ok" });
        match status_code {
            400..=599 => event.set_error_response_body(&body),
            _ => event.set_response_body(&body),
        }
        serde_json::to_value(&event).unwrap()
    }

    fn str_field<'a>(event: &'a serde_json::Value, key: &str) -> Option<&'a str> {
        event.get(key).and_then(serde_json::Value::as_str)
    }

    /// True when the field is present and non-null.
    fn present(event: &serde_json::Value, key: &str) -> bool {
        match event.get(key) {
            Some(value) => !value.is_null(),
            None => false,
        }
    }

    #[test]
    fn ucs_call_tags_destination_and_execution_mode() {
        let primary = serialized_event(
            200,
            EventDestination::UnifiedConnectorService,
            ExecutionMode::Primary.into(),
        );
        assert_eq!(
            str_field(&primary, "destination"),
            Some("unified_connector_service")
        );
        assert_eq!(str_field(&primary, "execution_mode"), Some("primary"));

        let shadow = serialized_event(
            200,
            EventDestination::UnifiedConnectorService,
            ExecutionMode::Shadow.into(),
        );
        assert_eq!(str_field(&shadow, "execution_mode"), Some("shadow"));

        // A direct connector call (`NotApplicable`) is a live call -> recorded as primary.
        let direct = serialized_event(
            200,
            EventDestination::Connector,
            ExecutionMode::NotApplicable.into(),
        );
        assert_eq!(str_field(&direct, "destination"), Some("connector"));
        assert_eq!(str_field(&direct, "execution_mode"), Some("primary"));
    }

    #[test]
    fn response_body_routes_by_status_code() {
        let ok = serialized_event(
            200,
            EventDestination::UnifiedConnectorService,
            EventExecutionMode::Primary,
        );
        assert!(!present(&ok, "error"));
        assert!(present(&ok, "masked_response"));

        let failure = serialized_event(
            402,
            EventDestination::UnifiedConnectorService,
            EventExecutionMode::Primary,
        );
        assert!(present(&failure, "error"));
        assert!(!present(&failure, "masked_response"));
    }
}
