use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user_role::{
    role::{
        CreateRoleRequest, CreateRoleV2Request, GetParentGroupsAtEntityLevelRequest,
        GetRoleRequest, GroupsAndResources, ListRolesAtEntityLevelRequest, ListRolesRequest,
        ListRolesResponse, ParentGroupInfoRequest, RoleInfoResponseNew,
        RoleInfoResponseWithParentsGroup, RoleInfoWithGroupsResponse, RoleInfoWithParents,
        UpdateRoleRequest,
    },
    AuthorizationInfoResponse, DeleteUserRoleRequest, ListUsersInEntityRequest,
    UpdateUserRoleRequest,
};

common_utils::impl_api_event_type!(
    Miscellaneous,
    (
        GetRoleRequest,
        GetParentGroupsAtEntityLevelRequest,
        AuthorizationInfoResponse,
        UpdateUserRoleRequest,
        DeleteUserRoleRequest,
        CreateRoleRequest,
        CreateRoleV2Request,
        UpdateRoleRequest,
        ListRolesAtEntityLevelRequest,
        RoleInfoResponseNew,
        RoleInfoWithGroupsResponse,
        ListUsersInEntityRequest,
        ListRolesRequest,
        GroupsAndResources,
        RoleInfoWithParents,
        ParentGroupInfoRequest,
        RoleInfoResponseWithParentsGroup,
        ListRolesResponse
    )
);
