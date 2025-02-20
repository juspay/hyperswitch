//! Optional `Secret` wrapper type for the `bytes::BytesMut` crate.

use core::fmt;

use bytes::BytesMut;
#[cfg(all(feature = "bytes", feature = "serde"))]
use serde::de::{self, Deserialize};

use super::{PeekInterface, ZeroizableSecret};

/// Instance of [`BytesMut`] protected by a type that impls the [`ExposeInterface`]
/// trait like `Secret<T>`.
///
/// Because of the nature of how the `BytesMut` type works, it needs some special
/// care in order to have a proper zeroizing drop handler.
#[derive(Clone)]
#[cfg_attr(docsrs, cfg(feature = "bytes"))]
pub struct SecretBytesMut(BytesMut);

impl SecretBytesMut {
    /// Wrap bytes in `SecretBytesMut`
    pub fn new(bytes: impl Into<BytesMut>) -> Self {
        Self(bytes.into())
    }
}

impl PeekInterface<BytesMut> for SecretBytesMut {
    fn peek(&self) -> &BytesMut {
        &self.0
    }

    fn peek_mut(&mut self) -> &mut BytesMut {
        &mut self.0
    }
}

impl fmt::Debug for SecretBytesMut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretBytesMut([REDACTED])")
    }
}

impl From<BytesMut> for SecretBytesMut {
    fn from(bytes: BytesMut) -> Self {
        Self::new(bytes)
    }
}

impl Drop for SecretBytesMut {
    fn drop(&mut self) {
        self.0.resize(self.0.capacity(), 0);
        self.0.as_mut().zeroize();
        debug_assert!(self.0.as_ref().iter().all(|b| *b == 0));
    }
}

#[cfg(all(feature = "bytes", feature = "serde"))]
impl<'de> Deserialize<'de> for SecretBytesMut {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SecretBytesVisitor;

        impl<'de> de::Visitor<'de> for SecretBytesVisitor {
            type Value = SecretBytesMut;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("byte array")
            }

            #[inline]
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let mut bytes = BytesMut::with_capacity(v.len());
                bytes.extend_from_slice(v);
                Ok(SecretBytesMut(bytes))
            }

            #[inline]
            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                // 4096 is cargo culted from upstream
                let len = core::cmp::min(seq.size_hint().unwrap_or(0), 4096);
                let mut bytes = BytesMut::with_capacity(len);

                use bytes::BufMut;

                while let Some(value) = seq.next_element()? {
                    bytes.put_u8(value);
                }

                Ok(SecretBytesMut(bytes))
            }
        }

        deserializer.deserialize_bytes(SecretBytesVisitor)
    }
}
