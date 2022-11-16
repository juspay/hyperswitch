//!
//! Secret `Vec` types
//!
//! There is not alias type by design.

use alloc::vec::Vec;

#[cfg(feature = "serde")]
use super::{SerializableSecret, Serialize};

#[cfg(feature = "serde")]
impl<S: Serialize> SerializableSecret for Vec<S> {}
