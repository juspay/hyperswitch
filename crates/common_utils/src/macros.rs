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
