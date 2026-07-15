use cards::CardNumber;
use common_utils::types::MinorUnit;
use hyperswitch_masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{enums as api_enums, relay::RelayError};

// ─── Request ────────────────────────────────────────────────────────────────

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct UnreferencedRefundRequest {
    /// Amount in minor units
    #[schema(value_type = i64, example = 1000)]
    pub amount: MinorUnit,

    /// Currency
    #[schema(value_type = api_enums::Currency, example = "USD")]
    pub currency: api_enums::Currency,

    /// Identifier of the connector ( merchant connector account ) which was chosen to make the refund
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = String)]
    pub connector_id: common_utils::id_type::MerchantConnectorAccountId,

    /// Customer ID
    #[schema(example = "cus_123456789")]
    pub customer_id: Option<String>,

    /// The identifier that is associated to a resource at the connector reference
    #[schema(example = "7256228702616471803954")]
    pub connector_resource_id: Option<String>,

    /// Recipient payment method data
    pub recipient_payment_method_data: RecipientPaymentMethodData,
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipientPaymentMethodData {
    Card(RecipientCardData),
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct RecipientCardData {
    #[schema(value_type = String, example = "4000000000009995")]
    pub card_number: CardNumber,

    #[schema(value_type = String, example = "03")]
    pub card_exp_month: Secret<String>,

    #[schema(value_type = String, example = "30")]
    pub card_exp_year: Secret<String>,

    #[schema(value_type = Option<String>, example = "John Doe")]
    pub card_holder_name: Option<Secret<String>>,
}

impl RecipientCardData {
    pub fn get_expiry_year_4_digit(&self) -> Secret<String> {
        let year = self.card_exp_year.peek();
        let full_year = if year.len() == 2 {
            format!("20{year}")
        } else {
            year.to_string()
        };
        Secret::new(full_year)
    }
}

// ─── Response ────────────────────────────────────────────────────────────────

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct UnreferencedRefundResponse {
    /// Unique identifier for this unreferenced refund
    #[schema(example = "relay_V0BvOr4rk7AEIuq4nW5H", value_type = String)]
    pub id: common_utils::id_type::RelayId,

    /// The status of the relay request
    #[schema(value_type = api_enums::RelayStatus)]
    pub status: api_enums::RelayStatus,

    /// Connector name for which the relay was processed
    #[schema(example = "fiservcommercehub")]
    pub connector: String,

    /// Identifier of the connector ( merchant connector account ) which was chosen to make the refund
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR", value_type = String)]
    pub connector_id: common_utils::id_type::MerchantConnectorAccountId,

    /// The identifier that is associated to a resource at the connector reference
    #[schema(example = "7256228702616471803954")]
    pub connector_resource_id: String,

    /// The identifier that is associated to a resource at the connector to which the  request is being made
    #[schema(example = "re_3QY4TnEOqOywnAIx1Mm1p7GQ")]
    pub connector_reference_id: Option<String>,

    /// The business profile that is associated with this request.
    #[schema(example = "pro_xxx", value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,

    /// Error details if failed
    pub error: Option<RelayError>,

    /// Raw response from connector
    #[schema(value_type = Option<Object>)]
    pub raw_connector_response: Option<Secret<serde_json::Value>>,
}

impl TryFrom<crate::relay::RelayRequest> for UnreferencedRefundRequest {
    type Error = error_stack::Report<common_utils::errors::ValidationError>;

    fn try_from(request: crate::relay::RelayRequest) -> Result<Self, Self::Error> {
        let data = match request.data {
            Some(crate::relay::RelayData::UnreferencedRefund(data)) => data,
            _ => Err(error_stack::report!(
                common_utils::errors::ValidationError::InvalidValue {
                    message: "Relay data of type unreferenced_refund is required".to_string(),
                }
            ))?,
        };

        let recipient_payment_method_data =
            data.recipient_payment_method_data.ok_or_else(|| {
                error_stack::report!(
                    common_utils::errors::ValidationError::MissingRequiredField {
                        field_name: "recipient_payment_method_data".to_string(),
                    }
                )
            })?;

        Ok(Self {
            amount: data.amount,
            currency: data.currency,
            connector_id: request.connector_id,
            customer_id: data.customer_id,
            connector_resource_id: Some(request.connector_resource_id),
            recipient_payment_method_data,
        })
    }
}

impl common_utils::events::ApiEventMetric for UnreferencedRefundRequest {}
impl common_utils::events::ApiEventMetric for UnreferencedRefundResponse {}
