pub use crate::{merchant_account::MerchantAccount, merchant_key_store::MerchantKeyStore};

/// Provider (business-owner side)
#[derive(Clone, Debug)]
pub struct Provider {
    account: MerchantAccount,
    key_store: MerchantKeyStore,
}

impl Provider {
    // private constructor
    fn new(account: MerchantAccount, key_store: MerchantKeyStore) -> Self {
        Self { account, key_store }
    }

    /// Get reference to the merchant account
    pub fn get_account(&self) -> &MerchantAccount {
        &self.account
    }

    /// Get reference to the merchant key store
    pub fn get_key_store(&self) -> &MerchantKeyStore {
        &self.key_store
    }
}

/// Processor (credential/connector side)
#[derive(Clone, Debug)]
pub struct Processor {
    account: MerchantAccount,
    key_store: MerchantKeyStore,
}

impl Processor {
    // private constructor
    fn new(account: MerchantAccount, key_store: MerchantKeyStore) -> Self {
        Self { account, key_store }
    }

    /// Get reference to the merchant account
    pub fn get_account(&self) -> &MerchantAccount {
        &self.account
    }

    /// Get reference to the merchant key store
    pub fn get_key_store(&self) -> &MerchantKeyStore {
        &self.key_store
    }
}

/// Holds both provider and processor information
#[derive(Clone, Debug)]
pub struct Platform {
    provider: Box<Provider>,
    processor: Box<Processor>,
}

impl Platform {
    // public constructor
    pub fn new(
        provider_account: MerchantAccount,
        provider_keys: MerchantKeyStore,
        processor_account: MerchantAccount,
        processor_keys: MerchantKeyStore,
    ) -> Self {
        let provider = Provider::new(provider_account, provider_keys);
        let processor = Processor::new(processor_account, processor_keys);
        Self {
            provider: Box::new(provider),
            processor: Box::new(processor),
        }
    }

    /// Get provider
    pub fn get_provider(&self) -> &Provider {
        &self.provider
    }

    /// Get processor
    pub fn get_processor(&self) -> &Processor {
        &self.processor
    }
}
