use crate::{schema, schema_v2};

/// This trait will return a single column as primary key even in case of composite primary key.
///
/// In case of composite key, it will return the column that is used as local unique.
pub(super) trait GetPrimaryKey: diesel::Table {
    type PK: diesel::ExpressionMethods;
    fn get_primary_key(&self) -> Self::PK;
}

/// This trait must be implemented for all composite keys.
pub(super) trait CompositeKey {
    type UK;
    /// It will return the local unique key of the composite key.
    ///
    /// If `(attempt_id, merchant_id)` is the composite key for `payment_attempt` table, then it will return `attempt_id`.
    fn get_local_unique_key(&self) -> Self::UK;
}

/// implementation of `CompositeKey` trait for all the composite keys must be done here.
mod composite_key {
    use super::{schema, schema_v2, CompositeKey};
    impl CompositeKey for <schema::payment_attempt::table as diesel::Table>::PrimaryKey {
        type UK = schema::payment_attempt::dsl::attempt_id;
        fn get_local_unique_key(&self) -> Self::UK {
            self.0
        }
    }
    impl CompositeKey for <schema::refund::table as diesel::Table>::PrimaryKey {
        type UK = schema::refund::dsl::refund_id;
        fn get_local_unique_key(&self) -> Self::UK {
            self.1
        }
    }
    impl CompositeKey for <schema::customers::table as diesel::Table>::PrimaryKey {
        type UK = schema::customers::dsl::customer_id;
        fn get_local_unique_key(&self) -> Self::UK {
            self.0
        }
    }
    impl CompositeKey for <schema::blocklist::table as diesel::Table>::PrimaryKey {
        type UK = schema::blocklist::dsl::fingerprint_id;
        fn get_local_unique_key(&self) -> Self::UK {
            self.1
        }
    }
    impl CompositeKey for <schema::incremental_authorization::table as diesel::Table>::PrimaryKey {
        type UK = schema::incremental_authorization::dsl::authorization_id;
        fn get_local_unique_key(&self) -> Self::UK {
            self.0
        }
    }
    impl CompositeKey for <schema::hyperswitch_ai_interaction::table as diesel::Table>::PrimaryKey {
        type UK = schema::hyperswitch_ai_interaction::dsl::id;
        fn get_local_unique_key(&self) -> Self::UK {
            self.0
        }
    }
    impl CompositeKey for <schema_v2::incremental_authorization::table as diesel::Table>::PrimaryKey {
        type UK = schema_v2::incremental_authorization::dsl::authorization_id;
        fn get_local_unique_key(&self) -> Self::UK {
            self.0
        }
    }
    impl CompositeKey for <schema_v2::blocklist::table as diesel::Table>::PrimaryKey {
        type UK = schema_v2::blocklist::dsl::fingerprint_id;
        fn get_local_unique_key(&self) -> Self::UK {
            self.1
        }
    }
}

/// This macro will implement the `GetPrimaryKey` trait for all the tables with single primary key.
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
    // v1 tables
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
    schema::process_tracker::table,
    schema::invoice::table,
    // v2 tables
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

/// This macro will implement the `GetPrimaryKey` trait for all the tables with composite key.
macro_rules! impl_get_primary_key_for_composite {
    ($($table:ty),*) => {
        $(
            impl GetPrimaryKey for $table
            {
                type PK = <<$table as diesel::Table>::PrimaryKey as CompositeKey>::UK;
                fn get_primary_key(&self) -> Self::PK {
                    <Self as diesel::Table>::primary_key(self).get_local_unique_key()
                }
            }
        )*
    };
}

impl_get_primary_key_for_composite!(
    schema::payment_attempt::table,
    schema::refund::table,
    schema::customers::table,
    schema::blocklist::table,
    schema::incremental_authorization::table,
    schema::hyperswitch_ai_interaction::table,
    schema_v2::incremental_authorization::table,
    schema_v2::blocklist::table
);
