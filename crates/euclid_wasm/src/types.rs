use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Details<'a> {
    pub description: Option<&'a str>,
    pub category: Option<&'a str>,
}
