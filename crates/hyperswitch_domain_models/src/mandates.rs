use std::collections::HashMap;

use api_models::payments::{
    AcceptanceType as ApiAcceptanceType, CustomerAcceptance as ApiCustomerAcceptance,
    MandateAmountData as ApiMandateAmountData, MandateData as ApiMandateData, MandateType,
    OnlineMandate as ApiOnlineMandate,
};
use common_enums::Currency;
use common_utils::{
    date_time,
    errors::{CustomResult, ParsingError},
    pii,
    types::MinorUnit,
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct MandateDetails {
    pub update_mandate_id: Option<String>,
}

impl From<MandateDetails> for diesel_models::enums::MandateDetails {
    fn from(value: MandateDetails) -> Self {
        Self {
            update_mandate_id: value.update_mandate_id,
        }
    }
}

impl From<diesel_models::enums::MandateDetails> for MandateDetails {
    fn from(value: diesel_models::enums::MandateDetails) -> Self {
        Self {
            update_mandate_id: value.update_mandate_id,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MandateDataType {
    SingleUse(MandateAmountData),
    MultiUse(Option<MandateAmountData>),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct MandateAmountData {
    pub amount: MinorUnit,
    pub currency: Currency,
    pub start_date: Option<PrimitiveDateTime>,
    pub end_date: Option<PrimitiveDateTime>,
    pub metadata: Option<pii::SecretSerdeValue>,
}

// The fields on this struct are optional, as we want to allow the merchant to provide partial
// information about creating mandates
#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct MandateData {
    /// A way to update the mandate's payment method details
    pub update_mandate_id: Option<String>,
    /// A consent from the customer to store the payment method
    pub customer_acceptance: Option<CustomerAcceptance>,
    /// A way to select the type of mandate used
    pub mandate_type: Option<MandateDataType>,
}

#[derive(Default, Eq, PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CustomerAcceptance {
    /// Type of acceptance provided by the
    pub acceptance_type: AcceptanceType,
    /// Specifying when the customer acceptance was provided
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub accepted_at: Option<PrimitiveDateTime>,
    /// Information required for online mandate generation
    pub online: Option<OnlineMandate>,
}

#[derive(Default, Debug, PartialEq, Eq, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AcceptanceType {
    Online,
    #[default]
    Offline,
}

#[derive(Default, Eq, PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OnlineMandate {
    /// Ip address of the customer machine from which the mandate was created
    #[serde(skip_deserializing)]
    pub ip_address: Option<Secret<String, pii::IpAddress>>,
    /// The user-agent of the customer's browser
    pub user_agent: String,
}

impl From<MandateType> for MandateDataType {
    fn from(mandate_type: MandateType) -> Self {
        match mandate_type {
            MandateType::SingleUse(mandate_amount_data) => {
                Self::SingleUse(mandate_amount_data.into())
            }
            MandateType::MultiUse(mandate_amount_data) => {
                Self::MultiUse(mandate_amount_data.map(|d| d.into()))
            }
        }
    }
}

impl From<MandateDataType> for diesel_models::enums::MandateDataType {
    fn from(value: MandateDataType) -> Self {
        match value {
            MandateDataType::SingleUse(data) => Self::SingleUse(data.into()),
            MandateDataType::MultiUse(None) => Self::MultiUse(None),
            MandateDataType::MultiUse(Some(data)) => Self::MultiUse(Some(data.into())),
        }
    }
}

impl From<diesel_models::enums::MandateDataType> for MandateDataType {
    fn from(value: diesel_models::enums::MandateDataType) -> Self {
        use diesel_models::enums::MandateDataType as DieselMandateDataType;

        match value {
            DieselMandateDataType::SingleUse(data) => Self::SingleUse(data.into()),
            DieselMandateDataType::MultiUse(None) => Self::MultiUse(None),
            DieselMandateDataType::MultiUse(Some(data)) => Self::MultiUse(Some(data.into())),
        }
    }
}

impl From<ApiMandateAmountData> for MandateAmountData {
    fn from(value: ApiMandateAmountData) -> Self {
        Self {
            amount: value.amount,
            currency: value.currency,
            start_date: value.start_date,
            end_date: value.end_date,
            metadata: value.metadata,
        }
    }
}

impl From<MandateAmountData> for diesel_models::enums::MandateAmountData {
    fn from(value: MandateAmountData) -> Self {
        Self {
            amount: value.amount,
            currency: value.currency,
            start_date: value.start_date,
            end_date: value.end_date,
            metadata: value.metadata,
        }
    }
}

impl From<diesel_models::enums::MandateAmountData> for MandateAmountData {
    fn from(value: diesel_models::enums::MandateAmountData) -> Self {
        Self {
            amount: value.amount,
            currency: value.currency,
            start_date: value.start_date,
            end_date: value.end_date,
            metadata: value.metadata,
        }
    }
}

impl From<ApiMandateData> for MandateData {
    fn from(value: ApiMandateData) -> Self {
        Self {
            customer_acceptance: value.customer_acceptance.map(|d| d.into()),
            mandate_type: value.mandate_type.map(|d| d.into()),
            update_mandate_id: value.update_mandate_id,
        }
    }
}

impl From<ApiCustomerAcceptance> for CustomerAcceptance {
    fn from(value: ApiCustomerAcceptance) -> Self {
        Self {
            acceptance_type: value.acceptance_type.into(),
            accepted_at: value.accepted_at,
            online: value.online.map(|d| d.into()),
        }
    }
}

impl From<CustomerAcceptance> for ApiCustomerAcceptance {
    fn from(value: CustomerAcceptance) -> Self {
        Self {
            acceptance_type: value.acceptance_type.into(),
            accepted_at: value.accepted_at,
            online: value.online.map(|d| d.into()),
        }
    }
}

impl From<ApiAcceptanceType> for AcceptanceType {
    fn from(value: ApiAcceptanceType) -> Self {
        match value {
            ApiAcceptanceType::Online => Self::Online,
            ApiAcceptanceType::Offline => Self::Offline,
        }
    }
}
impl From<AcceptanceType> for ApiAcceptanceType {
    fn from(value: AcceptanceType) -> Self {
        match value {
            AcceptanceType::Online => Self::Online,
            AcceptanceType::Offline => Self::Offline,
        }
    }
}

impl From<ApiOnlineMandate> for OnlineMandate {
    fn from(value: ApiOnlineMandate) -> Self {
        Self {
            ip_address: value.ip_address,
            user_agent: value.user_agent,
        }
    }
}
impl From<OnlineMandate> for ApiOnlineMandate {
    fn from(value: OnlineMandate) -> Self {
        Self {
            ip_address: value.ip_address,
            user_agent: value.user_agent,
        }
    }
}

impl CustomerAcceptance {
    pub fn get_ip_address(&self) -> Option<String> {
        self.online
            .as_ref()
            .and_then(|data| data.ip_address.as_ref().map(|ip| ip.peek().to_owned()))
    }

    pub fn get_user_agent(&self) -> Option<String> {
        self.online.as_ref().map(|data| data.user_agent.clone())
    }

    pub fn get_accepted_at(&self) -> PrimitiveDateTime {
        self.accepted_at.unwrap_or_else(date_time::now)
    }
}

impl MandateAmountData {
    pub fn get_end_date(
        &self,
        format: date_time::DateFormat,
    ) -> error_stack::Result<Option<String>, ParsingError> {
        self.end_date
            .map(|date| {
                date_time::format_date(date, format)
                    .change_context(ParsingError::DateTimeParsingError)
            })
            .transpose()
    }
    pub fn get_metadata(&self) -> Option<pii::SecretSerdeValue> {
        self.metadata.clone()
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReferenceRecord {
    pub connector_mandate_id: String,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<Currency>,
    pub mandate_metadata: Option<pii::SecretSerdeValue>,
    pub connector_mandate_status: Option<common_enums::ConnectorMandateStatus>,
    pub connector_mandate_request_reference_id: Option<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReferenceRecord {
    pub connector_mandate_id: String,
    pub payment_method_subtype: Option<common_enums::PaymentMethodType>,
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<Currency>,
    pub mandate_metadata: Option<pii::SecretSerdeValue>,
    pub connector_mandate_status: Option<common_enums::ConnectorMandateStatus>,
    pub connector_mandate_request_reference_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PayoutsMandateReferenceRecord {
    pub transfer_method_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PayoutsMandateReference(
    pub HashMap<common_utils::id_type::MerchantConnectorAccountId, PayoutsMandateReferenceRecord>,
);

impl std::ops::Deref for PayoutsMandateReference {
    type Target =
        HashMap<common_utils::id_type::MerchantConnectorAccountId, PayoutsMandateReferenceRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PayoutsMandateReference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReference(
    pub HashMap<common_utils::id_type::MerchantConnectorAccountId, PaymentsMandateReferenceRecord>,
);

impl std::ops::Deref for PaymentsMandateReference {
    type Target =
        HashMap<common_utils::id_type::MerchantConnectorAccountId, PaymentsMandateReferenceRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PaymentsMandateReference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommonMandateReference {
    pub payments: Option<PaymentsMandateReference>,
    pub payouts: Option<PayoutsMandateReference>,
}

impl CommonMandateReference {
    pub fn get_mandate_details_value(&self) -> CustomResult<serde_json::Value, ParsingError> {
        let mut payments = self
            .payments
            .as_ref()
            .map_or_else(|| Ok(serde_json::json!({})), serde_json::to_value)
            .change_context(ParsingError::StructParseFailure("payment mandate details"))?;

        self.payouts
            .as_ref()
            .map(|payouts_mandate| {
                serde_json::to_value(payouts_mandate).map(|payouts_mandate_value| {
                    payments.as_object_mut().map(|payments_object| {
                        payments_object.insert("payouts".to_string(), payouts_mandate_value);
                    })
                })
            })
            .transpose()
            .change_context(ParsingError::StructParseFailure("payout mandate details"))?;

        Ok(payments)
    }
}

impl From<diesel_models::CommonMandateReference> for CommonMandateReference {
    fn from(value: diesel_models::CommonMandateReference) -> Self {
        Self {
            payments: value.payments.map(|payments| payments.into()),
            payouts: value.payouts.map(|payouts| payouts.into()),
        }
    }
}

impl From<CommonMandateReference> for diesel_models::CommonMandateReference {
    fn from(value: CommonMandateReference) -> Self {
        Self {
            payments: value.payments.map(|payments| payments.into()),
            payouts: value.payouts.map(|payouts| payouts.into()),
        }
    }
}

impl From<diesel_models::PayoutsMandateReference> for PayoutsMandateReference {
    fn from(value: diesel_models::PayoutsMandateReference) -> Self {
        Self(
            value
                .0
                .into_iter()
                .map(|(key, record)| (key, record.into()))
                .collect(),
        )
    }
}

impl From<PayoutsMandateReference> for diesel_models::PayoutsMandateReference {
    fn from(value: PayoutsMandateReference) -> Self {
        Self(
            value
                .0
                .into_iter()
                .map(|(key, record)| (key, record.into()))
                .collect(),
        )
    }
}

impl From<diesel_models::PaymentsMandateReference> for PaymentsMandateReference {
    fn from(value: diesel_models::PaymentsMandateReference) -> Self {
        Self(
            value
                .0
                .into_iter()
                .map(|(key, record)| (key, record.into()))
                .collect(),
        )
    }
}

impl From<PaymentsMandateReference> for diesel_models::PaymentsMandateReference {
    fn from(value: PaymentsMandateReference) -> Self {
        Self(
            value
                .0
                .into_iter()
                .map(|(key, record)| (key, record.into()))
                .collect(),
        )
    }
}

impl From<diesel_models::PayoutsMandateReferenceRecord> for PayoutsMandateReferenceRecord {
    fn from(value: diesel_models::PayoutsMandateReferenceRecord) -> Self {
        Self {
            transfer_method_id: value.transfer_method_id,
        }
    }
}

impl From<PayoutsMandateReferenceRecord> for diesel_models::PayoutsMandateReferenceRecord {
    fn from(value: PayoutsMandateReferenceRecord) -> Self {
        Self {
            transfer_method_id: value.transfer_method_id,
        }
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
impl From<diesel_models::PaymentsMandateReferenceRecord> for PaymentsMandateReferenceRecord {
    fn from(value: diesel_models::PaymentsMandateReferenceRecord) -> Self {
        Self {
            connector_mandate_id: value.connector_mandate_id,
            payment_method_type: value.payment_method_type,
            original_payment_authorized_amount: value.original_payment_authorized_amount,
            original_payment_authorized_currency: value.original_payment_authorized_currency,
            mandate_metadata: value.mandate_metadata,
            connector_mandate_status: value.connector_mandate_status,
            connector_mandate_request_reference_id: value.connector_mandate_request_reference_id,
        }
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
impl From<PaymentsMandateReferenceRecord> for diesel_models::PaymentsMandateReferenceRecord {
    fn from(value: PaymentsMandateReferenceRecord) -> Self {
        Self {
            connector_mandate_id: value.connector_mandate_id,
            payment_method_type: value.payment_method_type,
            original_payment_authorized_amount: value.original_payment_authorized_amount,
            original_payment_authorized_currency: value.original_payment_authorized_currency,
            mandate_metadata: value.mandate_metadata,
            connector_mandate_status: value.connector_mandate_status,
            connector_mandate_request_reference_id: value.connector_mandate_request_reference_id,
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<diesel_models::PaymentsMandateReferenceRecord> for PaymentsMandateReferenceRecord {
    fn from(value: diesel_models::PaymentsMandateReferenceRecord) -> Self {
        Self {
            connector_mandate_id: value.connector_mandate_id,
            payment_method_subtype: value.payment_method_subtype,
            original_payment_authorized_amount: value.original_payment_authorized_amount,
            original_payment_authorized_currency: value.original_payment_authorized_currency,
            mandate_metadata: value.mandate_metadata,
            connector_mandate_status: value.connector_mandate_status,
            connector_mandate_request_reference_id: value.connector_mandate_request_reference_id,
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<PaymentsMandateReferenceRecord> for diesel_models::PaymentsMandateReferenceRecord {
    fn from(value: PaymentsMandateReferenceRecord) -> Self {
        Self {
            connector_mandate_id: value.connector_mandate_id,
            payment_method_subtype: value.payment_method_subtype,
            original_payment_authorized_amount: value.original_payment_authorized_amount,
            original_payment_authorized_currency: value.original_payment_authorized_currency,
            mandate_metadata: value.mandate_metadata,
            connector_mandate_status: value.connector_mandate_status,
            connector_mandate_request_reference_id: value.connector_mandate_request_reference_id,
        }
    }
}
