#[cfg(feature = "olap")]
pub const MAX_NAME_LENGTH: usize = 70;
#[cfg(feature = "olap")]
pub const MAX_COMPANY_NAME_LENGTH: usize = 70;

// USER ROLES
#[cfg(any(feature = "olap", feature = "oltp"))]
pub const ROLE_ID_ORGANIZATION_ADMIN: &str = "org_admin";
