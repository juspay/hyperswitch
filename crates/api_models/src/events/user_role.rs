use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user_role::{
    AuthorizationInfoResponse, DeleteUserRoleRequest, GetRoleRequest, ListRolesResponse,
    RoleInfoResponse, UpdateUserRoleRequest,
};

common_utils::impl_misc_api_event_type!(
    ListRolesResponse,
    RoleInfoResponse,
    GetRoleRequest,
    AuthorizationInfoResponse,
    UpdateUserRoleRequest,
    DeleteUserRoleRequest
);
