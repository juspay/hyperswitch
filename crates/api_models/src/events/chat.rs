use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::chat::{ChatRequest, EmbeddedAiDataResponse};

common_utils::impl_api_event_type!(Miscellaneous, (ChatRequest, EmbeddedAiDataResponse));
