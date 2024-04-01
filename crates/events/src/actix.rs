use router_env::tracing_actix_web::RequestId;

use crate::EventInfo;

impl EventInfo for RequestId {
    fn data(
        &self,
    ) -> error_stack::Result<Box<dyn masking::ErasedMaskSerialize + Sync + Send>, crate::EventsError>
    {
        Ok(Box::new(self.as_hyphenated().to_string()))
    }

    fn key(&self) -> String {
        "request_id".to_string()
    }
}
