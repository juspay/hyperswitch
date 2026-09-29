//! Response intent selected by core and rendered by the shared request wrapper.

use actix_web::http::header::HeaderMap;

/// Success responses that need more than the default JSON rendering.
/// Secret-bearing header values must be marked sensitive before insertion.
#[derive(Debug)]
pub enum ApplicationResponse<T = ()> {
    JsonWithHeaders { body: T, headers: HeaderMap },
    NoContentWithHeaders { headers: HeaderMap },
}
