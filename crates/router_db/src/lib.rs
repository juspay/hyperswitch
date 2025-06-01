pub(crate) mod connection;
pub mod db;
pub mod types;

use common_utils::errors as common_errors;
use diesel_models::errors as storage_errors;
use scheduler::errors as sched_errors;
use storage_impl::errors as storage_impl_errors;

pub(crate) mod core {
    pub(crate) mod errors {
        pub use super::super::{
            common_errors::*, sched_errors::*, storage_errors::*, storage_impl_errors::*,
        };
    }
}

pub(crate) mod services {
    pub use crate::db::Store;
}

pub(crate) mod utils {
    pub(crate) mod db_utils {
        pub trait RedisErrorExt {
            #[track_caller]
            fn to_redis_failed_response(
                self,
                key: &str,
            ) -> error_stack::Report<crate::core::errors::StorageError>;
        }

        impl RedisErrorExt for error_stack::Report<crate::core::errors::RedisError> {
            fn to_redis_failed_response(
                self,
                key: &str,
            ) -> error_stack::Report<crate::core::errors::StorageError> {
                match self.current_context() {
                    crate::core::errors::RedisError::NotFound => {
                        self.change_context(crate::core::errors::StorageError::ValueNotFound(
                            format!("Data does not exist for key {key}",),
                        ))
                    }
                    crate::core::errors::RedisError::SetNxFailed => {
                        self.change_context(crate::core::errors::StorageError::DuplicateValue {
                            entity: "redis",
                            key: Some(key.to_string()),
                        })
                    }
                    _ => self.change_context(crate::core::errors::StorageError::KVError),
                }
            }
        }

        // The first argument should be a future while the second argument should be a closure that returns a future for a database call
        pub async fn try_redis_get_else_try_database_get<F, RFut, DFut, T>(
            redis_fut: RFut,
            database_call_closure: F,
        ) -> error_stack::Result<T, crate::core::errors::StorageError>
        where
            F: FnOnce() -> DFut,
            RFut: futures::Future<
                Output = error_stack::Result<T, redis_interface::errors::RedisError>,
            >,
            DFut:
                futures::Future<Output = error_stack::Result<T, crate::core::errors::StorageError>>,
        {
            let redis_output = redis_fut.await;
            match redis_output {
                Ok(output) => Ok(output),
                Err(redis_error) => match redis_error.current_context() {
                    redis_interface::errors::RedisError::NotFound => {
                        // metrics::KV_MISS.add(1, &[]);
                        database_call_closure().await
                    }
                    // Keeping the key empty here since the error would never go here.
                    _ => Err(redis_error.to_redis_failed_response("")),
                },
            }
        }


        /// Generates hscan field pattern. Suppose the field is pa_1234_ref_1211 it will generate
/// pa_1234_ref_*
pub fn generate_hscan_pattern_for_refund(sk: &str) -> String {
    sk.split('_')
        .take(3)
        .chain(["*"])
        .collect::<Vec<&str>>()
        .join("_")
}


    }
}
