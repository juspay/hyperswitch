use common_utils::{consts, errors::CustomResult, request::Request};
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use quick_xml::{
    events::{BytesDecl, BytesText, Event},
    Writer,
};
use request::{HeaderExt, RequestBuilderExt};
use router_env::{instrument, logger};
/// client module
pub mod client;
/// metrics module
pub mod metrics;
/// outbound_destination module
pub mod outbound_destination;
/// request module
pub mod request;

/// Boundary capture/replay for the outgoing HTTP client.
///
/// Public so the tape-conformance gate can reconstruct a recorded response
/// through the SAME codec replay runs, rather than reimplementing what that
/// codec accepts — a second copy of the rule is free to drift from the one that
/// actually decides whether a worker lives.
#[cfg(feature = "deja")]
pub mod boundary;

use std::{error::Error, time::Duration};

pub use common_utils::request::{ContentType, Method, RequestBuilder};
use common_utils::request::{RequestContent, XmlConfig};
use error_stack::ResultExt;

#[allow(missing_docs)]
#[instrument(skip_all)]
pub fn serialize_to_xml_bytes<T: serde::Serialize>(
    item: &T,
    config: Option<XmlConfig>,
) -> Result<Vec<u8>, error_stack::Report<HttpClientError>> {
    let mut xml_bytes = Vec::new();
    let mut writer = Writer::new(std::io::Cursor::new(&mut xml_bytes));

    if let Some(xml_config) = config {
        let xml_version = xml_config.xml_version;
        let xml_encoding = xml_config.xml_encoding.as_deref();
        let xml_standalone = xml_config.xml_standalone.as_deref();
        let xml_doc_type = xml_config.xml_doc_type.as_deref();

        writer
            .write_event(Event::Decl(BytesDecl::new(
                xml_version.as_str(),
                xml_encoding,
                xml_standalone,
            )))
            .change_context(HttpClientError::BodySerializationFailed)
            .attach_printable("Failed to write XML declaration")?;

        if let Some(xml_doc_type_data) = xml_doc_type {
            writer
                .write_event(Event::DocType(BytesText::from_escaped(xml_doc_type_data)))
                .change_context(HttpClientError::BodySerializationFailed)
                .attach_printable("Failed to write XML DOCTYPE declaration")?;
        }
    }

    let xml_body = quick_xml::se::to_string(item)
        .change_context(HttpClientError::BodySerializationFailed)
        .attach_printable("Failed to serialize XML body")?;

    writer
        .write_event(Event::Text(BytesText::from_escaped(xml_body)))
        .change_context(HttpClientError::BodySerializationFailed)
        .attach_printable("Failed to write XML body text")?;

    Ok(xml_bytes)
}

#[allow(missing_docs)]
#[cfg_attr(
    feature = "deja",
    deja::http(outgoing,
        component = "external_services::http_client",
        operation = "send_request",
        correlation = boundary::request_id(&request),
        // The deja handoff: expose the Secret-wrapped payload so the tape
        // records full fidelity (Secret only redacts Debug/log output).
        args = hyperswitch_masking::ExposeInterface::expose(boundary::request_args(&request, option_timeout_secs)),
        // Rebuild the recorded reqwest::Response (status+headers+body) so the
        // outgoing call (e.g. Stripe) is served from the lookup table with no
        // network. A recorded value this build cannot reconstruct fail-stops the
        // request; it is not a silent fallback to a live call.
        owned_codec = boundary::HttpResponseCodec,
        // A miss answers as a send failure: nobody answered, and a fabricated status
        // could report a payment outcome that never happened. The url comes from the
        // recorded args because the real call moves `request`.
        on_miss = Err(HttpClientError::RequestNotSent(
            format!(
                "deja: no recorded response for this outbound call (url {}, occurrence {})",
                __deja_miss
                    .args
                    .get("url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown"),
                __deja_miss.occurrence,
            ),
        )
        .into()),
    )
)]
#[instrument(skip_all)]
pub async fn send_request(
    client_proxy: &Proxy,
    request: Request,
    option_timeout_secs: Option<u64>,
) -> CustomResult<reqwest::Response, HttpClientError> {
    logger::info!(method=?request.method, headers=?request.headers, payload=?request.body, ?request);

    let url = url::Url::parse(&request.url).change_context(HttpClientError::UrlParsingFailed)?;

    let client = client::create_client(
        client_proxy,
        request.certificate,
        request.certificate_key,
        request.ca_certificate,
    )?;

    let headers = request.headers.construct_header_map()?;
    let metrics_tag = router_env::metric_attributes!((
        consts::METRICS_HOST_TAG_NAME,
        url.host_str().unwrap_or_default().to_owned()
    ));

    let method = request.method;
    let request_builder = {
        match request.method {
            Method::Get => client.get(url),
            Method::Post => {
                let client = client.post(url);
                match request.body {
                    Some(RequestContent::Json(payload)) => client.json(&payload),
                    Some(RequestContent::FormData((form, _))) => client.multipart(form),
                    Some(RequestContent::FormUrlEncoded(payload)) => client.form(&payload),
                    Some(RequestContent::Xml(payload, config)) => {
                        let body = serialize_to_xml_bytes(&payload, config)?;
                        client.body(body).header("Content-Type", "application/xml")
                    }
                    Some(RequestContent::RawBytes(payload)) => client.body(payload),
                    None => client,
                }
            }
            Method::Put => {
                let client = client.put(url);
                match request.body {
                    Some(RequestContent::Json(payload)) => client.json(&payload),
                    Some(RequestContent::FormData((form, _))) => client.multipart(form),
                    Some(RequestContent::FormUrlEncoded(payload)) => client.form(&payload),
                    Some(RequestContent::Xml(payload, config)) => {
                        let body = serialize_to_xml_bytes(&payload, config)?;
                        client.body(body).header("Content-Type", "application/xml")
                    }
                    Some(RequestContent::RawBytes(payload)) => client.body(payload),
                    None => client,
                }
            }
            Method::Patch => {
                let client = client.patch(url);
                match request.body {
                    Some(RequestContent::Json(payload)) => client.json(&payload),
                    Some(RequestContent::FormData((form, _))) => client.multipart(form),
                    Some(RequestContent::FormUrlEncoded(payload)) => client.form(&payload),
                    Some(RequestContent::Xml(payload, config)) => {
                        let body = serialize_to_xml_bytes(&payload, config)?;
                        client.body(body).header("Content-Type", "application/xml")
                    }
                    Some(RequestContent::RawBytes(payload)) => client.body(payload),
                    None => client,
                }
            }
            Method::Delete => client.delete(url),
        }
    };

    let request = match request.query_params.as_ref() {
        Some(params) => request_builder.query(params),
        None => request_builder,
    };

    let request = request.add_headers(headers).timeout(Duration::from_secs(
        option_timeout_secs.unwrap_or(consts::REQUEST_TIME_OUT),
    ));

    // Retain the typed cause so callers can distinguish transport errors from
    // request-construction errors within RequestNotSent, including on retries.
    let map_request_error = |error: reqwest::Error| {
        let context = match &error {
            error if error.is_timeout() => {
                metrics::REQUEST_BUILD_FAILURE.add(1, metrics_tag);
                HttpClientError::RequestTimeoutReceived
            }
            error if is_connection_closed_before_message_could_complete(error) => {
                metrics::REQUEST_BUILD_FAILURE.add(1, metrics_tag);
                HttpClientError::ConnectionClosedIncompleteMessage
            }
            _ => HttpClientError::RequestNotSent(error.to_string()),
        };
        error_stack::Report::new(error).change_context(context)
    };

    // We cannot clone the request type, because it has Form trait which is not cloneable. So we are cloning the request builder here.
    let cloned_send_request = request.try_clone().map(|cloned_request| async {
        cloned_request
            .send()
            .await
            .map_err(map_request_error)
            .attach_printable("Unable to send request to connector")
    });

    let send_request = async {
        request
            .send()
            .await
            .map_err(map_request_error)
            .attach_printable("Unable to send request to connector")
    };

    let response = common_utils::metrics::utils::record_operation_time(
        send_request,
        &metrics::EXTERNAL_REQUEST_TIME,
        metrics_tag,
    )
    .await;
    // Retry a GET once if the response is connection closed.
    //
    // This is just due to the racy nature of networking.
    // hyper has a connection pool of idle connections, and it selected one to send your request.
    // Most of the time, hyper will receive the server’s FIN and drop the dead connection from its pool.
    // But occasionally, a connection will be selected from the pool
    // and written to at the same time the server is deciding to close the connection.
    // Since hyper already wrote some of the request, the server may have acted on it already,
    // so only a GET is safe to send again; any other request could be applied twice.
    let response = match response {
        Err(error)
            if error.current_context() == &HttpClientError::ConnectionClosedIncompleteMessage
                && method == Method::Get =>
        {
            metrics::AUTO_RETRY_CONNECTION_CLOSED.add(1, metrics_tag);
            match cloned_send_request {
                Some(cloned_request) => {
                    logger::info!(
                        "Retrying request due to connection closed before message could complete"
                    );
                    common_utils::metrics::utils::record_operation_time(
                        cloned_request,
                        &metrics::EXTERNAL_REQUEST_TIME,
                        metrics_tag,
                    )
                    .await
                }
                None => {
                    logger::info!("Retrying request due to connection closed before message could complete failed as request is not cloneable");
                    Err(error)
                }
            }
        }
        response => response,
    };

    response
}

fn is_connection_closed_before_message_could_complete(error: &reqwest::Error) -> bool {
    let mut source = error.source();
    while let Some(err) = source {
        if let Some(hyper_err) = err.downcast_ref::<hyper::Error>() {
            if hyper_err.is_incomplete_message() {
                return true;
            }
        }
        source = err.source();
    }
    false
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use tokio::{io::AsyncReadExt, net::TcpListener};

    use super::*;

    /// A server that reads each request and closes the connection without answering,
    /// counting the connections it accepted.
    async fn closing_server() -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let accepted = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&accepted);
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                let mut buf = [0u8; 4096];
                let _ = socket.read(&mut buf).await;
            }
        });
        (url, accepted)
    }

    async fn send_to_closing_server(method: Method) -> (HttpClientError, usize) {
        let (url, accepted) = closing_server().await;
        let request = RequestBuilder::new().method(method).url(&url).build();
        let error = send_request(&Proxy::default(), request, Some(5))
            .await
            .expect_err("server never answers");
        (
            error.current_context().clone(),
            accepted.load(Ordering::SeqCst),
        )
    }

    #[tokio::test]
    async fn connection_closed_post_is_not_sent_again() {
        let (error, connections) = send_to_closing_server(Method::Post).await;
        assert_eq!(error, HttpClientError::ConnectionClosedIncompleteMessage);
        assert_eq!(connections, 1);
    }

    #[tokio::test]
    async fn connection_closed_get_is_sent_again() {
        let (error, connections) = send_to_closing_server(Method::Get).await;
        assert_eq!(error, HttpClientError::ConnectionClosedIncompleteMessage);
        assert_eq!(connections, 2);
    }
}
