use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user_role::{
    AuthorizationInfoResponse, GetRoleRequest, GetUsersResponse, ListRolesResponse,
    RoleInfoResponse, UpdateUserRoleRequest,
};

common_utils::impl_misc_api_event_type!(
    ListRolesResponse,
    RoleInfoResponse,
    GetRoleRequest,
    AuthorizationInfoResponse,
    GetUsersResponse,
    UpdateUserRoleRequest
);
