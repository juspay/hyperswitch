//! Notification: receiving alert data and delivering it to a destination channel.
//!
//! The first concern to live in this crate, and deliberately *a* concern rather than *the*
//! concern — `alerts` is the alerting plane, and notification is one part of it.
//!
//! Empty by design. The `Notifier` trait, the envelope every channel renders, and the webhook
//! that receives them are defined in "Define the Notifier interface and the webhook API"
//! (hyperswitch-cloud#23116). This module exists now so that ticket fills a seam that is already
//! there rather than relitigating where it goes.
//!
//! One constraint is already settled and must survive whatever lands here: **a `Notifier` is a
//! pure sink**. It is told what to say; it does not decide. The `hyperswitch-alerts` R service
//! learned this the hard way — when deciding and delivering lived in the same step, the first
//! sink consumed the shared lifecycle store and every later sink went permanently silent. Any
//! design where an implementation mutates shared delivery state reintroduces that bug.
