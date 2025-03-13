use common_enums::enums;
use common_utils::{
    self,
    errors::{CustomResult, ValidationError},
    id_type::{self, GenerateId},
    types::keymanager,
};
use masking::Secret;
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoBadgedCardInfo {
    pub id: id_type::CoBadgedCardsInfoID,
    pub card_bin_min: i64,
    pub card_bin_max: i64,
    pub issuing_bank_name: String,
    pub card_network: enums::CardNetwork,
    pub country: enums::CountryAlpha2,
    pub card_type: enums::CardType,
    pub regulated: bool,
    pub regulated_name: Option<String>,
    pub prepaid: bool,
    pub reloadable: bool,
    pub pan_or_token: enums::PanOrToken,
    pub card_bin_length: i16,
    pub card_brand_is_additional: bool,
    pub domestic_only: bool,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub last_updated_provider: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, router_derive::DebugAsDisplay, serde::Deserialize)]
pub struct UpdateCoBadgedCardInfo {
    pub card_bin_min: Option<i64>,
    pub card_bin_max: Option<i64>,
    pub card_network: Option<enums::CardNetwork>,
    pub country: Option<enums::CountryAlpha2>,
    pub regulated: Option<bool>,
    pub regulated_name: Option<String>,
    pub prepaid: Option<bool>,
    pub reloadable: Option<bool>,
    pub pan_or_token: Option<enums::PanOrToken>,
    pub card_bin_length: Option<i16>,
    pub card_brand_is_additional: bool,
    pub domestic_only: Option<bool>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub last_updated_provider: Option<String>,
}

impl CoBadgedCardInfo {
    pub fn new(
        card_bin_min: i64,
        card_bin_max: i64,
        issuing_bank_name: String,
        card_network: enums::CardNetwork,
        country: enums::CountryAlpha2,
        card_type: enums::CardType,
        regulated: bool,
        regulated_name: Option<String>,
        prepaid: bool,
        reloadable: bool,
        pan_or_token: enums::PanOrToken,
        card_bin_length: i16,
        card_brand_is_additional: bool,
        domestic_only: bool,
        last_updated_provider: Option<String>,
    ) -> Self {
        Self {
            id: id_type::CoBadgedCardsInfoID::generate(),
            card_bin_min,
            card_bin_max,
            issuing_bank_name,
            card_network,
            country,
            card_type,
            regulated,
            regulated_name,
            prepaid,
            reloadable,
            pan_or_token,
            card_bin_length,
            card_brand_is_additional,
            domestic_only,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            last_updated_provider,
        }
    }
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for CoBadgedCardInfo {
    type DstType = diesel_models::CoBadgedCardInfo;
    type NewDstType = diesel_models::CoBadgedCardInfo;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::CoBadgedCardInfo {
            id: self.id,
            card_bin_min: self.card_bin_min,
            card_bin_max: self.card_bin_max,
            issuing_bank_name: self.issuing_bank_name,
            card_network: self.card_network,
            country: self.country,
            card_type: self.card_type,
            regulated: self.regulated,
            regulated_name: self.regulated_name,
            prepaid: self.prepaid,
            reloadable: self.reloadable,
            pan_or_token: self.pan_or_token,
            card_bin_length: self.card_bin_length,
            card_brand_is_additional: self.card_brand_is_additional,
            domestic_only: self.domestic_only,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_updated_provider: self.last_updated_provider,
        })
    }

    async fn convert_back(
        _state: &keymanager::KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError> {
        Ok(Self {
            id: item.id,
            card_bin_min: item.card_bin_min,
            card_bin_max: item.card_bin_max,
            issuing_bank_name: item.issuing_bank_name,
            card_network: item.card_network,
            country: item.country,
            card_type: item.card_type,
            regulated: item.regulated,
            regulated_name: item.regulated_name,
            prepaid: item.prepaid,
            reloadable: item.reloadable,
            pan_or_token: item.pan_or_token,
            card_bin_length: item.card_bin_length,
            card_brand_is_additional: item.card_brand_is_additional,
            domestic_only: item.domestic_only,
            created_at: item.created_at,
            modified_at: item.modified_at,
            last_updated_provider: item.last_updated_provider,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::CoBadgedCardInfo {
            id: self.id,
            card_bin_min: self.card_bin_min,
            card_bin_max: self.card_bin_max,
            issuing_bank_name: self.issuing_bank_name,
            card_network: self.card_network,
            country: self.country,
            card_type: self.card_type,
            regulated: self.regulated,
            regulated_name: self.regulated_name,
            prepaid: self.prepaid,
            reloadable: self.reloadable,
            pan_or_token: self.pan_or_token,
            card_bin_length: self.card_bin_length,
            card_brand_is_additional: self.card_brand_is_additional,
            domestic_only: self.domestic_only,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_updated_provider: self.last_updated_provider,
        })
    }
}

impl From<UpdateCoBadgedCardInfo> for diesel_models::UpdateCoBadgedCardInfo {
    fn from(value: UpdateCoBadgedCardInfo) -> Self {
        Self {
            card_bin_max: value.card_bin_max,
            card_bin_min: value.card_bin_min,
            card_network: value.card_network,
            country: value.country,
            regulated: value.regulated,
            regulated_name: value.regulated_name,
            prepaid: value.prepaid,
            reloadable: value.reloadable,
            pan_or_token: value.pan_or_token,
            card_bin_length: value.card_bin_length,
            card_brand_is_additional: value.card_brand_is_additional,
            domestic_only: value.domestic_only,
            modified_at: common_utils::date_time::now(),
            last_updated_provider: value.last_updated_provider,
        }
    }
}
