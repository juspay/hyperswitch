use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::chat::ChatResponse;

common_utils::impl_api_event_type!(Miscellaneous, (ChatResponse));
