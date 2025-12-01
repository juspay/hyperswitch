/// Platform wrapper for database operations
///
/// Purpose: Prevent mixing Provider and Processor credentials when making DB calls.
///
/// Use wrappers when:
/// - You have Platform and need DB calls
/// - Multiple credentials needed (key_store, merchant_id, storage_scheme)
///
/// Direct DB calls are OK when no wrapper is needed:
/// - Function already has Provider or Processor directly
/// - All credentials are bundled in the context
///
/// Never mix Provider and Processor credentials or pass Platform to wrappers.
/// Extract the specific context (Provider/Processor) and pass that instead.
///
// TODO: Remove wrappers and migrate to DB interface with typed parameters (ProviderMerchantId, ProcessorMerchantId, etc.) once the platform stabilizes
pub mod business_profile;
pub mod mandate;
pub mod merchant_connector_account;
pub mod payment_attempt;
pub mod payment_intent;
pub mod payment_method;
