#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use cards::CardNumber;
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use cards::{CardNumber, NetworkToken};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub type NetworkTokenNumber = CardNumber;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub type NetworkTokenNumber = NetworkToken;
