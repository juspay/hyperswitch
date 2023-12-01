use euclid::frontend::dir::DirKeyKind;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Details<'a> {
    pub description: Option<&'a str>,
    pub kind: DirKeyKind,
}
