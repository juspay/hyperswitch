use router_env::RequestId;

use crate::EventInfo;

impl EventInfo for RequestId {
    type Data = String;

    fn data(&self) -> error_stack::Result<String, crate::EventsError> {
        Ok(self.to_string())
    }

    fn key(&self) -> String {
        "request_id".to_string()
    }
}
