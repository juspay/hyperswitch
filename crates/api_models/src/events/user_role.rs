use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user_role::{
    role::{
        CreateRoleRequest, GetRoleFromTokenResponse, GetRoleRequest, ListRolesResponse,
        RoleInfoResponse, RoleInfoWithGroupsResponse, RoleInfoWithPermissionsResponse,
        UpdateRoleRequest,
    },
    AcceptInvitationRequest, AuthorizationInfoResponse, DeleteUserRoleRequest,
    TransferOrgOwnershipRequest, UpdateUserRoleRequest,
};

common_utils::impl_misc_api_event_type!(
    RoleInfoWithPermissionsResponse,
    GetRoleRequest,
    AuthorizationInfoResponse,
    UpdateUserRoleRequest,
    AcceptInvitationRequest,
    DeleteUserRoleRequest,
    TransferOrgOwnershipRequest,
    CreateRoleRequest,
    UpdateRoleRequest,
    ListRolesResponse,
    RoleInfoResponse,
    GetRoleFromTokenResponse,
    RoleInfoWithGroupsResponse
);
