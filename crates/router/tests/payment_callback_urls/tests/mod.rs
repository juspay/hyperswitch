mod connector_callback_urls;
mod sdk_callback_urls;

fn callback_path_and_token(path: &str) -> (String, &str) {
    let (prefix, remaining) = path.split_once("/redirect/token/").expect("token path");
    let (token, suffix) = remaining.split_once('/').expect("callback action");
    (
        format!("{prefix}/redirect/token/{{callback_token}}/{suffix}"),
        token,
    )
}
