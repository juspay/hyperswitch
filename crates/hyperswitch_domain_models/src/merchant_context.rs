pub use crate::{merchant_account::MerchantAccount, merchant_key_store::MerchantKeyStore};

#[derive(Clone, Debug)]
pub enum MerchantContext {
    NormalMerchant(Box<Context>),
}

#[derive(Clone, Debug)]
pub struct Context(pub MerchantAccount, pub MerchantKeyStore);

impl MerchantContext {
    pub fn get_merchant_account(&self) -> &MerchantAccount {
        match self {
            Self::NormalMerchant(merchant_account) => &merchant_account.0,
        }
    }

    pub fn get_merchant_key_store(&self) -> &MerchantKeyStore {
        match self {
            Self::NormalMerchant(merchant_account) => &merchant_account.1,
        }
    }
}
