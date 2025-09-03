use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::chat::{ChatListRequest, ChatListResponse, ChatRequest, ChatResponse};

common_utils::impl_api_event_type!(
    Chat,
    (ChatRequest, ChatResponse, ChatListRequest, ChatListResponse)
);
