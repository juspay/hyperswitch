use std::{
    future::Future,
    panic::Location,
    task::{Context, Poll, Waker},
};

#[router_derive::track_async_interface]
#[async_trait::async_trait]
trait MerchantAccountInterface {
    type Error;

    async fn find(&self, id: &u32) -> Result<(u32, Option<u32>), Self::Error>;
}

struct Store;

trait AccountsStorageInterface: TrackedMerchantAccountInterface<Error = ()> {}

impl AccountsStorageInterface for Store {}

#[async_trait::async_trait]
impl MerchantAccountInterface for Store {
    type Error = ();

    async fn find(&self, id: &u32) -> Result<(u32, Option<u32>), Self::Error> {
        Ok((
            *id,
            router_env::call_origin::current_origin().map(Location::line),
        ))
    }
}

fn poll_now<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    let Poll::Ready(output) = future.as_mut().poll(&mut context) else {
        panic!("test future unexpectedly pending");
    };
    output
}

#[test]
fn tracked_trait_object_passes_the_call_site_to_the_raw_method() {
    let store: Box<dyn AccountsStorageInterface> = Box::new(Store);
    let id = 42;
    let expected_line = line!() + 1;
    let output = poll_now(store.find(&id));

    assert_eq!(output, Ok((id, Some(expected_line))));
    assert!(router_env::call_origin::current_origin().is_none());
}
