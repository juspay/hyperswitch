use std::collections::HashMap;

use api_models::payments::{
    MandateAmountData as ApiMandateAmountData, MandateData as ApiMandateData, MandateType,
};
use common_enums::Currency;
use common_types::payments as common_payments_types;
use common_utils::{
    date_time,
    errors::{CustomResult, ParsingError},
    pii,
    types::MinorUnit,
};
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::router_data::RecurringMandatePaymentData;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct MandateDetails {
    pub update_mandate_id: Option<String>,
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

#[derive(Default, Eq, PartialEq, Debug, Clone, serde::Serialize)]
pub struct MandateData {
    pub update_mandate_id: Option<String>,
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    pub mandate_type: Option<MandateDataType>,
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

impl From<ApiMandateData> for MandateData {
    fn from(value: ApiMandateData) -> Self {
        Self {
            customer_acceptance: value.customer_acceptance,
            mandate_type: value.mandate_type.map(|d| d.into()),
            update_mandate_id: value.update_mandate_id,
        }
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

#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReferenceRecord {
    pub connector_mandate_id: String,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<Currency>,
    pub mandate_metadata: Option<pii::SecretSerdeValue>,
    pub connector_mandate_status: Option<common_enums::ConnectorMandateStatus>,
    pub connector_mandate_request_reference_id: Option<String>,
    pub connector_customer_id: Option<String>,
}

#[cfg(feature = "v1")]
impl From<&PaymentsMandateReferenceRecord> for RecurringMandatePaymentData {
    fn from(mandate_reference_record: &PaymentsMandateReferenceRecord) -> Self {
        Self {
            payment_method_type: mandate_reference_record.payment_method_type,
            original_payment_authorized_amount: mandate_reference_record
                .original_payment_authorized_amount,
            original_payment_authorized_currency: mandate_reference_record
                .original_payment_authorized_currency,
            mandate_metadata: mandate_reference_record.mandate_metadata.clone(),
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorTokenReferenceRecord {
    pub connector_token: String,
    pub payment_method_subtype: Option<common_enums::PaymentMethodType>,
    pub original_payment_authorized_amount: Option<MinorUnit>,
    pub original_payment_authorized_currency: Option<Currency>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_token_status: common_enums::ConnectorTokenStatus,
    pub connector_token_request_reference_id: Option<String>,
    pub connector_customer_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PayoutsMandateReferenceRecord {
    pub transfer_method_id: Option<String>,
    pub connector_customer_id: Option<String>,
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

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsTokenReference(
    pub HashMap<common_utils::id_type::MerchantConnectorAccountId, ConnectorTokenReferenceRecord>,
);

#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReference(
    pub HashMap<common_utils::id_type::MerchantConnectorAccountId, PaymentsMandateReferenceRecord>,
);

#[cfg(feature = "v1")]
impl PaymentsMandateReference {
    pub fn is_active_connector_mandate_available(&self) -> bool {
        self.clone().0.into_iter().any(|detail| {
            detail
                .1
                .connector_mandate_status
                .map(|connector_mandate_status| connector_mandate_status.is_active())
                .unwrap_or(false)
        })
    }
}

#[cfg(feature = "v1")]
impl std::ops::Deref for PaymentsMandateReference {
    type Target =
        HashMap<common_utils::id_type::MerchantConnectorAccountId, PaymentsMandateReferenceRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "v1")]
impl std::ops::DerefMut for PaymentsMandateReference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "v2")]
impl std::ops::Deref for PaymentsTokenReference {
    type Target =
        HashMap<common_utils::id_type::MerchantConnectorAccountId, ConnectorTokenReferenceRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "v2")]
impl std::ops::DerefMut for PaymentsTokenReference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommonMandateReference {
    pub payments: Option<PaymentsMandateReference>,
    pub payouts: Option<PayoutsMandateReference>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommonMandateReference {
    pub payments: Option<PaymentsTokenReference>,
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

    #[cfg(feature = "v2")]
    pub fn insert_payment_token_reference_record(
        &mut self,
        connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        record: ConnectorTokenReferenceRecord,
    ) {
        match self.payments {
            Some(ref mut payments_reference) => {
                payments_reference.insert(connector_id.clone(), record);
            }
            None => {
                let mut payments_reference = HashMap::new();
                payments_reference.insert(connector_id.clone(), record);
                self.payments = Some(PaymentsTokenReference(payments_reference));
            }
        }
    }
}
