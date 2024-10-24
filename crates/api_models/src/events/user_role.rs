use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user_role::{
    role::{
        CreateRoleRequest, GetRoleRequest, GroupsAndResources, ListRolesAtEntityLevelRequest,
        ListRolesRequest, RoleInfoResponseNew, RoleInfoWithGroupsResponse, RoleInfoWithParents,
        UpdateRoleRequest,
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
        ListRolesRequest,
        GroupsAndResources,
        RoleInfoWithParents
    )
);
