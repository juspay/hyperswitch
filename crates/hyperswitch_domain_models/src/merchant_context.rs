pub use crate::{merchant_account::MerchantAccount, merchant_key_store::MerchantKeyStore};

/// `MerchantContext` represents the authentication and operational context for a merchant.
///
/// This enum encapsulates the merchant's account information and cryptographic keys
/// needed for secure operations. Currently supports only normal merchant operations,
/// but the enum structure allows for future expansion to different merchant types for example a
/// **platform** context.
#[derive(Clone, Debug)]
pub enum MerchantContext {
    /// Represents a normal operation merchant context
    StandardMerchant(Box<Context>),

    /// Platform‐scoped operations, where both a platform account and a connected merchant account are present
    PlatformAndConnectedMerchant(Box<PlatformAndConnectedMerchantContext>),
}

/// `Context` holds the merchant account details and cryptographic key store.
#[derive(Clone, Debug)]
pub struct Context(pub MerchantAccount, pub MerchantKeyStore);

/// Holds context for both sides of a platform flow:
/// - `platform_account`: the platform (owner) account context  
/// - `connected_account`: the merchant whose processor credentials are used
#[derive(Clone, Debug)]
pub struct PlatformAndConnectedMerchantContext {
    /// Context of the platform (owner) account
    pub platform_account_context: Context,
    /// Context of the connected (processor) account whose connector credentials are used
    pub connected_account_context: Context,
}

impl MerchantContext {
    /// Returns a reference to the owner merchant account (platform or normal)
    /// In platform flow, this is the platform's account whose API keys/settings are used
    pub fn get_owner_merchant_account(&self) -> &MerchantAccount {
        match self {
            Self::StandardMerchant(context) => &context.0,
            Self::PlatformAndConnectedMerchant(context) => &context.platform_account_context.0,
        }
    }

    /// Returns a reference to the key store of the owner merchant (platform or normal)
    /// Used for decrypting secrets tied to the owner's configuration.
    pub fn get_owner_merchant_key_store(&self) -> &MerchantKeyStore {
        match self {
            Self::StandardMerchant(context) => &context.1,
            Self::PlatformAndConnectedMerchant(context) => &context.platform_account_context.1,
        }
    }

    /// Returns a reference to the processor merchant account (connected or normal)
    /// In platform flow, this can be connected merchant whose connector credentials are used
    pub fn get_processor_merchant_account(&self) -> &MerchantAccount {
        match self {
            Self::StandardMerchant(context) => &context.0,
            Self::PlatformAndConnectedMerchant(context) => &context.connected_account_context.0,
        }
    }

    /// Returns a reference to the key store of the processor merchant (connected or normal)
    /// Used to access encrypted credentials of the processor merchant account, eg business profile
    pub fn get_processor_merchant_key_store(&self) -> &MerchantKeyStore {
        match self {
            Self::StandardMerchant(context) => &context.1,
            Self::PlatformAndConnectedMerchant(context) => &context.connected_account_context.1,
        }
    }
}
