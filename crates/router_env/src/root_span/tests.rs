use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing_subscriber::{layer::Context, prelude::*, Layer, Registry};

use super::*;

#[derive(Clone)]
struct CaptureTarget(Arc<Mutex<Vec<String>>>);

impl Visit for CaptureTarget {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "http.target" {
            self.0.lock().expect("targets").push(format!("{value:?}"));
        }
    }
}

impl Layer<Registry> for CaptureTarget {
    fn on_new_span(
        &self,
        attributes: &tracing::span::Attributes<'_>,
        _: &tracing::Id,
        _: Context<'_, Registry>,
    ) {
        attributes.record(&mut self.clone());
    }
}

#[test]
fn unmatched_callback_target_is_generic_before_subscribers_observe_the_span() {
    let capture = CaptureTarget(Arc::new(Mutex::new(Vec::new())));
    let subscriber = Registry::default().with(capture.clone());
    tracing::subscriber::with_default(subscriber, || {
        let request = actix_web::test::TestRequest::get()
                .uri("/payments/pay/merchant/redirect/token/d9dc53cc-d1a1-4f57-9077-d20c2d614daa/complete/paybox/extra?connector_param=value")
                .to_srv_request();
        request
            .extensions_mut()
            .insert(tracing_actix_web::root_span_macro::private::generate_request_id());
        let _span = CustomRootSpanBuilder::on_request_start(&request);
        assert!(request
            .path()
            .contains("d9dc53cc-d1a1-4f57-9077-d20c2d614daa"));
        assert_eq!(request.query_string(), "connector_param=value");
    });
    let targets = capture.0.lock().expect("targets");
    assert_eq!(targets.len(), 1);
    let target = targets.first().expect("root span target");
    assert!(!target.contains("d9dc53cc"));
    assert!(target.contains("/unmatched"));
    assert!(!target.contains("connector_param"));
}

#[actix_web::test]
async fn matched_callback_target_keeps_ids_and_masks_only_the_token() {
    use actix_web::{test, web, App, HttpResponse};

    let capture = CaptureTarget(Arc::new(Mutex::new(Vec::new())));
    let subscriber = Registry::default().with(capture.clone());
    let _guard = tracing::subscriber::set_default(subscriber);
    let app = test::init_service(
            App::new()
                .wrap(tracing_actix_web::TracingLogger::<CustomRootSpanBuilder>::new())
                .service(web::scope("/payments").service(
                    web::resource("/{payment_id}/{merchant_id}/redirect/token/{callback_token}/complete/{connector}")
                        .route(web::get().to(|| async { HttpResponse::Ok().finish() })),
                )),
        )
        .await;

    let request = test::TestRequest::get()
            .uri("/payments/pay/merchant/redirect/token/d9dc53cc-d1a1-4f57-9077-d20c2d614daa/complete/paybox?connector_param=value")
            .to_request();
    let response = test::call_service(&app, request).await;
    assert!(response.status().is_success());

    let targets = capture.0.lock().expect("targets");
    assert_eq!(targets.len(), 1);
    let target = targets.first().expect("root span target");
    assert!(!target.contains("d9dc53cc"));
    assert!(
        target.contains("/payments/pay/merchant/redirect/token/{callback_token}/complete/paybox")
    );
    assert!(!target.contains("connector_param"));
}
