#[doc(inline)]
pub use router_env::*;

pub mod logger {
    #[doc(inline)]
    pub use router_env::{log, logger::*};

    /// Setup logging sub-system
    pub fn setup(conf: &config::Log) -> TelemetryGuard {
        router_env::setup(
            conf,
            router_env::service_name!(),
            [router_env::service_name!()],
        )
    }
}
