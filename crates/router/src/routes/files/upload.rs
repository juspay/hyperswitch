use actix_multipart::Multipart;
use actix_web::{error::ErrorPayloadTooLarge, web, HttpRequest};

// Allow the largest supported file (Adyen's 10 MB images), plus multipart metadata.
const BODY_LIMIT: usize = 11 * 1024 * 1024;

pub(super) async fn multipart(
    req: &HttpRequest,
    payload: web::Payload,
) -> actix_web::Result<Multipart> {
    // Count actual bytes before parsing, including fields we would otherwise ignore.
    // This also bounds requests without Content-Length (e.g. chunked transfers).
    let body = payload
        .to_bytes_limited(BODY_LIMIT)
        .await
        .map_err(ErrorPayloadTooLarge)??;
    Ok(Multipart::new(
        req.headers(),
        futures::stream::once(async move { Ok(body) }),
    ))
}

#[cfg(test)]
mod tests {
    use actix_web::{dev::Payload, http::StatusCode, test, web, App, FromRequest, HttpResponse};
    use futures::{StreamExt, TryStreamExt};

    use super::{multipart, BODY_LIMIT};

    async fn read_upload(
        req: actix_web::HttpRequest,
        payload: web::Payload,
    ) -> actix_web::Result<HttpResponse> {
        let mut multipart = multipart(&req, payload).await?;
        let mut contents = Vec::new();
        while let Some(mut field) = multipart.try_next().await? {
            while let Some(chunk) = field.try_next().await? {
                contents.extend_from_slice(&chunk);
            }
        }
        Ok(HttpResponse::Ok().body(contents))
    }

    #[actix_web::test]
    async fn accepts_supported_file_and_rejects_oversized_body_before_parsing() {
        let app = test::init_service(App::new().route("/files", web::post().to(read_upload))).await;
        let prefix = b"--boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"image.jpg\"\r\n\r\n";
        let suffix = b"\r\n--boundary--\r\n";
        for (size, expected_status) in [
            (10_000_000, StatusCode::OK),
            (BODY_LIMIT - prefix.len() - suffix.len(), StatusCode::OK),
            (
                BODY_LIMIT + 1 - prefix.len() - suffix.len(),
                StatusCode::PAYLOAD_TOO_LARGE,
            ),
        ] {
            let mut body = prefix.to_vec();
            body.resize(body.len() + size, b'x');
            body.extend_from_slice(suffix);
            let req = test::TestRequest::post()
                .uri("/files")
                .insert_header(("Content-Type", "multipart/form-data; boundary=boundary"))
                .set_payload(body)
                .to_request();
            let response = test::call_service(&app, req).await;
            assert_eq!(response.status(), expected_status);
            if expected_status == StatusCode::OK {
                assert_eq!(
                    test::read_body(response).await,
                    web::Bytes::from(vec![b'x'; size])
                );
            }
        }

        // Invalid multipart headers cannot bypass the body cap or trigger parsing first.
        let req = test::TestRequest::post()
            .uri("/files")
            .set_payload(vec![b'x'; BODY_LIMIT + 1])
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[actix_web::test]
    async fn stops_reading_unknown_length_stream_when_total_exceeds_limit() {
        // No Content-Length, and each chunk is below the cap. The cap must count the
        // aggregate body, not individual chunks, and stop polling on overflow.
        let req = test::TestRequest::post().to_http_request();
        let chunks = futures::stream::iter([
            Ok(web::Bytes::from(vec![b'x'; BODY_LIMIT / 2])),
            Ok(web::Bytes::from(vec![b'x'; BODY_LIMIT / 2 + 1])),
        ])
        .chain(futures::stream::poll_fn(|_| {
            panic!("body was polled after exceeding the limit")
        }));
        let mut payload = Payload::from(chunks.boxed_local());
        let payload = web::Payload::from_request(&req, &mut payload)
            .await
            .unwrap();
        let err = multipart(&req, payload).await.err().unwrap();
        assert_eq!(
            err.as_response_error().status_code(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[actix_web::test]
    async fn preserves_payload_stream_errors() {
        let req = test::TestRequest::post().to_http_request();
        let chunks =
            futures::stream::once(async { Err(actix_web::error::PayloadError::Incomplete(None)) });
        let mut payload = Payload::from(chunks.boxed_local());
        let payload = web::Payload::from_request(&req, &mut payload)
            .await
            .unwrap();
        let err = multipart(&req, payload).await.err().unwrap();
        assert_eq!(
            err.as_response_error().status_code(),
            StatusCode::BAD_REQUEST
        );
    }
}
