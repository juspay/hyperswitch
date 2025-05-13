pub use crate::{merchant_account::MerchantAccount, merchant_key_store::MerchantKeyStore};

/// `MerchantContext` represents the authentication and operational context for a merchant.
///
/// This enum encapsulates the merchant's account information and cryptographic keys
/// needed for secure operations. Currently supports only normal merchant operations,
/// but the enum structure allows for future expansion to different merchant types for example a
/// **platform** context.
#[derive(Clone, Debug)]
pub enum MerchantContext {
    /// Represents a normal operation merchant context.
    NormalMerchant(Box<Context>),
}

/// `Context` holds the merchant account details and cryptographic key store.
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
