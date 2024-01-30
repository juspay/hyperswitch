//!
//! Storing [layer](https://docs.rs/tracing-subscriber/0.3.15/tracing_subscriber/layer/trait.Layer.html) for Router.
//!

use std::{collections::HashMap, fmt, time::Instant};

use tracing::{
    field::{Field, Visit},
    span::{Attributes, Record},
    Id, Subscriber,
};
use tracing_subscriber::{layer::Context, Layer};

/// Storage to store key value pairs of spans.
#[derive(Clone, Debug)]
pub struct StorageSubscription;

/// Storage to store key value pairs of spans.
/// When new entry is crated it stores it in [HashMap] which is owned by `extensions`.
#[derive(Clone, Debug)]
pub struct Storage<'a> {
    /// Hash map to store values.
    pub values: HashMap<&'a str, serde_json::Value>,
}

impl<'a> Storage<'a> {
    /// Default constructor.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Default constructor.
impl Default for Storage<'_> {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
        }
    }
}

/// Visitor to store entry.
impl Visit for Storage<'_> {
    /// A i64.
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    /// A u64.
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    /// A 64-bit floating point.
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    /// A boolean.
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    /// A string.
    fn record_str(&mut self, field: &Field, value: &str) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    /// Otherwise.
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        match field.name() {
            // Skip fields which are already handled
            name if name.starts_with("log.") => (),
            name if name.starts_with("r#") => {
                self.values.insert(
                    #[allow(clippy::expect_used)]
                    name.get(2..)
                        .expect("field name must have a minimum of two characters"),
                    serde_json::Value::from(format!("{value:?}")),
                );
            }
            name => {
                self.values
                    .insert(name, serde_json::Value::from(format!("{value:?}")));
            }
        };
    }
}

const PERSISTENT_KEYS: [&str; 4] = ["payment_id", "connector_name", "merchant_id", "flow"];

impl<S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>> Layer<S>
    for StorageSubscription
{
    /// On new span.
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        #[allow(clippy::expect_used)]
        let span = ctx.span(id).expect("No span");
        let mut extensions = span.extensions_mut();

        let mut visitor = if let Some(parent_span) = span.parent() {
            let mut extensions = parent_span.extensions_mut();
            extensions
                .get_mut::<Storage<'_>>()
                .map(|v| v.to_owned())
                .unwrap_or_default()
        } else {
            Storage::default()
        };

        attrs.record(&mut visitor);
        extensions.insert(visitor);
    }

    /// On additional key value pairs store it.
    fn on_record(&self, span: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        #[allow(clippy::expect_used)]
        let span = ctx.span(span).expect("No span");
        let mut extensions = span.extensions_mut();
        #[allow(clippy::expect_used)]
        let visitor = extensions
            .get_mut::<Storage<'_>>()
            .expect("The span does not have storage");
        values.record(visitor);
    }

    /// On enter store time.
    fn on_enter(&self, span: &Id, ctx: Context<'_, S>) {
        #[allow(clippy::expect_used)]
        let span = ctx.span(span).expect("No span");
        let mut extensions = span.extensions_mut();
        if extensions.get_mut::<Instant>().is_none() {
            extensions.insert(Instant::now());
        }
    }

    /// On close create an entry about how long did it take.
    fn on_close(&self, span: Id, ctx: Context<'_, S>) {
        #[allow(clippy::expect_used)]
        let span = ctx.span(&span).expect("No span");

        let elapsed_milliseconds = {
            let extensions = span.extensions();
            extensions
                .get::<Instant>()
                .map(|i| i.elapsed().as_millis())
                .unwrap_or(0)
        };

        if let Some(s) = span.extensions().get::<Storage<'_>>() {
            s.values.iter().for_each(|(k, v)| {
                if PERSISTENT_KEYS.contains(k) {
                    span.parent().and_then(|p| {
                        p.extensions_mut()
                            .get_mut::<Storage<'_>>()
                            .map(|s| s.values.insert(k, v.to_owned()))
                    });
                }
            })
        };

        let mut extensions_mut = span.extensions_mut();
        #[allow(clippy::expect_used)]
        let visitor = extensions_mut
            .get_mut::<Storage<'_>>()
            .expect("No visitor in extensions");

        if let Ok(elapsed) = serde_json::to_value(elapsed_milliseconds) {
            visitor.values.insert("elapsed_milliseconds", elapsed);
        }
    }
}
