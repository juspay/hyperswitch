use common_utils::{consts, errors::CustomResult, request::Request};
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use quick_xml::{
    events::{BytesDecl, BytesText, Event},
    Writer,
};
use request::{HeaderExt, RequestBuilderExt};
use router_env::{instrument, logger, tracing};
/// client module
pub mod client;
/// metrics module
pub mod metrics;
/// request module
pub mod request;
#[cfg(feature = "deja")]
mod semantic_boundary;
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
        correlation = semantic_boundary::request_id(&request),
        args = semantic_boundary::request_args(&request, option_timeout_secs),
        // Rebuild the recorded reqwest::Response (status+headers+body) so the
        // outgoing call (e.g. Stripe) is served from the lookup table with no
        // network. A recorded error reconstructs to None -> falls through to live.
        codec = semantic_boundary::HttpResponseCodec,
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

    // We cannot clone the request type, because it has Form trait which is not cloneable. So we are cloning the request builder here.
    let cloned_send_request = request.try_clone().map(|cloned_request| async {
        cloned_request
            .send()
            .await
            .map_err(|error| match error {
                error if error.is_timeout() => {
                    metrics::REQUEST_BUILD_FAILURE.add(1, metrics_tag);
                    HttpClientError::RequestTimeoutReceived
                }
                error if is_connection_closed_before_message_could_complete(&error) => {
                    metrics::REQUEST_BUILD_FAILURE.add(1, metrics_tag);
                    HttpClientError::ConnectionClosedIncompleteMessage
                }
                _ => HttpClientError::RequestNotSent(error.to_string()),
            })
            .attach_printable("Unable to send request to connector")
    });

    let send_request = async {
        request
            .send()
            .await
            .map_err(|error| match error {
                error if error.is_timeout() => {
                    metrics::REQUEST_BUILD_FAILURE.add(1, metrics_tag);
                    HttpClientError::RequestTimeoutReceived
                }
                error if is_connection_closed_before_message_could_complete(&error) => {
                    metrics::REQUEST_BUILD_FAILURE.add(1, metrics_tag);
                    HttpClientError::ConnectionClosedIncompleteMessage
                }
                _ => HttpClientError::RequestNotSent(error.to_string()),
            })
            .attach_printable("Unable to send request to connector")
    };

    let response = common_utils::metrics::utils::record_operation_time(
        send_request,
        &metrics::EXTERNAL_REQUEST_TIME,
        metrics_tag,
    )
    .await;
    // Retry once if the response is connection closed.
    //
    // This is just due to the racy nature of networking.
    // hyper has a connection pool of idle connections, and it selected one to send your request.
    // Most of the time, hyper will receive the server’s FIN and drop the dead connection from its pool.
    // But occasionally, a connection will be selected from the pool
    // and written to at the same time the server is deciding to close the connection.
    // Since hyper already wrote some of the request,
    // it can’t really retry it automatically on a new connection, since the server may have acted already
    let response = match response {
        Err(error)
            if error.current_context() == &HttpClientError::ConnectionClosedIncompleteMessage =>
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

    #[cfg(feature = "deja")]
    {
        match response {
            Ok(response) if semantic_boundary::is_active() => {
                semantic_boundary::response_with_captured_body(response).await
            }
            response => response,
        }
    }

    #[cfg(not(feature = "deja"))]
    {
        response
    }
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
