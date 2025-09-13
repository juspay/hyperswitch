//! Customers Interface for V1

#[cfg(feature = "v1")]
use super::{payments::ConnectorCustomer as PaymentsConnectorCustomer, ConnectorCommon};

#[cfg(feature = "v1")]
/// trait Subscriptions for V1
pub trait Subscriptions: ConnectorCommon + PaymentsConnectorCustomer {}

#[cfg(not(feature = "v1"))]
/// trait CreateCustomer (disabled when not V1)
pub trait ConnectorCustomer {}

#[cfg(not(feature = "v1"))]
/// trait Subscriptions (disabled when not V1)
pub trait Subscriptions {}
