#![cfg(feature = "v1")]

#[cfg(test)]
fn callback_path_and_token(path: &str) -> (String, &str) {
    let (prefix, remaining) = path.split_once("/redirect/token/").expect("token path");
    let (token, suffix) = remaining.split_once('/').expect("callback action");
    (
        format!("{prefix}/redirect/token/{{callback_token}}/{suffix}"),
        token,
    )
}

#[cfg(test)]
#[path = "payment_callback_urls/sdk_callback_url_tests.rs"]
mod sdk_callback_url_tests;

#[path = "payment_callback_urls/connector_callback_url_tests.rs"]
mod connector_callback_url_tests;
