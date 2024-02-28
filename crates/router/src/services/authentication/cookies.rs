use cookie::{
    time::{Duration, OffsetDateTime},
    Cookie, SameSite,
};
use masking::{ExposeInterface, Secret};

#[cfg(feature = "olap")]
use crate::core::errors::{UserErrors, UserResponse};
use crate::{
    consts::{JWT_TOKEN_COOKIE_NAME, JWT_TOKEN_TIME_IN_SECS},
    services::ApplicationResponse,
};

#[cfg(feature = "olap")]
pub fn set_cookie_response<R>(response: R, token: Secret<String>) -> UserResponse<R> {
    let jwt_expiry_in_seconds = JWT_TOKEN_TIME_IN_SECS
        .try_into()
        .map_err(|_| UserErrors::InternalServerError)?;
    let (expiry, max_age) = get_expiry_and_max_age_from_seconds(jwt_expiry_in_seconds);

    let header_value = create_cookie(token, expiry, max_age).to_string();
    let header_key = get_cookie_header();
    let header = vec![(header_key, header_value)];

    Ok(ApplicationResponse::JsonWithHeaders((response, header)))
}

#[cfg(feature = "olap")]
pub fn remove_cookie_response() -> UserResponse<()> {
    let (expiry, max_age) = get_expiry_and_max_age_from_seconds(0);

    let header_key = get_cookie_header();
    let header_value = create_cookie("".to_string().into(), expiry, max_age).to_string();
    let header = vec![(header_key, header_value)];
    Ok(ApplicationResponse::JsonWithHeaders(((), header)))
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

fn get_cookie_header() -> String {
    actix_http::header::SET_COOKIE.to_string()
}