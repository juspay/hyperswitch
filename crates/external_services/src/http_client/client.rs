use std::{collections::HashMap, sync::Mutex, time::Duration};

use base64::Engine;
use common_utils::consts::BASE64_ENGINE;
pub use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use masking::ExposeInterface;
use once_cell::sync::OnceCell;


static DEFAULT_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

static PROXY_CLIENT_CACHE: OnceCell<Mutex<HashMap<String, reqwest::Client>>> = OnceCell::new();

use router_env::{logger, metric_attributes};

use super::metrics;


trait ProxyClientCacheKey {
    fn cache_key(&self) -> Option<String>;
    fn has_proxy_config(&self) -> bool;
}

/// Helper function to extract host from proxy URL for metrics
fn extract_proxy_host(url_opt: &Option<String>) -> String {
    url_opt.as_ref()
        .and_then(|url| url::Url::parse(url).ok())
        .and_then(|parsed_url| parsed_url.host_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "none".to_string())
}

// We may need to use outbound proxy to connect to external world.
// Precedence will be the environment variables, followed by the config.
#[allow(missing_docs)]
pub fn create_client(
    proxy_config: &Proxy,
    client_certificate: Option<masking::Secret<String>>,
    client_certificate_key: Option<masking::Secret<String>>,
    ca_certificate: Option<masking::Secret<String>>,
) -> CustomResult<reqwest::Client, HttpClientError> {
    // Case 1: Mutual TLS with client certificate and key
    if let (Some(encoded_certificate), Some(encoded_certificate_key)) =
        (client_certificate.clone(), client_certificate_key.clone())
    {
        if ca_certificate.is_some() {
            logger::warn!("All of client certificate, client key, and CA certificate are provided. CA certificate will be ignored in mutual TLS setup.");
        }

        logger::debug!("Creating HTTP client with mutual TLS (client cert + key)");
        let client_builder =
            apply_mitm_certificate(get_client_builder(proxy_config)?, proxy_config);

        let identity = create_identity_from_certificate_and_key(
            encoded_certificate.clone(),
            encoded_certificate_key,
        )?;
        let certificate_list = create_certificate(encoded_certificate)?;
        let client_builder = certificate_list
            .into_iter()
            .fold(client_builder, |client_builder, certificate| {
                client_builder.add_root_certificate(certificate)
            });
        return client_builder
            .identity(identity)
            .use_rustls_tls()
            .build()
            .change_context(HttpClientError::ClientConstructionFailed)
            .attach_printable("Failed to construct client with certificate and certificate key");
    }

    // Case 2: Use provided CA certificate for server authentication only (one-way TLS)
    if let Some(ca_pem) = ca_certificate {
        logger::debug!("Creating HTTP client with one-way TLS (CA certificate)");
        let pem = ca_pem.expose().replace("\\r\\n", "\n"); // Fix escaped newlines
        let cert = reqwest::Certificate::from_pem(pem.as_bytes())
            .change_context(HttpClientError::ClientConstructionFailed)
            .attach_printable("Failed to parse CA certificate PEM block")?;
        let client_builder =
            apply_mitm_certificate(get_client_builder(proxy_config)?, proxy_config)
                .add_root_certificate(cert);
        return client_builder
            .use_rustls_tls()
            .build()
            .change_context(HttpClientError::ClientConstructionFailed)
            .attach_printable("Failed to construct client with CA certificate");
    }

    // Case 3: Default client (no certs)
    logger::debug!("Creating default HTTP client (no client or CA certificates)");
    get_base_client(proxy_config)
}

#[allow(missing_docs)]
pub fn get_client_builder(
    proxy_config: &Proxy,
) -> CustomResult<reqwest::ClientBuilder, HttpClientError> {
    let mut client_builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .pool_idle_timeout(Duration::from_secs(
            proxy_config
                .idle_pool_connection_timeout
                .unwrap_or_default(),
        ));

    let proxy_exclusion_config =
        reqwest::NoProxy::from_string(&proxy_config.bypass_proxy_hosts.clone().unwrap_or_default());

    logger::debug!("Proxy HTTP Proxy -> {:?} and HTTPS Proxy -> {:?}", proxy_config.http_url.clone(), proxy_config.https_url.clone());

    // Proxy all HTTPS traffic through the configured HTTPS proxy
    if let Some(url) = proxy_config.https_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::https(url)
                .change_context(HttpClientError::InvalidProxyConfiguration)
                .attach_printable("HTTPS proxy configuration error")?
                .no_proxy(proxy_exclusion_config.clone()),
        );
    }

    // Proxy all HTTP traffic through the configured HTTP proxy
    if let Some(url) = proxy_config.http_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::http(url)
                .change_context(HttpClientError::InvalidProxyConfiguration)
                .attach_printable("HTTP proxy configuration error")?
                .no_proxy(proxy_exclusion_config),
        );
    }

    Ok(client_builder)
}

#[allow(missing_docs)]
pub fn create_identity_from_certificate_and_key(
    encoded_certificate: masking::Secret<String>,
    encoded_certificate_key: masking::Secret<String>,
) -> Result<reqwest::Identity, error_stack::Report<HttpClientError>> {
    let decoded_certificate = BASE64_ENGINE
        .decode(encoded_certificate.expose())
        .change_context(HttpClientError::CertificateDecodeFailed)?;

    let decoded_certificate_key = BASE64_ENGINE
        .decode(encoded_certificate_key.expose())
        .change_context(HttpClientError::CertificateDecodeFailed)?;

    let certificate = String::from_utf8(decoded_certificate)
        .change_context(HttpClientError::CertificateDecodeFailed)?;

    let certificate_key = String::from_utf8(decoded_certificate_key)
        .change_context(HttpClientError::CertificateDecodeFailed)?;

    let key_chain = format!("{certificate_key}{certificate}");
    reqwest::Identity::from_pem(key_chain.as_bytes())
        .change_context(HttpClientError::CertificateDecodeFailed)
}

#[allow(missing_docs)]
pub fn create_certificate(
    encoded_certificate: masking::Secret<String>,
) -> Result<Vec<reqwest::Certificate>, error_stack::Report<HttpClientError>> {
    let decoded_certificate = BASE64_ENGINE
        .decode(encoded_certificate.expose())
        .change_context(HttpClientError::CertificateDecodeFailed)?;

    let certificate = String::from_utf8(decoded_certificate)
        .change_context(HttpClientError::CertificateDecodeFailed)?;
    reqwest::Certificate::from_pem_bundle(certificate.as_bytes())
        .change_context(HttpClientError::CertificateDecodeFailed)
}

fn apply_mitm_certificate(
    mut client_builder: reqwest::ClientBuilder,
    proxy_config: &Proxy,
) -> reqwest::ClientBuilder {
    if let Some(mitm_ca_cert) = &proxy_config.mitm_ca_certificate {
        let pem = mitm_ca_cert.clone().expose().replace("\\r\\n", "\n");
        match reqwest::Certificate::from_pem(pem.as_bytes()) {
            Ok(cert) => {
                logger::debug!("Successfully added MITM CA certificate");
                client_builder = client_builder.add_root_certificate(cert);
            }
            Err(err) => {
                logger::error!(
                    "Failed to parse MITM CA certificate: {}, continuing without MITM support",
                    err
                );
            }
        }
    }
    client_builder
}

impl ProxyClientCacheKey for Proxy {
    fn cache_key(&self) -> Option<String> {
        if !self.has_proxy_config() {
            return None;
        }
        Some(format!(
            "http:{}_https:{}_bypass:{}_timeout:{}_mitm:{}",
            self.http_url.as_deref().unwrap_or(""),
            self.https_url.as_deref().unwrap_or(""), 
            self.bypass_proxy_hosts.as_deref().unwrap_or(""),
            self.idle_pool_connection_timeout.unwrap_or_default(),
            self.mitm_ca_certificate.is_some()
        ))
    }
    
    fn has_proxy_config(&self) -> bool {
        self.http_url.is_some() 
            || self.https_url.is_some() 
            || self.mitm_ca_certificate.is_some()
    }
}

fn get_base_client(proxy_config: &Proxy) -> CustomResult<reqwest::Client, HttpClientError> {
    // Check if proxy configuration is provided using trait method
    if let Some(cache_key) = proxy_config.cache_key() {
        logger::debug!("Using proxy-specific client cache with key: {}", cache_key);
        
        // Create simple metrics attributes for proxy client tracking
        let http_proxy_host = extract_proxy_host(&proxy_config.http_url);
        let https_proxy_host = extract_proxy_host(&proxy_config.https_url);
            
        let proxy_metrics_tag = metric_attributes!(
            ("client_type", "proxy"),
            ("http_proxy_host", http_proxy_host),
            ("https_proxy_host", https_proxy_host)
        );
        
        // Use proxy-specific client cache
        let cache = PROXY_CLIENT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut cache_lock = cache.lock().map_err(|_| {
            error_stack::Report::new(HttpClientError::ClientConstructionFailed)
                .attach_printable("Failed to acquire proxy client cache lock")
        })?;
        
        if let Some(client) = cache_lock.get(&cache_key) {
            logger::debug!("Retrieved cached proxy client for key: {}", cache_key);
            metrics::HTTP_CLIENT_CACHE_HIT.add(1, &proxy_metrics_tag);
            return Ok(client.clone());
        }
        
        // Create new proxy client if not in cache
        logger::info!("Creating new proxy client for key: {} (http_proxy: {:?}, https_proxy: {:?})", 
            cache_key, 
            proxy_config.http_url, 
            proxy_config.https_url
        );
        
        metrics::HTTP_CLIENT_CACHE_MISS.add(1, &proxy_metrics_tag);
        
        let client = apply_mitm_certificate(get_client_builder(proxy_config)?, proxy_config)
            .build()
            .change_context(HttpClientError::ClientConstructionFailed)
            .attach_printable("Failed to construct proxy client")?;
        
        metrics::HTTP_CLIENT_CREATED.add(1, &proxy_metrics_tag);
        
        cache_lock.insert(cache_key.clone(), client.clone());
        logger::debug!("Cached new proxy client with key: {}", cache_key);
        Ok(client)
    } else {
        logger::debug!("No proxy configuration detected, using DEFAULT_CLIENT");
        
        let default_metrics_tag = metric_attributes!(("client_type", "default"));
        
        // Use DEFAULT_CLIENT for non-proxy scenarios
        let client = DEFAULT_CLIENT
            .get_or_try_init(|| {
                logger::info!("Initializing DEFAULT_CLIENT (no proxy configuration)");
                metrics::HTTP_CLIENT_CREATED.add(1, &default_metrics_tag);
                apply_mitm_certificate(get_client_builder(proxy_config)?, proxy_config)
                    .build()
                    .change_context(HttpClientError::ClientConstructionFailed)
                    .attach_printable("Failed to construct default client")
            })?
            .clone();
            
        metrics::HTTP_CLIENT_CACHE_HIT.add(1, &default_metrics_tag);
        Ok(client)
    }
}
