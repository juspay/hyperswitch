#![allow(clippy::indexing_slicing)]

use super::i18n;
// allowing clippy indexing_slicing due to this
// https://github.com/longbridgeapp/rust-i18n/blob/v3.0.1/crates/macro/src/lib.rs#L225
i18n!("locales", fallback = "en");
