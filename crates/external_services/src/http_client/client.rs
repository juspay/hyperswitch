use std::time::Duration;

use base64::Engine;
use common_utils::consts::BASE64_ENGINE;
pub use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use masking::ExposeInterface;
use once_cell::sync::OnceCell;

static DEFAULT_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
use router_env::logger;

// We may need to use outbound proxy to connect to external world.
// Precedence will be the environment variables, followed by the config.
#[allow(missing_docs)]
pub fn create_client(
    proxy_config: &Proxy,
    client_certificate: Option<masking::Secret<String>>,
    client_certificate_key: Option<masking::Secret<String>>,
    ca_certificate: Option<masking::Secret<String>>,
) -> CustomResult<reqwest::Client, HttpClientError> {
    logger::info!(
        has_client_cert = client_certificate.is_some(),
        has_ca_cert = ca_certificate.is_some(),
        has_mitm_ca_cert = proxy_config.mitm_ca_cert.is_some(),
        "HTTP client configuration summary"
    );

    // Check if any certificates are provided
    let has_certificates = client_certificate.is_some() 
        || client_certificate_key.is_some() 
        || ca_certificate.is_some() 
        || proxy_config.mitm_ca_cert.is_some();

    // Use cached default client if no certificates are provided
    if !has_certificates {
        logger::debug!("Using cached default HTTP client (no certificates)");
        return get_base_client(proxy_config);
    }

    // Build custom client with certificates
    logger::debug!("Building custom HTTP client with certificates");

    // Step 1: Get base client builder with proxy configuration
    let mut client_builder = get_client_builder(proxy_config)?;

    // Step 2: Handle existing certificate cases (mutual TLS, one-way TLS)
    client_builder = handle_request_certificates(
        client_builder,
        client_certificate,
        client_certificate_key,
        ca_certificate,
    )?;

    // Step 3: ALWAYS apply MITM CA certificate if present
    client_builder = process_mitm_ca_certificate(proxy_config, client_builder)?;

    // Step 4: Build final client
    client_builder
        .use_rustls_tls()
        .build()
        .change_context(HttpClientError::ClientConstructionFailed)
        .attach_printable("Failed to construct HTTP client")
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

/// Handle request-level certificates (mutual TLS and one-way TLS)
fn handle_request_certificates(
    client_builder: reqwest::ClientBuilder,
    client_certificate: Option<masking::Secret<String>>,
    client_certificate_key: Option<masking::Secret<String>>,
    ca_certificate: Option<masking::Secret<String>>,
) -> CustomResult<reqwest::ClientBuilder, HttpClientError> {
    // Case 1: Mutual TLS with client certificate and key
    if let (Some(encoded_certificate), Some(encoded_certificate_key)) =
        (client_certificate.clone(), client_certificate_key.clone())
    {
        if ca_certificate.is_some() {
            logger::warn!("All of client certificate, client key, and CA certificate are provided. CA certificate will be ignored in mutual TLS setup.");
        }

        logger::debug!("Setting up mutual TLS (client cert + key)");
        let identity = create_identity_from_certificate_and_key(
            encoded_certificate.clone(),
            encoded_certificate_key,
        )?;
        let certificate_list = create_certificate(encoded_certificate)?;
        let client_builder = certificate_list
            .into_iter()
            .fold(client_builder, |builder, certificate| {
                builder.add_root_certificate(certificate)
            });
        return Ok(client_builder.identity(identity));
    }

    // Case 2: Use provided CA certificate for server authentication only (one-way TLS)
    if let Some(ca_pem) = ca_certificate {
        logger::debug!("Setting up one-way TLS (CA certificate)");
        let pem = ca_pem.expose().replace("\\r\\n", "\n"); // Fix escaped newlines
        let cert = reqwest::Certificate::from_pem(pem.as_bytes())
            .change_context(HttpClientError::ClientConstructionFailed)
            .attach_printable("Failed to parse CA certificate PEM block")?;
        return Ok(client_builder.add_root_certificate(cert));
    }

    // Case 3: No additional certificates
    logger::debug!("No additional certificates configured");
    Ok(client_builder)
}

/// Process MITM CA certificate for proxy authentication
fn process_mitm_ca_certificate(
    proxy_config: &Proxy,
    client_builder: reqwest::ClientBuilder,
) -> CustomResult<reqwest::ClientBuilder, HttpClientError> {
    if let Some(mitm_ca_cert) = &proxy_config.mitm_ca_cert {
        logger::debug!("Adding MITM CA certificate for proxy authentication");
        let pem = mitm_ca_cert.replace("\\r\\n", "\n"); // Fix escaped newlines
        let cert = reqwest::Certificate::from_pem(pem.as_bytes())
            .change_context(HttpClientError::ClientConstructionFailed)
            .attach_printable("Failed to parse MITM CA certificate PEM block - ensure certificate is in valid PEM format")?;
        Ok(client_builder.add_root_certificate(cert))
    } else {
        Ok(client_builder)
    }
}

fn get_base_client(proxy_config: &Proxy) -> CustomResult<reqwest::Client, HttpClientError> {
    Ok(DEFAULT_CLIENT
        .get_or_try_init(|| {
            let mut client_builder = get_client_builder(proxy_config)?;
            
            // Apply MITM CA certificate for the default client as well
            client_builder = process_mitm_ca_certificate(proxy_config, client_builder)?;
            
            client_builder
                .build()
                .change_context(HttpClientError::ClientConstructionFailed)
                .attach_printable("Failed to construct base client")
        })?
        .clone())
}
