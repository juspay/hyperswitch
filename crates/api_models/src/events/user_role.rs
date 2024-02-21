use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user_role::{
    role::{CreateRoleRequest, UpdateRoleRequest, ListRolesResponse, RoleInfoResponse, GetRoleRequest},
    AcceptInvitationRequest, AuthorizationInfoResponse, DeleteUserRoleRequest, TransferOrgOwnershipRequest, UpdateUserRoleRequest,
};

common_utils::impl_misc_api_event_type!(
    ListRolesResponse,
    RoleInfoResponse,
    GetRoleRequest,
    AuthorizationInfoResponse,
    UpdateUserRoleRequest,
    AcceptInvitationRequest,
    DeleteUserRoleRequest,
    TransferOrgOwnershipRequest,
    CreateRoleRequest,
    UpdateRoleRequest
);
