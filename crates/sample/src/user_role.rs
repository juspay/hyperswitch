use common_utils::errors::CustomResult;
use common_utils::id_type;
use diesel_models::{enums, user_role as storage};

pub struct ListUserRolesByOrgIdPayload<'a> {
    pub user_id: Option<&'a String>,
    pub tenant_id: &'a id_type::TenantId,
    pub org_id: &'a id_type::OrganizationId,
    pub merchant_id: Option<&'a id_type::MerchantId>,
    pub profile_id: Option<&'a id_type::ProfileId>,
    pub version: Option<enums::UserRoleVersion>,
    pub limit: Option<u32>,
}

pub struct ListUserRolesByUserIdPayload<'a> {
    pub user_id: &'a str,
    pub tenant_id: &'a id_type::TenantId,
    pub org_id: Option<&'a id_type::OrganizationId>,
    pub merchant_id: Option<&'a id_type::MerchantId>,
    pub profile_id: Option<&'a id_type::ProfileId>,
    pub entity_id: Option<&'a String>,
    pub version: Option<enums::UserRoleVersion>,
    pub status: Option<enums::UserStatus>,
    pub limit: Option<u32>,
}

#[async_trait::async_trait]
pub trait UserRoleInterface {
    type Error;
    async fn insert_user_role(
        &self,
        user_role: storage::UserRoleNew,
    ) -> CustomResult<storage::UserRole, Self::Error>;

    async fn find_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, Self::Error>;

    #[allow(clippy::too_many_arguments)]
    async fn update_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: Option<&id_type::MerchantId>,
        profile_id: Option<&id_type::ProfileId>,
        update: storage::UserRoleUpdate,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, Self::Error>;

    async fn delete_user_role_by_user_id_and_lineage(
        &self,
        user_id: &str,
        tenant_id: &id_type::TenantId,
        org_id: &id_type::OrganizationId,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
        version: enums::UserRoleVersion,
    ) -> CustomResult<storage::UserRole, Self::Error>;

    async fn list_user_roles_by_user_id<'a>(
        &self,
        payload: ListUserRolesByUserIdPayload<'a>,
    ) -> CustomResult<Vec<storage::UserRole>, Self::Error>;

    async fn list_user_roles_by_user_id_across_tenants(
        &self,
        user_id: &str,
        limit: Option<u32>,
    ) -> CustomResult<Vec<storage::UserRole>, Self::Error>;

    async fn list_user_roles_by_org_id<'a>(
        &self,
        payload: ListUserRolesByOrgIdPayload<'a>,
    ) -> CustomResult<Vec<storage::UserRole>, Self::Error>;
}