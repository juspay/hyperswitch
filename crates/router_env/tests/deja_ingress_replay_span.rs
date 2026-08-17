#![cfg(feature = "deja")]

use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, Mutex},
};

use actix_web::{test, web, App, HttpResponse};
use router_env::request_id::{IdReuse, RequestIdentifier};
use tracing::{
    field::{Field, Visit},
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{layer::Context as LayerContext, prelude::*, registry::LookupSpan, Layer};

struct EmptyLookup;

impl deja::LookupTableSource for EmptyLookup {
    fn load(&mut self) -> std::io::Result<deja::LookupTable> {
        Ok(deja::LookupTable {
            recording_id: "empty".to_string(),
            policy_version: 1,
            entries: Vec::new(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CapturedSpan {
    name: String,
    fields: BTreeMap<String, String>,
}

#[derive(Default)]
struct FieldVisitor(BTreeMap<String, String>);

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}

#[derive(Clone)]
struct CaptureEnteredIngressSpans(Arc<Mutex<Vec<CapturedSpan>>>);

impl<S> Layer<S> for CaptureEnteredIngressSpans
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attributes: &Attributes<'_>, id: &Id, context: LayerContext<'_, S>) {
        if attributes.metadata().name() != "deja::http_incoming" {
            return;
        }

        let mut visitor = FieldVisitor::default();
        attributes.record(&mut visitor);
        context
            .span(id)
            .expect("new ingress span must be available to the registry")
            .extensions_mut()
            .insert(CapturedSpan {
                name: attributes.metadata().name().to_string(),
                fields: visitor.0,
            });
    }

    fn on_enter(&self, id: &Id, context: LayerContext<'_, S>) {
        let span = context
            .span(id)
            .expect("entered span must be available to the registry");
        let Some(captured) = span.extensions().get::<CapturedSpan>().cloned() else {
            return;
        };
        self.0.lock().expect("capture lock").push(captured);
    }
}

#[actix_web::test]
async fn replay_ingress_enters_http_incoming_span_end_to_end() {
    let entered = Arc::new(Mutex::new(Vec::new()));
    tracing_subscriber::registry()
        .with(deja::DejaCorrelationLayer::new())
        .with(CaptureEnteredIngressSpans(Arc::clone(&entered)))
        .try_init()
        .expect("install replay ingress subscriber (own process)");

    let hook = deja::LookupTableHook::from_source(EmptyLookup, deja::InMemoryObservedSink::new())
        .expect("construct lookup replay hook");
    deja::set_global_runtime_hook(Some(deja::RuntimeHook::LookupReplay(hook)))
        .expect("install replay hook (own process)");
    assert!(
        deja::runtime_mode().is_replay(),
        "the request must exercise the replay-mode middleware branch"
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestIdentifier::new("x-request-id").use_incoming_id(IdReuse::UseIncoming))
            .route(
                "/payments/confirm",
                web::patch().to(|| async { HttpResponse::Ok().body("confirmed") }),
            ),
    )
    .await;

    let request = test::TestRequest::patch()
        .uri("/payments/confirm")
        .insert_header(("x-request-id", "req-replay-123"))
        .to_request();
    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    let body = test::read_body(response).await;
    assert_eq!(&body[..], b"confirmed");

    let entered = entered.lock().expect("capture lock");
    let expected = CapturedSpan {
        name: "deja::http_incoming".to_string(),
        fields: BTreeMap::from([
            ("method".to_string(), "PATCH".to_string()),
            ("path".to_string(), "/payments/confirm".to_string()),
            ("request_id".to_string(), "req-replay-123".to_string()),
        ]),
    };
    assert!(
        !entered.is_empty(),
        "replay middleware must enter at least one deja::http_incoming span"
    );
    for captured in entered.iter() {
        assert_eq!(captured, &expected);
    }
}
