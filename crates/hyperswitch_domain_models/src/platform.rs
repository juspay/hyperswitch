pub use crate::{merchant_account::MerchantAccount, merchant_key_store::MerchantKeyStore};

/// Provider = The business owner or the governing entity in the hierarchy.
/// In a platform-connected setup this is represented by the platform merchant.
/// For a standard merchant, both provider and processor are represented by the same entity.
#[derive(Clone, Debug)]
pub struct Provider {
    account: MerchantAccount,
    key_store: MerchantKeyStore,
}

impl Provider {
    fn new(account: MerchantAccount, key_store: MerchantKeyStore) -> Self {
        Self { account, key_store }
    }

    /// Returns a reference to the merchant account of the provider.
    pub fn get_account(&self) -> &MerchantAccount {
        &self.account
    }

    /// Returns a reference to the key store associated with the provider.
    pub fn get_key_store(&self) -> &MerchantKeyStore {
        &self.key_store
    }
}

/// Processor = The merchant account whose processor credentials are used
/// to execute the operation.
#[derive(Clone, Debug)]
pub struct Processor {
    account: MerchantAccount,
    key_store: MerchantKeyStore,
}

impl Processor {
    fn new(account: MerchantAccount, key_store: MerchantKeyStore) -> Self {
        Self { account, key_store }
    }

    /// Returns a reference to the merchant account of the processor.
    pub fn get_account(&self) -> &MerchantAccount {
        &self.account
    }

    /// Returns a reference to the key store associated with the processor.
    pub fn get_key_store(&self) -> &MerchantKeyStore {
        &self.key_store
    }
}

/// Initiator = The entity that initiated the operation.
#[derive(Clone, Debug)]
pub enum Initiator {
    Api {
        merchant_id: common_utils::id_type::MerchantId,
        merchant_account_type: common_enums::MerchantAccountType,
    },
    Jwt {
        user_id: String,
    },
    Admin,
}

impl Initiator {
    /// Converts the domain Initiator to CreatedBy for database storage.
    ///
    /// # Returns
    /// - `Some(CreatedBy::Api)` for API initiators
    /// - `Some(CreatedBy::Jwt)` for JWT initiators
    /// - `None` for Admin initiators (CreatedBy doesn't have an Admin variant)
    pub fn to_created_by(&self) -> Option<common_utils::types::CreatedBy> {
        match self {
            Self::Api { merchant_id, .. } => Some(common_utils::types::CreatedBy::Api {
                merchant_id: merchant_id.get_string_repr().to_string(),
            }),
            Self::Jwt { user_id } => Some(common_utils::types::CreatedBy::Jwt {
                user_id: user_id.clone(),
            }),
            Self::Admin => None,
        }
    }

    /// Computes the initiator context for API responses.
    ///
    /// # Returns
    /// - `Some(Platform)`: Platform merchant initiated the operation
    /// - `Some(Connected)`: Connected merchant initiated the operation
    /// - `None`: Standard merchant flow, JWT/Admin initiator, or no initiator
    pub fn to_api_initiator(&self) -> Option<api_models::platform::Initiator> {
        match self {
            Self::Api {
                merchant_account_type,
                ..
            } => {
                // If this returns Option<Initiator>, just return it directly (NO extra Some)
                api_models::platform::Initiator::from_merchant_account_type(*merchant_account_type)
            }
            Self::Jwt { .. } | Self::Admin => None,
        }
    }
}

/// Platform holds both Provider and Processor together.
/// This struct makes it possible to distinguish the business owner for the org versus whose processor credentials are used for execution.
/// For a standard merchant flow, provider == processor.
#[derive(Clone, Debug)]
pub struct Platform {
    provider: Box<Provider>,
    processor: Box<Processor>,
    initiator: Option<Initiator>,
}

impl Platform {
    /// Creates a Platform pairing from two merchant identities:
    /// one acting as provider and one as processor
    /// Standard merchants can pass the same account/key_store for both provider and processor
    pub fn new(
        provider_account: MerchantAccount,
        provider_key_store: MerchantKeyStore,
        processor_account: MerchantAccount,
        processor_key_store: MerchantKeyStore,
        initiator: Option<Initiator>,
    ) -> Self {
        let provider = Provider::new(provider_account, provider_key_store);
        let processor = Processor::new(processor_account, processor_key_store);
        Self {
            provider: Box::new(provider),
            processor: Box::new(processor),
            initiator,
        }
    }

    /// Returns a reference to the provider.
    pub fn get_provider(&self) -> &Provider {
        &self.provider
    }

    /// Returns a reference to the processor.
    pub fn get_processor(&self) -> &Processor {
        &self.processor
    }

    /// Returns a reference to the initiator.
    /// Returns None if the initiator is not known or not applicable.
    pub fn get_initiator(&self) -> Option<&Initiator> {
        self.initiator.as_ref()
    }
}
