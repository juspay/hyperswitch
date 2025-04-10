use diesel;

use crate::{schema, schema_v2};

/// This trait will return a single column as primary key even in case of composite primary key.
pub(super) trait GetPrimaryKey: diesel::Table {
    type PK: diesel::ExpressionMethods;
    fn get_primary_key(&self) -> Self::PK;
}

/// This macro will implement the `GetPrimaryKey` trait for all the tables with single primary key.
/// For tables with composite key, we must implement the trait manually.
macro_rules! impl_get_primary_key {
    ($($table:ty),*) => {
        $(
            impl GetPrimaryKey for $table
            {
                type PK = <$table as diesel::Table>::PrimaryKey;
                fn get_primary_key(&self) -> Self::PK {
                    <Self as diesel::Table>::primary_key(self)
                }
            }
        )*
    };
}
impl_get_primary_key!(
    schema::dashboard_metadata::table,
    schema::merchant_connector_account::table,
    schema::merchant_key_store::table,
    schema::payment_methods::table,
    schema::user_authentication_methods::table,
    schema::user_key_store::table,
    schema::users::table,
    schema::api_keys::table,
    schema::captures::table,
    schema::business_profile::table,
    schema::mandate::dsl::mandate,
    schema::dispute::table,
    schema::events::table,
    schema::merchant_account::table,
    schema::process_tracker::table
);
impl_get_primary_key!(
    schema_v2::dashboard_metadata::table,
    schema_v2::merchant_connector_account::table,
    schema_v2::merchant_key_store::table,
    schema_v2::payment_methods::table,
    schema_v2::user_authentication_methods::table,
    schema_v2::user_key_store::table,
    schema_v2::users::table,
    schema_v2::api_keys::table,
    schema_v2::captures::table,
    schema_v2::business_profile::table,
    schema_v2::mandate::table,
    schema_v2::dispute::table,
    schema_v2::events::table,
    schema_v2::merchant_account::table,
    schema_v2::process_tracker::table,
    schema_v2::refund::table,
    schema_v2::customers::table,
    schema_v2::payment_attempt::table
);

// Manual implementations below

impl GetPrimaryKey for schema::incremental_authorization::table {
    type PK = schema::incremental_authorization::dsl::authorization_id;
    fn get_primary_key(&self) -> Self::PK {
        schema::incremental_authorization::dsl::authorization_id
    }
}

impl GetPrimaryKey for schema_v2::incremental_authorization::table {
    type PK = schema_v2::incremental_authorization::dsl::authorization_id;
    fn get_primary_key(&self) -> Self::PK {
        schema_v2::incremental_authorization::dsl::authorization_id
    }
}

impl GetPrimaryKey for schema::blocklist::table {
    type PK = schema::blocklist::dsl::fingerprint_id;
    fn get_primary_key(&self) -> Self::PK {
        schema::blocklist::dsl::fingerprint_id
    }
}

impl GetPrimaryKey for schema_v2::blocklist::table {
    type PK = schema_v2::blocklist::dsl::fingerprint_id;
    fn get_primary_key(&self) -> Self::PK {
        schema_v2::blocklist::dsl::fingerprint_id
    }
}

impl GetPrimaryKey for schema::customers::table {
    type PK = schema::customers::dsl::customer_id;
    fn get_primary_key(&self) -> Self::PK {
        schema::customers::dsl::customer_id
    }
}

impl GetPrimaryKey for schema::payment_attempt::table {
    type PK = schema::payment_attempt::dsl::attempt_id;
    fn get_primary_key(&self) -> Self::PK {
        schema::payment_attempt::dsl::attempt_id
    }
}

impl GetPrimaryKey for schema::refund::table {
    type PK = schema::refund::dsl::refund_id;
    fn get_primary_key(&self) -> Self::PK {
        schema::refund::dsl::refund_id
    }
}
