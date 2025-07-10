use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::chat::{AutomationAiDataResponse, ChatRequest, EmbeddedAiDataResponse};

common_utils::impl_api_event_type!(
    Miscellaneous,
    (
        ChatRequest,
        AutomationAiDataResponse,
        EmbeddedAiDataResponse
    )
);
