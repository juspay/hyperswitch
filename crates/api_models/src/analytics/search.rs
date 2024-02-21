#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct SearchFilters {
    pub payment_method: Vec<String>,
}
