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
    /// Represents a platform account operating on behalf of a connected account.
    PlatformConnectedAccount(Box<PlatformConnectedAccountContext>),
}

/// `Context` holds the merchant account details and cryptographic key store.
#[derive(Clone, Debug)]
pub struct Context(pub MerchantAccount, pub MerchantKeyStore);

/// PlatformConnectedAccountContext holds the context for a platform account
/// operating on behalf of a connected account
#[derive(Clone, Debug)]
pub struct PlatformConnectedAccountContext {
    /// Context of the platform (owner) account initiating the request and his settings getting used
    pub platform_account_context: Context,
    /// Context of the connected (processor) account whose connector credentials are used
    pub connected_account_context: Context,
}

impl MerchantContext {
    /// Returns a reference to the owner merchant account (platform or normal)
    /// In platform flow, this is the platform's account whose API keys/settings are used
    pub fn get_owner_merchant_account(&self) -> &MerchantAccount {
        match self {
            Self::NormalMerchant(context) => &context.0,
            Self::PlatformConnectedAccount(context) => &context.platform_account_context.0,
        }
    }

    /// Returns a reference to the key store of the owner merchant (platform or normal)
    /// Used for decrypting secrets tied to the owner's configuration.
    pub fn get_owner_merchant_key_store(&self) -> &MerchantKeyStore {
        match self {
            Self::NormalMerchant(context) => &context.1,
            Self::PlatformConnectedAccount(context) => &context.platform_account_context.1,
        }
    }

    /// Returns a reference to the processor merchant account (connected or normal)
    /// In platform flow, this can be connected merchant whose connector credentials are used
    pub fn get_processor_merchant_account(&self) -> &MerchantAccount {
        match self {
            Self::NormalMerchant(context) => &context.0,
            Self::PlatformConnectedAccount(context) => &context.connected_account_context.0,
        }
    }

    /// Returns a reference to the key store of the processor merchant (connected or normal)
    /// Used to access encrypted credentials of the processor merchant account, eg business profile
    pub fn get_processor_merchant_key_store(&self) -> &MerchantKeyStore {
        match self {
            Self::NormalMerchant(context) => &context.1,
            Self::PlatformConnectedAccount(context) => &context.connected_account_context.1,
        }
    }
}
