use euclid::frontend::dir::DirKeyKind;
#[cfg(feature = "payouts")]
use euclid::frontend::dir::PayoutDirKeyKind;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Details<'a> {
    pub description: Option<&'a str>,
    pub kind: DirKeyKind,
}

#[cfg(feature = "payouts")]
#[derive(Serialize, Clone)]
pub struct PayoutDetails<'a> {
    pub description: Option<&'a str>,
    pub kind: PayoutDirKeyKind,
}
