use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::chat::{
    AutomationAiDataResponse, AutomationAiGetDataRequest, EmbeddedAiDataResponse,
    EmbeddedAiGetDataRequest,
};

common_utils::impl_api_event_type!(
    Miscellaneous,
    (
        AutomationAiGetDataRequest,
        AutomationAiDataResponse,
        EmbeddedAiGetDataRequest,
        EmbeddedAiDataResponse
    )
);
