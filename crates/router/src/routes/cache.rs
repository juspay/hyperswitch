use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{services::{api, authentication as auth}, core::cache};

use super::AppState;

#[instrument(skip_all)]
pub async fn invalidate(
    state: web::Data<AppState>,
    req: HttpRequest,
    key: web::Path<String>) -> impl Responder {
    let flow = Flow::CacheInvalidate;        
    
    let key = key.into_inner().to_owned();
    println!("cache key is {}", &key);
    
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        &key,
        |state, _, key| cache::invalidate(&*state.store, key),
        &auth::AdminApiAuth,
    )
    .await
}
