use cookie::Cookie;
#[cfg(feature = "olap")]
use cookie::{
    time::{Duration, OffsetDateTime},
    SameSite,
};
use error_stack::{report, ResultExt};
#[cfg(feature = "olap")]
use masking::Mask;
#[cfg(feature = "olap")]
use masking::{ExposeInterface, Secret};

use crate::{
    consts::JWT_TOKEN_COOKIE_NAME,
    core::errors::{ApiErrorResponse, RouterResult},
};
#[cfg(feature = "olap")]
use crate::{
    consts::JWT_TOKEN_TIME_IN_SECS,
    core::errors::{UserErrors, UserResponse},
    services::ApplicationResponse,
};

#[cfg(feature = "olap")]
pub fn set_cookie_response<R>(response: R, token: Secret<String>) -> UserResponse<R> {
    let jwt_expiry_in_seconds = JWT_TOKEN_TIME_IN_SECS
        .try_into()
        .map_err(|_| UserErrors::InternalServerError)?;
    let (expiry, max_age) = get_expiry_and_max_age_from_seconds(jwt_expiry_in_seconds);

    let header_value = create_cookie(token, expiry, max_age)
        .to_string()
        .into_masked();
    let header_key = get_set_cookie_header();
    let header = vec![(header_key, header_value)];

    Ok(ApplicationResponse::JsonWithHeaders((response, header)))
}

#[cfg(feature = "olap")]
pub fn remove_cookie_response() -> UserResponse<()> {
    let (expiry, max_age) = get_expiry_and_max_age_from_seconds(0);

    let header_key = get_set_cookie_header();
    let header_value = create_cookie("".to_string().into(), expiry, max_age)
        .to_string()
        .into_masked();
    let header = vec![(header_key, header_value)];
    Ok(ApplicationResponse::JsonWithHeaders(((), header)))
}

pub fn parse_cookie(cookies: &str) -> RouterResult<String> {
    Cookie::split_parse(cookies)
        .find_map(|cookie| {
            cookie
                .ok()
                .filter(|parsed_cookie| parsed_cookie.name() == JWT_TOKEN_COOKIE_NAME)
                .map(|parsed_cookie| parsed_cookie.value().to_owned())
        })
        .ok_or(report!(ApiErrorResponse::InvalidCookie))
        .attach_printable("Cookie Parsing Failed")
}

#[cfg(feature = "olap")]
fn create_cookie<'c>(
    token: Secret<String>,
    expires: OffsetDateTime,
    max_age: Duration,
) -> Cookie<'c> {
    Cookie::build((JWT_TOKEN_COOKIE_NAME, token.expose()))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .expires(expires)
        .max_age(max_age)
        .build()
}

#[cfg(feature = "olap")]
fn get_expiry_and_max_age_from_seconds(seconds: i64) -> (OffsetDateTime, Duration) {
    let max_age = Duration::seconds(seconds);
    let expiry = OffsetDateTime::now_utc().saturating_add(max_age);
    (expiry, max_age)
}

#[cfg(feature = "olap")]
fn get_set_cookie_header() -> String {
    actix_http::header::SET_COOKIE.to_string()
}

pub fn get_cookie_header() -> String {
    actix_http::header::COOKIE.to_string()
}
