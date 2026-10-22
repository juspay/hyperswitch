//! The chain of active `#[instrument]` spans, for records that need to say
//! *where* they came from.

use std::fmt::Write;

use tracing_subscriber::registry::LookupSpan;

/// Root-to-leaf names of the active spans, each with its source location:
///
/// ```text
/// ROOT_SPAN@crates/router_env/src/root_span.rs:28>payments_confirm@crates/router/src/core/payments.rs:1768
/// ```
///
/// `fn` on a record names the function that logged it, not how execution
/// reached it -- the half that matters when the same work repeats from two call
/// sites. Locations are where each function is *defined*, which is what a span
/// carries; an async call site is not recoverable at runtime, and the
/// definition is what you open anyway.
///
/// Call it inside a log macro's field list, so the walk only happens for a
/// record that is emitted. `None` below two spans (one is what `fn` already
/// reports) or under a subscriber not built on a
/// [`tracing_subscriber::Registry`].
pub fn current_span_path() -> Option<String> {
    tracing::Span::current()
        .with_subscriber(|(id, dispatch)| {
            let registry = dispatch.downcast_ref::<tracing_subscriber::Registry>()?;

            let mut path = String::new();
            let mut depth = 0usize;
            for ancestor in registry.span(id)?.scope().from_root() {
                if depth > 0 {
                    path.push('>');
                }
                depth += 1;

                let metadata = ancestor.metadata();
                path.push_str(metadata.name());
                if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
                    // Ignore a formatting failure: writing to a String cannot
                    // fail, and a path missing a location is still worth having.
                    let _ = write!(path, "@{file}:{line}");
                }
            }

            (depth > 1).then_some(path)
        })
        .flatten()
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::layer::SubscriberExt;

    use super::*;

    #[test]
    fn walks_the_chain_from_root_with_locations() {
        tracing::subscriber::with_default(tracing_subscriber::registry(), || {
            let outer = tracing::info_span!("outer");
            let _outer = outer.enter();
            let inner = tracing::info_span!("inner");
            let _inner = inner.enter();

            let path = current_span_path().expect("two spans are active");
            let (first, second) = path.split_once('>').expect("two elements");
            assert!(first.starts_with("outer@"), "{path}");
            assert!(second.starts_with("inner@"), "{path}");
            // Each element carries where its span is defined.
            assert!(first.contains("span_path.rs:"), "{path}");
            assert!(second.contains("span_path.rs:"), "{path}");
        });
    }

    /// The production subscriber is layers over a `Registry`, so the walk has to
    /// find the registry through `Layered` rather than only as the subscriber
    /// itself.
    #[test]
    fn finds_the_registry_under_layers() {
        let subscriber = tracing_subscriber::registry()
            .with(tracing_subscriber::filter::LevelFilter::TRACE)
            .with(tracing_subscriber::filter::LevelFilter::TRACE);
        tracing::subscriber::with_default(subscriber, || {
            let outer = tracing::info_span!("outer");
            let _outer = outer.enter();
            let inner = tracing::info_span!("inner");
            let _inner = inner.enter();

            assert!(current_span_path().is_some());
        });
    }

    #[test]
    fn a_single_span_is_not_a_path() {
        tracing::subscriber::with_default(tracing_subscriber::registry(), || {
            let only = tracing::info_span!("only");
            let _only = only.enter();
            assert!(current_span_path().is_none());
        });
    }

    #[test]
    fn no_span_is_not_a_path() {
        tracing::subscriber::with_default(tracing_subscriber::registry(), || {
            assert!(current_span_path().is_none());
        });
    }

    /// A subscriber that is not registry-backed cannot be walked; that must be
    /// `None` rather than a panic.
    #[test]
    fn a_registryless_subscriber_yields_nothing() {
        tracing::subscriber::with_default(tracing::subscriber::NoSubscriber::default(), || {
            assert!(current_span_path().is_none());
        });
    }
}
