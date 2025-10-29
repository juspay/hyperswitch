use common_utils::{consts, errors::CustomResult, request::Request};
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use request::{HeaderExt, RequestBuilderExt};
use router_env::{instrument, logger, tracing};
/// client module
pub mod client;
/// metrics module
pub mod metrics;
/// request module
pub mod request;
use std::{error::Error, time::Duration};

use common_utils::request::RequestContent;
pub use common_utils::request::{ContentType, Method, RequestBuilder};
use error_stack::ResultExt;

#[allow(missing_docs)]
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
    let request = {
        match request.method {
            Method::Get => client.get(url),
            Method::Post => {
                let client = client.post(url);
                match request.body {
                    Some(RequestContent::Json(payload)) => client.json(&payload),
                    Some(RequestContent::FormData((form, _))) => client.multipart(form),
                    Some(RequestContent::FormUrlEncoded(payload)) => client.form(&payload),
                    Some(RequestContent::Xml(payload)) => {
                        let body = quick_xml::se::to_string(&payload)
                            .change_context(HttpClientError::BodySerializationFailed)?;
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
                    Some(RequestContent::Xml(payload)) => {
                        let body = quick_xml::se::to_string(&payload)
                            .change_context(HttpClientError::BodySerializationFailed)?;
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
                    Some(RequestContent::Xml(payload)) => {
                        let body = quick_xml::se::to_string(&payload)
                            .change_context(HttpClientError::BodySerializationFailed)?;
                        client.body(body).header("Content-Type", "application/xml")
                    }
                    Some(RequestContent::RawBytes(payload)) => client.body(payload),
                    None => client,
                }
            }
            Method::Delete => client.delete(url),
        }
        .add_headers(headers)
        .timeout(Duration::from_secs(
            option_timeout_secs.unwrap_or(consts::REQUEST_TIME_OUT),
        ))
    };

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
    match response {
        Ok(response) => Ok(response),
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
        err @ Err(_) => err,
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
