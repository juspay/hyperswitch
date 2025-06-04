#[cfg(
    any(feature = "v1", feature = "v2")
 )]
use cards::CardNumber;
#[cfg(feature = "v2")]
use cards::{CardNumber, NetworkToken};

#[cfg(
    any(feature = "v1", feature = "v2")
 )]
pub type NetworkTokenNumber = CardNumber;

#[cfg(feature = "v2")]
pub type NetworkTokenNumber = NetworkToken;
