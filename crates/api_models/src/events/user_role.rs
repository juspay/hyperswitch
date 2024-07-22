use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user_role::{
    role::{
        CreateRoleRequest, GetRoleFromTokenResponse, GetRoleRequest, ListRolesResponse,
        RoleInfoResponse, RoleInfoWithGroupsResponse, RoleInfoWithPermissionsResponse,
        UpdateRoleRequest,
    },
    AcceptInvitationRequest, AuthorizationInfoResponse, DeleteUserRoleRequest,
    MerchantSelectRequest, TransferOrgOwnershipRequest, UpdateUserRoleRequest,
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
        TransferOrgOwnershipRequest,
        CreateRoleRequest,
        UpdateRoleRequest,
        ListRolesResponse,
        RoleInfoResponse,
        GetRoleFromTokenResponse,
        RoleInfoWithGroupsResponse
    )
);
