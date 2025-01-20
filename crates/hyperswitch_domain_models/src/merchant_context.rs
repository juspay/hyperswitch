pub use crate::merchant_account::MerchantAccount;

pub enum MerchantContext {
    PlatformMechant(Box<PlatformAndConnectedAccount>),
    DefaultMerchant(Box<MerchantAccount>),
}

pub struct PlatformAndConnectedAccount {
    platform_merchant_account: MerchantAccount,
    connected_merchant_account: MerchantAccount,
}

impl MerchantContext {
    pub fn new_from_merchant_accounts(
        merchant_account: MerchantAccount,
        platform_merchant_account: Option<MerchantAccount>,
    ) -> Self {
        if let Some(platform_merchant_account) = platform_merchant_account {
            Self::PlatformMechant(Box::new(PlatformAndConnectedAccount {
                platform_merchant_account,
                connected_merchant_account: merchant_account,
            }))
        } else {
            Self::DefaultMerchant(Box::new(merchant_account))
        }
    }
    pub fn get_platform_merchant_account(&self) -> Option<&MerchantAccount> {
        match self {
            MerchantContext::PlatformMechant(data) => Some(&data.platform_merchant_account),
            MerchantContext::DefaultMerchant(_) => None,
        }
    }

    pub fn get_connected_merchant_account(&self) -> Option<&MerchantAccount> {
        match self {
            MerchantContext::PlatformMechant(data) => Some(&data.connected_merchant_account),
            MerchantContext::DefaultMerchant(_) => None,
        }
    }

    pub fn get_tracker_merchant_account(&self) -> &MerchantAccount {
        match self {
            MerchantContext::PlatformMechant(data) => &data.connected_merchant_account,
            MerchantContext::DefaultMerchant(data) => data,
        }
    }

    pub fn get_domain_merchant_account(&self) -> &MerchantAccount {
        match self {
            MerchantContext::PlatformMechant(data) => &data.platform_merchant_account,
            MerchantContext::DefaultMerchant(data) => data,
        }
    }
}
