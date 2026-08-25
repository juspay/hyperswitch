use std::fmt::Debug;

use common_utils::events::ApiEventMetric;
use utoipa::ToSchema;

use crate::enums;

/// Co-badge networks come in two ways: a JSON array from the API, or a JSON string wrapping that
/// array from a CSV upload.
fn deserialize_co_badged_card_networks<'de, D>(
    deserializer: D,
) -> Result<Option<common_utils::types::CoBadgedCardNetworkMetadata>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct CoBadgedCardNetworkMetadataVisitor;

    impl<'de> serde::de::Visitor<'de> for CoBadgedCardNetworkMetadataVisitor {
        type Value = Option<common_utils::types::CoBadgedCardNetworkMetadata>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON array of co-badge entries")
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            // From a CSV cell the array arrives as a JSON string; parse it as a JSON array.
            let networks =
                serde_json::from_str::<Vec<common_utils::types::SecondaryNetwork>>(trimmed)
                    .map_err(|err| E::custom(format!("invalid co_badged_card_networks: {err}")))?;
            Ok(Some(common_utils::types::CoBadgedCardNetworkMetadata(
                networks,
            )))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut networks = Vec::new();
            while let Some(entry) = seq.next_element::<common_utils::types::SecondaryNetwork>()? {
                networks.push(entry);
            }
            Ok(Some(common_utils::types::CoBadgedCardNetworkMetadata(
                networks,
            )))
        }
    }

    deserializer.deserialize_any(CoBadgedCardNetworkMetadataVisitor)
}

/// Same idea as above but generic over `T` — used for `authentication` and `cost`. Takes a JSON
/// array, or a JSON string wrapping one (CSV upload); a blank string or null gives `None`.
fn deserialize_json_or_stringified_json<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = <Option<serde_json::Value> as serde::Deserialize>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(raw)) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(serde_json::from_str(trimmed).map_err(serde::de::Error::custom)?)
            }
        }
        Some(other) => Some(serde_json::from_value(other).map_err(serde::de::Error::custom)?),
    })
}

#[derive(serde::Deserialize, ToSchema)]
pub struct CardsInfoRequestParams {
    #[schema(example = "pay_OSERgeV9qAy7tlK7aKpc_secret_TuDUoh11Msxh12sXn3Yp")]
    pub client_secret: Option<String>,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct CardsInfoRequest {
    pub client_secret: Option<String>,
    pub card_iin: String,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct CardInfoResponse {
    #[schema(example = "374431")]
    pub card_iin: String,
    #[schema(example = "AMEX")]
    pub card_issuer: Option<String>,
    #[schema(example = "AMEX")]
    pub card_network: Option<String>,
    #[schema(example = "CREDIT")]
    pub card_type: Option<String>,
    #[schema(example = "CLASSIC")]
    pub card_sub_type: Option<String>,
    #[schema(example = "INDIA")]
    pub card_issuing_country: Option<String>,
    #[schema(example = "IN")]
    pub country_code: Option<String>,
    #[schema(example = "CREDIT")]
    pub funding_source: Option<String>,
    #[schema(example = "PAN")]
    pub card_iin_type: Option<String>,
    #[schema(example = false)]
    pub virtual_card: Option<bool>,
    #[schema(example = false)]
    pub gambling_blocked: Option<bool>,
    #[schema(value_type = Option<Vec<Object>>, example = json!([{"network": "VISA"}, {"network": "RUPAY"}]))]
    pub co_badged_card_networks: Option<common_utils::types::CoBadgedCardNetworkMetadata>,
    pub card_segment_type: Option<String>,
    pub numeric_country_code: Option<String>,
    pub prepaid: Option<bool>,
    pub regulated: Option<bool>,
    pub issuer_phone: Option<String>,
    pub issuer_url: Option<String>,
    pub regulated_name: Option<String>,
    pub reloadable_prepaid: Option<bool>,
    pub account_updater: Option<bool>,
    pub account_level_management: Option<bool>,
    pub domestic_only: Option<bool>,
    pub level_two_supported: Option<bool>,
    pub level_three_supported: Option<bool>,
    pub issuer_currency: Option<String>,
    pub combo_card: Option<String>,
    #[schema(value_type = Option<Vec<Object>>)]
    pub authentication: Option<common_utils::types::CardAuthentication>,
    #[schema(value_type = Option<Vec<Object>>)]
    pub cost: Option<common_utils::types::CardCost>,
    pub issuer_supports_tokenization: Option<bool>,
    pub billpay_enabled: Option<bool>,
    pub ecom_enabled: Option<bool>,
    pub flexible_credential_supported: Option<bool>,
    pub card_subtype_code: Option<String>,
    pub multi_account_access_indicator: Option<String>,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct CardInfoMigrateResponseRecord {
    pub card_iin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<String>,
    pub card_type: Option<String>,
    pub card_sub_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub country_code: Option<String>,
    pub funding_source: Option<String>,
    pub card_iin_type: Option<String>,
    pub virtual_card: Option<bool>,
    pub gambling_blocked: Option<bool>,
    #[schema(value_type = Option<Vec<Object>>)]
    pub co_badged_card_networks: Option<common_utils::types::CoBadgedCardNetworkMetadata>,
    pub card_segment_type: Option<String>,
    pub numeric_country_code: Option<String>,
    pub prepaid: Option<bool>,
    pub regulated: Option<bool>,
    pub issuer_phone: Option<String>,
    pub issuer_url: Option<String>,
    pub regulated_name: Option<String>,
    pub reloadable_prepaid: Option<bool>,
    pub account_updater: Option<bool>,
    pub account_level_management: Option<bool>,
    pub domestic_only: Option<bool>,
    pub level_two_supported: Option<bool>,
    pub level_three_supported: Option<bool>,
    pub issuer_currency: Option<String>,
    pub combo_card: Option<String>,
    #[schema(value_type = Option<Vec<Object>>)]
    pub authentication: Option<common_utils::types::CardAuthentication>,
    #[schema(value_type = Option<Vec<Object>>)]
    pub cost: Option<common_utils::types::CardCost>,
    pub issuer_supports_tokenization: Option<bool>,
    pub billpay_enabled: Option<bool>,
    pub ecom_enabled: Option<bool>,
    pub flexible_credential_supported: Option<bool>,
    pub card_subtype_code: Option<String>,
    pub multi_account_access_indicator: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardInfoCreateRequest {
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_subtype: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code_id: Option<String>,
    pub bank_code: Option<String>,
    pub country_code: Option<String>,
    pub last_updated_provider: Option<String>,
    pub funding_source: Option<enums::FundingSource>,
    pub card_iin_type: Option<enums::PanOrToken>,
    pub virtual_card: Option<bool>,
    pub gambling_blocked: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_co_badged_card_networks")]
    #[schema(value_type = Option<Vec<Object>>, example = json!([{"network": "VISA"}, {"network": "STAR"}]))]
    pub co_badged_card_networks: Option<common_utils::types::CoBadgedCardNetworkMetadata>,
    pub card_segment_type: Option<String>,
    pub numeric_country_code: Option<String>,
    pub prepaid: Option<bool>,
    pub regulated: Option<bool>,
    pub issuer_phone: Option<String>,
    pub issuer_url: Option<String>,
    pub regulated_name: Option<String>,
    pub reloadable_prepaid: Option<bool>,
    pub account_updater: Option<bool>,
    pub account_level_management: Option<bool>,
    pub domestic_only: Option<bool>,
    pub level_two_supported: Option<bool>,
    pub level_three_supported: Option<bool>,
    pub issuer_currency: Option<String>,
    pub combo_card: Option<String>,
    #[serde(default, deserialize_with = "deserialize_json_or_stringified_json")]
    #[schema(value_type = Option<Vec<Object>>)]
    pub authentication: Option<common_utils::types::CardAuthentication>,
    #[serde(default, deserialize_with = "deserialize_json_or_stringified_json")]
    #[schema(value_type = Option<Vec<Object>>)]
    pub cost: Option<common_utils::types::CardCost>,
    pub issuer_supports_tokenization: Option<bool>,
    pub billpay_enabled: Option<bool>,
    pub ecom_enabled: Option<bool>,
    pub flexible_credential_supported: Option<bool>,
    pub card_subtype_code: Option<String>,
    pub multi_account_access_indicator: Option<String>,
}

impl ApiEventMetric for CardInfoCreateRequest {}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardInfoUpdateRequest {
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_subtype: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code_id: Option<String>,
    pub bank_code: Option<String>,
    pub country_code: Option<String>,
    pub last_updated_provider: Option<String>,
    pub funding_source: Option<enums::FundingSource>,
    pub card_iin_type: Option<enums::PanOrToken>,
    pub virtual_card: Option<bool>,
    pub gambling_blocked: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_co_badged_card_networks")]
    #[schema(value_type = Option<Vec<Object>>, example = json!([{"network": "VISA"}, {"network": "STAR"}]))]
    pub co_badged_card_networks: Option<common_utils::types::CoBadgedCardNetworkMetadata>,
    pub card_segment_type: Option<String>,
    pub numeric_country_code: Option<String>,
    pub prepaid: Option<bool>,
    pub regulated: Option<bool>,
    pub issuer_phone: Option<String>,
    pub issuer_url: Option<String>,
    pub regulated_name: Option<String>,
    pub reloadable_prepaid: Option<bool>,
    pub account_updater: Option<bool>,
    pub account_level_management: Option<bool>,
    pub domestic_only: Option<bool>,
    pub level_two_supported: Option<bool>,
    pub level_three_supported: Option<bool>,
    pub issuer_currency: Option<String>,
    pub combo_card: Option<String>,
    #[serde(default, deserialize_with = "deserialize_json_or_stringified_json")]
    #[schema(value_type = Option<Vec<Object>>)]
    pub authentication: Option<common_utils::types::CardAuthentication>,
    #[serde(default, deserialize_with = "deserialize_json_or_stringified_json")]
    #[schema(value_type = Option<Vec<Object>>)]
    pub cost: Option<common_utils::types::CardCost>,
    pub issuer_supports_tokenization: Option<bool>,
    pub billpay_enabled: Option<bool>,
    pub ecom_enabled: Option<bool>,
    pub flexible_credential_supported: Option<bool>,
    pub card_subtype_code: Option<String>,
    pub multi_account_access_indicator: Option<String>,
    pub line_number: Option<i64>,
}

impl ApiEventMetric for CardInfoUpdateRequest {}

#[derive(Debug, Default, serde::Serialize, ToSchema)]
pub enum CardInfoMigrationStatus {
    Success,
    #[default]
    Failed,
}
#[derive(Debug, Default, serde::Serialize, ToSchema)]
pub struct CardInfoMigrationResponse {
    pub line_number: Option<i64>,
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<String>,
    pub card_type: Option<String>,
    pub card_sub_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub country_code: Option<String>,
    pub funding_source: Option<String>,
    pub card_iin_type: Option<String>,
    pub virtual_card: Option<bool>,
    pub gambling_blocked: Option<bool>,
    #[schema(value_type = Option<Vec<Object>>)]
    pub co_badged_card_networks: Option<common_utils::types::CoBadgedCardNetworkMetadata>,
    pub card_segment_type: Option<String>,
    pub numeric_country_code: Option<String>,
    pub prepaid: Option<bool>,
    pub regulated: Option<bool>,
    pub issuer_phone: Option<String>,
    pub issuer_url: Option<String>,
    pub regulated_name: Option<String>,
    pub reloadable_prepaid: Option<bool>,
    pub account_updater: Option<bool>,
    pub account_level_management: Option<bool>,
    pub domestic_only: Option<bool>,
    pub level_two_supported: Option<bool>,
    pub level_three_supported: Option<bool>,
    pub issuer_currency: Option<String>,
    pub combo_card: Option<String>,
    #[schema(value_type = Option<Vec<Object>>)]
    pub authentication: Option<common_utils::types::CardAuthentication>,
    #[schema(value_type = Option<Vec<Object>>)]
    pub cost: Option<common_utils::types::CardCost>,
    pub issuer_supports_tokenization: Option<bool>,
    pub billpay_enabled: Option<bool>,
    pub ecom_enabled: Option<bool>,
    pub flexible_credential_supported: Option<bool>,
    pub card_subtype_code: Option<String>,
    pub multi_account_access_indicator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_error: Option<String>,
    pub migration_status: CardInfoMigrationStatus,
}
impl ApiEventMetric for CardInfoMigrationResponse {}

type CardInfoMigrationResponseType = (
    Result<CardInfoMigrateResponseRecord, String>,
    CardInfoUpdateRequest,
);

impl From<CardInfoMigrationResponseType> for CardInfoMigrationResponse {
    fn from((response, record): CardInfoMigrationResponseType) -> Self {
        match response {
            Ok(res) => Self {
                card_iin: record.card_iin,
                line_number: record.line_number,
                card_issuer: res.card_issuer,
                card_network: res.card_network,
                card_type: res.card_type,
                card_sub_type: res.card_sub_type,
                card_issuing_country: res.card_issuing_country,
                country_code: res.country_code,
                funding_source: res.funding_source,
                card_iin_type: res.card_iin_type,
                virtual_card: res.virtual_card,
                gambling_blocked: res.gambling_blocked,
                co_badged_card_networks: res.co_badged_card_networks,
                card_segment_type: res.card_segment_type,
                numeric_country_code: res.numeric_country_code,
                prepaid: res.prepaid,
                regulated: res.regulated,
                issuer_phone: res.issuer_phone,
                issuer_url: res.issuer_url,
                regulated_name: res.regulated_name,
                reloadable_prepaid: res.reloadable_prepaid,
                account_updater: res.account_updater,
                account_level_management: res.account_level_management,
                domestic_only: res.domestic_only,
                level_two_supported: res.level_two_supported,
                level_three_supported: res.level_three_supported,
                issuer_currency: res.issuer_currency,
                combo_card: res.combo_card,
                authentication: res.authentication,
                cost: res.cost,
                issuer_supports_tokenization: res.issuer_supports_tokenization,
                billpay_enabled: res.billpay_enabled,
                ecom_enabled: res.ecom_enabled,
                flexible_credential_supported: res.flexible_credential_supported,
                card_subtype_code: res.card_subtype_code,
                multi_account_access_indicator: res.multi_account_access_indicator,
                migration_status: CardInfoMigrationStatus::Success,
                migration_error: None,
            },
            Err(e) => Self {
                card_iin: record.card_iin,
                migration_status: CardInfoMigrationStatus::Failed,
                migration_error: Some(e),
                line_number: record.line_number,
                ..Self::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn co_badged_card_networks_deserializes_native_json_array() {
        let json = serde_json::json!({
            "card_iin": "414720",
            "co_badged_card_networks": [
                {"card_network": "STAR", "card_iin_type": "pan"},
                {"card_network": "PULSE"},
            ],
        });
        let request: CardInfoCreateRequest = serde_json::from_value(json).unwrap();
        let networks = request
            .co_badged_card_networks
            .expect("expected co_badged_card_networks to be present");
        assert_eq!(networks.0.len(), 2);
        let first = networks.0.first().expect("first network");
        assert_eq!(first.card_network, common_enums::CoBadgedCardNetwork::Star);
        assert_eq!(first.card_iin_type.as_deref(), Some("pan"));
        let second = networks.0.get(1).expect("second network");
        assert_eq!(
            second.card_network,
            common_enums::CoBadgedCardNetwork::Pulse
        );
        assert_eq!(second.card_iin_type, None);
    }

    #[test]
    fn co_badged_card_networks_deserializes_json_array_string_from_csv() {
        // A bulk-migration CSV cell delivers the array as a JSON string.
        let json = serde_json::json!({
            "card_iin": "414720",
            "co_badged_card_networks": "[{\"card_network\": \"ELO\", \"ecom_enabled\": true}]",
        });
        let request: CardInfoUpdateRequest = serde_json::from_value(json).unwrap();
        let networks = request
            .co_badged_card_networks
            .expect("expected co_badged_card_networks to be present");
        assert_eq!(networks.0.len(), 1);
        let first = networks.0.first().expect("first network");
        assert_eq!(first.card_network, common_enums::CoBadgedCardNetwork::Elo);
        assert_eq!(first.ecom_enabled, Some(true));
    }

    #[test]
    fn co_badged_card_networks_absent_is_none() {
        let json = serde_json::json!({ "card_iin": "414720" });
        let request: CardInfoCreateRequest = serde_json::from_value(json).unwrap();
        assert!(request.co_badged_card_networks.is_none());
    }

    #[test]
    fn co_badged_card_networks_empty_csv_cell_is_none() {
        let json = serde_json::json!({
            "card_iin": "414720",
            "co_badged_card_networks": "",
        });
        let request: CardInfoUpdateRequest = serde_json::from_value(json).unwrap();
        assert!(request.co_badged_card_networks.is_none());
    }

    #[test]
    fn combo_card_deserializes_as_text() {
        let json = serde_json::json!({
            "card_iin": "414720",
            "combo_card": "Credit and Debit",
        });
        let request: CardInfoCreateRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.combo_card.as_deref(), Some("Credit and Debit"));
    }

    #[test]
    fn authentication_and_cost_deserialize_native_json_arrays() {
        let json = serde_json::json!({
            "card_iin": "414720",
            "authentication": [{"authentication_name": "EU PSD2 - SCA"}],
            "cost": [{"cap_type_name": "US DURBIN REGULATION DEBIT VISA", "cap_fixed_amount": "0.21"}],
        });
        let request: CardInfoCreateRequest = serde_json::from_value(json).unwrap();

        let authentication = request.authentication.expect("authentication present");
        let auth_entry = authentication.0.first().expect("one authentication entry");
        assert_eq!(
            auth_entry.authentication_name.as_deref(),
            Some("EU PSD2 - SCA")
        );

        let cost = request.cost.expect("cost present");
        let cost_entry = cost.0.first().expect("one cost entry");
        assert_eq!(
            cost_entry.cap_type_name.as_deref(),
            Some("US DURBIN REGULATION DEBIT VISA")
        );
        assert_eq!(cost_entry.cap_fixed_amount.as_deref(), Some("0.21"));
    }

    #[test]
    fn authentication_and_cost_deserialize_json_string_from_csv() {
        // Bulk-migration CSV cells deliver these arrays as JSON strings.
        let json = serde_json::json!({
            "card_iin": "414720",
            "authentication": "[{\"authentication_name\": \"EU PSD2 - SCA\"}]",
            "cost": "[{\"cap_type_name\": \"US DURBIN REGULATION DEBIT VISA\", \"cap_fixed_amount\": \"0.21\"}]",
        });
        let request: CardInfoUpdateRequest = serde_json::from_value(json).unwrap();

        let auth = request.authentication.expect("authentication present");
        assert_eq!(
            auth.0
                .first()
                .expect("entry")
                .authentication_name
                .as_deref(),
            Some("EU PSD2 - SCA")
        );
        let cost = request.cost.expect("cost present");
        assert_eq!(
            cost.0.first().expect("entry").cap_fixed_amount.as_deref(),
            Some("0.21")
        );
    }

    #[test]
    fn authentication_and_cost_empty_csv_cell_is_none() {
        let json = serde_json::json!({
            "card_iin": "414720",
            "authentication": "",
            "cost": "",
        });
        let request: CardInfoUpdateRequest = serde_json::from_value(json).unwrap();
        assert!(request.authentication.is_none());
        assert!(request.cost.is_none());
    }

    #[test]
    fn authentication_and_cost_accept_native_json_arrays() {
        let json = serde_json::json!({
            "card_iin": "414720",
            "authentication": [{"authentication_name": "JCA - EMV 3DS MANDATE"}],
            "cost": [],
        });
        let request: CardInfoUpdateRequest = serde_json::from_value(json).unwrap();
        assert!(request.authentication.is_some());
        assert_eq!(request.cost, Some(common_utils::types::CardCost(vec![])));
    }

    #[test]
    fn authentication_and_cost_absent_is_none() {
        let json = serde_json::json!({ "card_iin": "414720" });
        let request: CardInfoCreateRequest = serde_json::from_value(json).unwrap();
        assert!(request.authentication.is_none());
        assert!(request.cost.is_none());
    }
}
