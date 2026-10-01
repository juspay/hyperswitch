//! Dedicated Router transport. No JWT decoding, caller-header forwarding or body logging.

use std::time::Duration;

use external_services::http_client::client::get_client_builder;
use hyperswitch_interfaces::types::Proxy;
use reqwest::{Client, Response};
use url::Url;

mod authorize_token;
mod user_details;

#[derive(Debug, PartialEq, Eq)]
pub enum RouterError {
    InvalidCredential,
    PermissionDenied,
    Unavailable,
}

/// Shared connection pool, constructed once in application state.
#[derive(Clone)]
pub struct RouterClient {
    client: Client,
    base_url: Url,
}

impl RouterClient {
    pub fn new(mut base_url: Url, proxy: &Proxy) -> Result<Self, RouterError> {
        // Treat the configured path as a directory so /api works with or without a trailing slash.
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        let mut builder = get_client_builder(proxy).map_err(|_| RouterError::Unavailable)?;
        if !proxy.has_proxy_config() {
            // With no explicit proxy, retain direct transport rather than inheriting environment proxies.
            builder = builder.no_proxy();
        }
        Ok(Self {
            client: builder
                .timeout(Duration::from_secs(3))
                .build()
                .map_err(|_| RouterError::Unavailable)?,
            base_url,
        })
    }
}

async fn read_bounded(mut response: Response) -> Result<Vec<u8>, RouterError> {
    const LIMIT: usize = 64 * 1024;
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| RouterError::Unavailable)?
    {
        if chunk.len() > LIMIT - body.len() {
            return Err(RouterError::Unavailable);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::{
        net::TcpListener,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
    use hyperswitch_masking::Secret;

    use super::*;

    #[actix_web::test]
    async fn configured_http_proxy_preserves_api_paths_and_credentials() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy = Proxy {
            http_url: Some(format!("http://{}", listener.local_addr().unwrap())),
            ..Proxy::default()
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let server = HttpServer::new(move || {
            let calls = calls.clone();
            App::new().default_service(web::to(move |request: HttpRequest, body: web::Bytes| {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    assert!(request.headers().get("cookie").is_none());
                    match request.uri().path() {
                        "/api/user/internal/authorize" => {
                            assert_eq!(request.method(), "POST");
                            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                            assert_eq!(body["token"], "signed.token.value");
                            assert_eq!(body["permission"], "ProfileReconRuleRead");
                            HttpResponse::Ok().finish()
                        }
                        "/api/user" => {
                            assert_eq!(request.method(), "GET");
                            assert_eq!(
                                request.headers().get("authorization").unwrap(),
                                "Bearer signed.token.value"
                            );
                            HttpResponse::Ok()
                                .json(serde_json::json!({"email": "user@example.com"}))
                        }
                        _ => panic!("unexpected Router path"),
                    }
                }
            }))
        })
        .workers(1)
        .listen(listener)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_web::rt::spawn(server);
        let client =
            RouterClient::new("http://router.invalid/api".parse().unwrap(), &proxy).unwrap();
        assert_eq!(
            client
                .authorize_token(
                    &Secret::new("signed.token.value".into()),
                    "ProfileReconRuleRead"
                )
                .await,
            Ok(())
        );
        assert_eq!(
            client
                .get_user_email(&Secret::new("signed.token.value".into()))
                .await,
            Ok("user@example.com".into())
        );
        assert_eq!(observed.load(Ordering::SeqCst), 2);
        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn configured_proxy_bypass_keeps_internal_router_direct() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/api/", listener.local_addr().unwrap());
        let server = HttpServer::new(|| {
            App::new().route(
                "/api/user",
                web::get().to(|request: HttpRequest| async move {
                    assert_eq!(
                        request.headers().get("authorization").unwrap(),
                        "Bearer signed.token.value"
                    );
                    assert!(request.headers().get("cookie").is_none());
                    HttpResponse::Ok().json(serde_json::json!({"email": "user@example.com"}))
                }),
            )
        })
        .workers(1)
        .listen(listener)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_web::rt::spawn(server);
        let proxy = Proxy {
            http_url: Some("http://127.0.0.1:1".into()),
            bypass_proxy_hosts: Some("127.0.0.1".into()),
            ..Proxy::default()
        };
        let client = RouterClient::new(url.parse().unwrap(), &proxy).unwrap();
        assert_eq!(
            client
                .get_user_email(&Secret::new("signed.token.value".into()))
                .await,
            Ok("user@example.com".into())
        );
        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn configured_https_proxy_receives_connect_and_denial_fails_closed() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy = Proxy {
            https_url: Some(format!("http://{}", listener.local_addr().unwrap())),
            ..Proxy::default()
        };
        let server = actix_web::rt::spawn(async move {
            tokio::time::timeout(Duration::from_secs(3), async move {
                let (mut connection, _) = listener.accept().await.unwrap();
                let mut request = [0; 4096];
                let size = connection.read(&mut request).await.unwrap();
                assert!(request[..size].starts_with(b"CONNECT router.invalid:443 HTTP/1.1\r\n"));
                connection
                    .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
                    .await
                    .unwrap();
            })
            .await
            .expect("proxy receives CONNECT before timeout");
        });
        let client =
            RouterClient::new("https://router.invalid/api/".parse().unwrap(), &proxy).unwrap();
        assert_eq!(
            client
                .get_user_email(&Secret::new("signed.token.value".into()))
                .await,
            Err(RouterError::Unavailable)
        );
        server.await.unwrap();
    }

    #[test]
    fn invalid_proxy_configuration_fails_closed() {
        let proxy = Proxy {
            https_url: Some("http://[".into()),
            ..Proxy::default()
        };
        assert!(matches!(
            RouterClient::new("https://app.hyperswitch.io/api/".parse().unwrap(), &proxy),
            Err(RouterError::Unavailable)
        ));
    }

    #[test]
    fn router_endpoints_preserve_the_configured_base_path() {
        for (base, expected) in [
            ("http://router:80", "http://router/"),
            ("http://router:80/", "http://router/"),
            (
                "https://app.hyperswitch.io/api",
                "https://app.hyperswitch.io/api/",
            ),
            (
                "https://app.hyperswitch.io/api/",
                "https://app.hyperswitch.io/api/",
            ),
        ] {
            let client =
                RouterClient::new(Url::parse(base).expect("valid base URL"), &Proxy::default())
                    .expect("HTTP client builds");
            for endpoint in ["user/internal/authorize", "user"] {
                assert_eq!(
                    client
                        .base_url
                        .join(endpoint)
                        .expect("valid endpoint")
                        .as_str(),
                    format!("{expected}{endpoint}"),
                );
            }
        }
    }
}
