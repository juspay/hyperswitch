use common_enums::MerchantProductType;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
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
