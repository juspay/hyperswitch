use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user_role::{
    role::{
        CreateRoleRequest, GetRoleRequest, ListRolesAtEntityLevelRequest, ListRolesRequest,
        RoleInfoResponseNew, RoleInfoWithGroupsResponse, UpdateRoleRequest,
    },
    AuthorizationInfoResponse, DeleteUserRoleRequest, ListUsersInEntityRequest,
    UpdateUserRoleRequest,
};

common_utils::impl_api_event_type!(
    Miscellaneous,
    (
        GetRoleRequest,
        AuthorizationInfoResponse,
        UpdateUserRoleRequest,
        DeleteUserRoleRequest,
        CreateRoleRequest,
        UpdateRoleRequest,
        ListRolesAtEntityLevelRequest,
        RoleInfoResponseNew,
        RoleInfoWithGroupsResponse,
        ListUsersInEntityRequest,
        ListRolesRequest
    )
);
