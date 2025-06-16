#[cfg(feature = "v1")]
use cards::CardNumber;
#[cfg(feature = "v2")]
use cards::{CardNumber, NetworkToken};

#[cfg(feature = "v1")]
pub type NetworkTokenNumber = CardNumber;

#[cfg(feature = "v2")]
pub type NetworkTokenNumber = NetworkToken;
