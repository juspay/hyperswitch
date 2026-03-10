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

/// Platform holds both Provider and Processor together.
/// This struct makes it possible to distinguish the business owner for the org versus whose processor credentials are used for execution.
/// For a standard merchant flow, provider == processor.
#[derive(Clone, Debug)]
pub struct Platform {
    provider: Box<Provider>,
    processor: Box<Processor>,
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
    ) -> Self {
        let provider = Provider::new(provider_account, provider_key_store);
        let processor = Processor::new(processor_account, processor_key_store);
        Self {
            provider: Box::new(provider),
            processor: Box::new(processor),
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
}
