// `pg_connection_read`/`pg_connection_write`/`pg_accounts_connection_*` used to live here as
// free functions; they're now default methods on `storage_impl::DatabaseStore`, callable
// directly as `self.pg_connection_read()` etc. This module only remains for the re-export
// below, which several `types::storage::*` modules depend on.
pub use diesel_models::DatabaseConnectionWithContext;
