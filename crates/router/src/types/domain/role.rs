use common_enums::MerchantProductType;

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum RoleProductCategory {
    Dashboard,
    Orchestration,
    Vault,
    Recon,
    Recovery,
    CostObservability,
    DynamicRouting,
}

impl From<MerchantProductType> for RoleProductCategory {
    fn from(value: MerchantProductType) -> Self {
        match value {
            MerchantProductType::Orchestration => Self::Orchestration,
            MerchantProductType::Vault => Self::Vault,
            MerchantProductType::Recon => Self::Recon,
            MerchantProductType::Recovery => Self::Recovery,
            MerchantProductType::CostObservability => Self::CostObservability,
            MerchantProductType::DynamicRouting => Self::DynamicRouting,
        }
    }
}

pub fn get_accessible_product_categories(
    product_type: MerchantProductType,
) -> Vec<RoleProductCategory> {
    match product_type {
        MerchantProductType::Orchestration => vec![
            RoleProductCategory::Orchestration,
            RoleProductCategory::Dashboard,
        ],
        MerchantProductType::Vault => {
            vec![RoleProductCategory::Vault, RoleProductCategory::Dashboard]
        }
        MerchantProductType::Recon => {
            vec![RoleProductCategory::Recon, RoleProductCategory::Dashboard]
        }
        MerchantProductType::Recovery => vec![
            RoleProductCategory::Recovery,
            RoleProductCategory::Dashboard,
        ],
        MerchantProductType::CostObservability => vec![
            RoleProductCategory::CostObservability,
            RoleProductCategory::Dashboard,
        ],
        MerchantProductType::DynamicRouting => vec![
            RoleProductCategory::DynamicRouting,
            RoleProductCategory::Dashboard,
        ],
    }
}
