mod transformers;
pub mod utils;

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use std::collections::hash_map;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use std::hash::{Hash, Hasher};
use std::{collections::HashMap, str::FromStr, sync::Arc};

use api_models::{
    admin as admin_api,
    enums::{self as api_enums},
    routing::ConnectorSelection,
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::{
    open_router::{self as or_types, DecidedGateway, OpenRouterDecideGatewayRequest},
    routing as api_routing,
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use common_utils::{
    ext_traits::{AsyncExt, BytesExt},
    request,
};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use euclid::{
    backend::{self, inputs as dsl_inputs, EuclidBackend},
    dssa::graph::{self as euclid_graph, CgraphExt},
    enums as euclid_enums,
    frontend::{ast, dir as euclid_dir},
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use external_services::grpc_client::dynamic_routing::{
    contract_routing_client::ContractBasedDynamicRouting,
    elimination_based_client::{EliminationBasedRouting, EliminationResponse},
    success_rate_client::{CalSuccessRateResponse, SuccessBasedDynamicRouting},
    DynamicRoutingError,
};
use hyperswitch_domain_models::address::Address;
use kgraph_utils::{
    mca as mca_graph,
    transformers::{IntoContext, IntoDirValue},
    types::CountryCurrencyFilter,
};
use masking::PeekInterface;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use rand::SeedableRng;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use router_env::{instrument, tracing};
use rustc_hash::FxHashMap;
use storage_impl::redis::cache::{CacheKey, CGRAPH_CACHE, ROUTING_CACHE};
use utils::perform_decision_euclid_routing;

#[cfg(feature = "v2")]
use crate::core::admin;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use crate::{core::routing::transformers::OpenRouterDecideGatewayRequestExt, headers, services};
use crate::
    core::{errors, errors as oss_errors}
;
pub use hyperswitch_routing::{core_logic::*, payment_routing::*};

type RoutingResult<O> = oss_errors::CustomResult<O, errors::RoutingError>;
