use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user_role::{
    role::{
        CreateRoleRequest, GetRoleRequest, ListRolesAtEntityLevelRequest, ListRolesRequest,
        ListRolesResponse, RoleInfoResponseNew, RoleInfoWithGroupsResponse,
        RoleInfoWithPermissionsResponse, UpdateRoleRequest,
    },
    AcceptInvitationRequest, AuthorizationInfoResponse, DeleteUserRoleRequest,
    ListUsersInEntityRequest, MerchantSelectRequest, UpdateUserRoleRequest,
};

common_utils::impl_api_event_type!(
    Miscellaneous,
    (
        RoleInfoWithPermissionsResponse,
        GetRoleRequest,
        AuthorizationInfoResponse,
        UpdateUserRoleRequest,
        MerchantSelectRequest,
        AcceptInvitationRequest,
        DeleteUserRoleRequest,
        CreateRoleRequest,
        UpdateRoleRequest,
        ListRolesResponse,
        ListRolesAtEntityLevelRequest,
        RoleInfoResponseNew,
        RoleInfoWithGroupsResponse,
        ListUsersInEntityRequest,
        ListRolesRequest
    )
);
