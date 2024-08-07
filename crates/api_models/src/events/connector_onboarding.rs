use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::connector_onboarding::{
    ActionUrlRequest, ActionUrlResponse, OnboardingStatus, OnboardingSyncRequest,
    ResetTrackingIdRequest,
};

common_utils::impl_api_event_type!(
    Miscellaneous,
    (
        ActionUrlRequest,
        ActionUrlResponse,
        OnboardingSyncRequest,
        OnboardingStatus,
        ResetTrackingIdRequest
    )
);
