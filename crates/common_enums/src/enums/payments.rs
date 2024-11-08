use serde;
use utoipa::ToSchema;

#[derive(Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProductType {
    #[default]
    Physical,
    Digital,
    Travel,
    Ride,
    Event,
    Accommodation,
}
