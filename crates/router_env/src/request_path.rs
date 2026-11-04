//! Request-path formatting for observability. Request bodies are unaffected.

use std::borrow::Cow;

/// Hide payment callback credentials, including before a route has been matched.
pub fn mask_payment_callback_path(path: &str) -> Cow<'_, str> {
    if path.starts_with("/payments/") {
        if let Some((prefix, _token)) = path.rsplit_once("/callback/") {
            return Cow::Owned(format!("{prefix}/callback/{{callback_token}}"));
        }
    }
    Cow::Borrowed(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_credentials_are_masked_for_every_route_shape() {
        for prefix in [
            "/payments/redirect/pay/merchant/attempt",
            "/payments/pay/merchant/redirect/response/connector",
            "/payments/pay/merchant/redirect/response/connector/creds",
            "/payments/pay/merchant/redirect/complete/connector",
            "/payments/pay/merchant/redirect/complete/connector/creds",
            "/payments/pay/merchant/authorize/connector",
        ] {
            let path = format!("{prefix}/callback/d9dc53cc-d1a1-4f57-9077-d20c2d614daa");
            assert_eq!(
                mask_payment_callback_path(&path),
                format!("{prefix}/callback/{{callback_token}}")
            );
        }
    }

    #[test]
    fn legacy_authentication_and_unrelated_paths_are_preserved() {
        for path in [
            "/payments/redirect/pay/merchant/attempt",
            "/payments/pay/3ds/authentication",
            "/other/callback/value",
        ] {
            assert!(matches!(mask_payment_callback_path(path), Cow::Borrowed(_)));
        }
    }
}
