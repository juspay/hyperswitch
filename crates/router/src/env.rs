#[doc(inline)]
pub use router_env::*;
pub mod logger {
    #[doc(inline)]
    pub use router_env::{log, logger::*};

    /// Setup logging sub-system.
    pub fn setup(
        conf: &config::Log,
        crates_to_filter: impl AsRef<[&'static str]>,
    ) -> TelemetryGuard {
        router_env::setup(conf, router_env::service_name!(), crates_to_filter)
    }
}
