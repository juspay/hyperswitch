//! Carries an operation's call location across async boundaries.

use std::{future::Future, panic::Location};

tokio::task_local! {
    static ORIGIN: &'static Location<'static>;
}

/// Run a future with a caller location in the current task's context.
pub fn with_origin<F: Future>(
    origin: &'static Location<'static>,
    future: F,
) -> impl Future<Output = F::Output> {
    ORIGIN.scope(origin, future)
}

/// Return the current caller location, if one was scoped.
pub fn current_origin() -> Option<&'static Location<'static>> {
    ORIGIN.try_with(|origin| *origin).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn origin_survives_await_and_is_removed_after_scope() {
        assert!(current_origin().is_none());

        let expected = line!() + 1;
        let observed = with_origin(Location::caller(), async {
            tokio::task::yield_now().await;
            current_origin().map(Location::line)
        })
        .await;

        assert_eq!(observed, Some(expected));
        assert!(current_origin().is_none());
    }
}
