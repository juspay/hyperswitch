use cookie::{
    time::{Duration, OffsetDateTime},
    Cookie, SameSite,
};
use error_stack::ResultExt;
use masking::{ExposeInterface, Mask, Secret};

use crate::{
    consts::{JWT_TOKEN_COOKIE_NAME, JWT_TOKEN_TIME_IN_SECS},
    core::errors::{ApiErrorResponse, RouterResult, UserErrors, UserResponse},
    services::ApplicationResponse,
};

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
        .ok_or(ApiErrorResponse::InvalidCookie.into())
        .attach_printable("Cookie Parsing Failed")
}

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

fn get_expiry_and_max_age_from_seconds(seconds: i64) -> (OffsetDateTime, Duration) {
    let max_age = Duration::seconds(seconds);
    let expiry = OffsetDateTime::now_utc().saturating_add(max_age);
    (expiry, max_age)
}

fn get_set_cookie_header() -> String {
    actix_http::header::SET_COOKIE.to_string()
}

pub fn get_cookie_header() -> String {
    actix_http::header::COOKIE.to_string()
}
