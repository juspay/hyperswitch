#![allow(missing_docs)]

#[macro_export]
macro_rules! newtype_impl {
    ($is_pub:vis, $name:ident, $ty_path:path) => {
        impl std::ops::Deref for $name {
            type Target = $ty_path;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl From<$ty_path> for $name {
            fn from(ty: $ty_path) -> Self {
                Self(ty)
            }
        }

        impl $name {
            pub fn into_inner(self) -> $ty_path {
                self.0
            }
        }
    };
}

#[macro_export]
macro_rules! newtype {
    ($is_pub:vis $name:ident = $ty_path:path) => {
        $is_pub struct $name(pub $ty_path);

        $crate::newtype_impl!($is_pub, $name, $ty_path);
    };

    ($is_pub:vis $name:ident = $ty_path:path, derives = ($($trt:path),*)) => {
        #[derive($($trt),*)]
        $is_pub struct $name(pub $ty_path);

        $crate::newtype_impl!($is_pub, $name, $ty_path);
    };
}

/// Use this to ensure that the corresponding
/// openapi route has been implemented in the openapi crate
#[macro_export]
macro_rules! openapi_route {
    ($route_name: ident) => {{
        #[cfg(feature = "openapi")]
        use openapi::routes::$route_name as _;

        $route_name
    }};
}

#[macro_export]
macro_rules! fallback_reverse_lookup_not_found {
    ($a:expr,$b:expr) => {
        match $a {
            Ok(res) => res,
            Err(err) => {
                router_env::logger::error!(reverse_lookup_fallback = %err);
                match err.current_context() {
                    errors::StorageError::ValueNotFound(_) => return $b,
                    errors::StorageError::DatabaseError(data_err) => {
                        match data_err.current_context() {
                            diesel_models::errors::DatabaseError::NotFound => return $b,
                            _ => return Err(err)
                        }
                    }
                    _=> return Err(err)
                }
            }
        };
    };
}

#[macro_export]
macro_rules! collect_missing_value_keys {
    [$(($key:literal, $option:expr)),+] => {
        {
            let mut keys: Vec<&'static str> = Vec::new();
            $(
                if $option.is_none() {
                    keys.push($key);
                }
            )*
            keys
        }
    };
}

#[macro_export]
macro_rules! impl_to_sql_from_sql_json {
    ($type:ty, $diesel_type:ty) => {
        #[allow(unused_qualifications)]
        impl diesel::serialize::ToSql<$diesel_type, diesel::pg::Pg> for $type {
            fn to_sql<'b>(
                &'b self,
                out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
            ) -> diesel::serialize::Result {
                let value = serde_json::to_value(self)?;

                // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
                // please refer to the diesel migration blog:
                // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
                <serde_json::Value as diesel::serialize::ToSql<
                                                                $diesel_type,
                                                                diesel::pg::Pg,
                                                            >>::to_sql(&value, &mut out.reborrow())
            }
        }

        #[allow(unused_qualifications)]
        impl diesel::deserialize::FromSql<$diesel_type, diesel::pg::Pg> for $type {
            fn from_sql(
                bytes: <diesel::pg::Pg as diesel::backend::Backend>::RawValue<'_>,
            ) -> diesel::deserialize::Result<Self> {
                let value = <serde_json::Value as diesel::deserialize::FromSql<
                    $diesel_type,
                    diesel::pg::Pg,
                >>::from_sql(bytes)?;
                Ok(serde_json::from_value(value)?)
            }
        }
    };
    ($type: ty) => {
        $crate::impl_to_sql_from_sql_json!($type, diesel::sql_types::Json);
        $crate::impl_to_sql_from_sql_json!($type, diesel::sql_types::Jsonb);
    };
}
