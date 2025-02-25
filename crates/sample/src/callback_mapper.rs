// use error_stack::report;

// use router_env::{instrument, tracing};
// use storage_impl::DataModelExt;

// use super::Store;
// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     types::storage,
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use hyperswitch_domain_models::callback_mapper as domain;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait CallbackMapperInterface {
    type Error;
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: domain::CallbackMapper,
    ) -> CustomResult<domain::CallbackMapper, Self::Error>;

    async fn find_call_back_mapper_by_id(
        &self,
        id: &str,
    ) -> CustomResult<domain::CallbackMapper, Self::Error>;
}
