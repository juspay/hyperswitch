use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user_role::{
    role::{
        CreateRoleRequest, GetRoleRequest, ListRolesResponse, ListRolesWithPermissionsResponse,
        RoleInfoWithPermissionsResponse, UpdateRoleRequest, RoleInfoResponse,
    },
    AcceptInvitationRequest, AuthorizationInfoResponse, DeleteUserRoleRequest,
    TransferOrgOwnershipRequest, UpdateUserRoleRequest,
};

common_utils::impl_misc_api_event_type!(
    ListRolesWithPermissionsResponse,
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
    RoleInfoResponse
);
