use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::chat::{ChatRequest, ChatResponse};

common_utils::impl_api_event_type!(Chat, (ChatRequest, ChatResponse));
