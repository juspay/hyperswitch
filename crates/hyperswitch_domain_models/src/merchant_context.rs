use common_utils::id_type::ProfileId;

pub use crate::{
    business_profile::Profile, merchant_account::MerchantAccount,
    merchant_key_store::MerchantKeyStore,
};

/// `MerchantContext` represents the authentication and operational context for a merchant.
///
/// This enum encapsulates the merchant's account information and cryptographic keys
/// needed for secure operations. Currently supports only normal merchant operations,
/// but the enum structure allows for future expansion to different merchant types for example a
/// **platform** context.
#[derive(Clone, Debug)]
pub enum MerchantContext<A = ()> {
    /// Represents a normal operation merchant context.
    NormalMerchant(Box<Context>),
    /// Represents a normal operation merchant context with additional accounts information.
    NormalMerchantWithAddOns(Box<ContextWithAddOns<A>>),
}

pub type MerchantContextWithProfile = MerchantContext<ProfileId>;

/// `Context` holds the merchant account details and cryptographic key store.
#[derive(Clone, Debug)]
pub struct Context(pub MerchantAccount, pub MerchantKeyStore);

impl Context {
    pub fn new(merchant_account: MerchantAccount, key_store: MerchantKeyStore) -> Self {
        Self(merchant_account, key_store)
    }
}

#[derive(Clone, Debug)]
pub struct ContextWithAddOns<A> {
    merchant_context: Context,
    add_on: A,
}

impl<A> ContextWithAddOns<A> {
    pub(super) fn new_with_boxed(merchant_context: Context, add_on: A) -> Box<Self> {
        Box::new(Self {
            merchant_context,
            add_on,
        })
    }
    pub(super) fn get_merchant_account(&self) -> &MerchantAccount {
        &self.merchant_context.0
    }

    pub(super) fn get_merchant_key_store(&self) -> &MerchantKeyStore {
        &self.merchant_context.1
    }
}

impl<A> MerchantContext<A> {
    pub fn get_merchant_account(&self) -> &MerchantAccount {
        match self {
            Self::NormalMerchant(merchant_account) => &merchant_account.0,
            Self::NormalMerchantWithAddOns(context) => context.get_merchant_account(),
        }
    }

    pub fn get_merchant_key_store(&self) -> &MerchantKeyStore {
        match self {
            Self::NormalMerchant(merchant_account) => &merchant_account.1,
            Self::NormalMerchantWithAddOns(context) => context.get_merchant_key_store(),
        }
    }
}

impl MerchantContext<()> {
    pub fn convert_to_profile_add_on(self, profile: ProfileId) -> MerchantContextWithProfile {
        match self {
            Self::NormalMerchant(merchant_context) => MerchantContext::NormalMerchantWithAddOns(
                ContextWithAddOns::new_with_boxed(*merchant_context, profile),
            ),
            Self::NormalMerchantWithAddOns(merchant_context) => {
                MerchantContext::NormalMerchantWithAddOns(ContextWithAddOns::new_with_boxed(
                    merchant_context.merchant_context,
                    profile,
                ))
            }
        }
    }
}

impl MerchantContextWithProfile {
    pub fn get_profile_id(&self) -> Option<&ProfileId> {
        match self {
            Self::NormalMerchantWithAddOns(merchant_context) => Some(&merchant_context.add_on),
            Self::NormalMerchant(_) => None,
        }
    }
    pub fn new_normal_merchant_with_profile(
        merchant_account: MerchantAccount,
        key_store: MerchantKeyStore,
        profile: ProfileId,
    ) -> Self {
        Self::NormalMerchantWithAddOns(ContextWithAddOns::new_with_boxed(
            Context::new(merchant_account, key_store),
            profile,
        ))
    }
}
