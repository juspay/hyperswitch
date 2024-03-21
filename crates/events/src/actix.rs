use router_env::tracing_actix_web::RequestId;

use crate::EventInfo;

impl EventInfo for RequestId {
    fn data(&self) -> error_stack::Result<serde_json::Value, crate::EventsError> {
        Ok(self.as_hyphenated().to_string().into())
    }

    fn key(&self) -> String {
        "request_id".to_string()
    }
}
