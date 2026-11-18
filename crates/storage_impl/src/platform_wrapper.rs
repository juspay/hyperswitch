/// Platform wrapper for database operations
///
/// Purpose: Prevent mixing Provider and Processor credentials when making DB calls.
///
/// Payment intent and payment attempt no longer need one: their interfaces take tagged
/// credentials. The rest are for interfaces that are not typed yet.
///
/// Use wrappers when:
/// - You have Platform and need DB calls
/// - Multiple credentials needed (key_store, merchant_id, storage_scheme)
///
/// Direct DB calls are OK when no wrapper is needed:
/// - Function already has Provider or Processor directly
/// - All credentials are bundled in the context
/// - Working with a deprecated module that still uses Processor
///
/// Never mix Provider and Processor credentials or pass Platform to wrappers.
/// Extract the specific context (Provider/Processor) and pass that instead.
///
// TODO: Type the remaining interfaces the same way payments are typed, then drop this module.
pub mod business_profile;
