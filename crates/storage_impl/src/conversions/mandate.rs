//! Conversion implementations for Mandate types

use crate::transformers::ForeignFrom;
use hyperswitch_domain_models::mandates::{
    CommonMandateReference, MandateAmountData, MandateDataType, MandateDetails,
    PaymentsMandateReference, PaymentsMandateReferenceRecord, PayoutsMandateReference,
    PayoutsMandateReferenceRecord,
};

#[cfg(feature = "v2")]
use hyperswitch_domain_models::mandates::{ConnectorTokenReferenceRecord, PaymentsTokenReference};

impl ForeignFrom<MandateDetails> for diesel_models::enums::MandateDetails {
    fn foreign_from(from: MandateDetails) -> Self {
        Self {
            update_mandate_id: from.update_mandate_id,
        }
    }
}

impl ForeignFrom<diesel_models::enums::MandateDetails> for MandateDetails {
    fn foreign_from(from: diesel_models::enums::MandateDetails) -> Self {
        Self {
            update_mandate_id: from.update_mandate_id,
        }
    }
}

impl ForeignFrom<MandateDataType> for diesel_models::enums::MandateDataType {
    fn foreign_from(from: MandateDataType) -> Self {
        use crate::DataModelExt;
        match from {
            MandateDataType::SingleUse(data) => Self::SingleUse(data.to_storage_model()),
            MandateDataType::MultiUse(None) => Self::MultiUse(None),
            MandateDataType::MultiUse(Some(data)) => Self::MultiUse(Some(data.to_storage_model())),
        }
    }
}

impl ForeignFrom<diesel_models::enums::MandateDataType> for MandateDataType {
    fn foreign_from(from: diesel_models::enums::MandateDataType) -> Self {
        use crate::DataModelExt;
        use diesel_models::enums::MandateDataType as DieselMandateDataType;

        match from {
            DieselMandateDataType::SingleUse(data) => {
                Self::SingleUse(MandateAmountData::from_storage_model(data))
            }
            DieselMandateDataType::MultiUse(None) => Self::MultiUse(None),
            DieselMandateDataType::MultiUse(Some(data)) => {
                Self::MultiUse(Some(MandateAmountData::from_storage_model(data)))
            }
        }
    }
}

impl ForeignFrom<MandateAmountData> for diesel_models::enums::MandateAmountData {
    fn foreign_from(from: MandateAmountData) -> Self {
        Self {
            amount: from.amount,
            currency: from.currency,
            start_date: from.start_date,
            end_date: from.end_date,
            metadata: from.metadata,
        }
    }
}

impl ForeignFrom<diesel_models::enums::MandateAmountData> for MandateAmountData {
    fn foreign_from(from: diesel_models::enums::MandateAmountData) -> Self {
        Self {
            amount: from.amount,
            currency: from.currency,
            start_date: from.start_date,
            end_date: from.end_date,
            metadata: from.metadata,
        }
    }
}

impl ForeignFrom<diesel_models::CommonMandateReference> for CommonMandateReference {
    fn foreign_from(from: diesel_models::CommonMandateReference) -> Self {
        use crate::transformers::ForeignFrom;
        Self {
            payments: from
                .payments
                .map(|payments| PaymentsMandateReference::foreign_from(payments)),
            payouts: from
                .payouts
                .map(|payouts| PayoutsMandateReference::foreign_from(payouts)),
        }
    }
}

impl ForeignFrom<CommonMandateReference> for diesel_models::CommonMandateReference {
    fn foreign_from(from: CommonMandateReference) -> Self {
        use crate::transformers::ForeignFrom;
        Self {
            payments: from
                .payments
                .map(|payments| diesel_models::PaymentsMandateReference::foreign_from(payments)),
            payouts: from
                .payouts
                .map(|payouts| diesel_models::PayoutsMandateReference::foreign_from(payouts)),
        }
    }
}

impl ForeignFrom<diesel_models::PayoutsMandateReference> for PayoutsMandateReference {
    fn foreign_from(from: diesel_models::PayoutsMandateReference) -> Self {
        use crate::transformers::ForeignFrom;
        Self(
            from.0
                .into_iter()
                .map(|(key, record)| (key, PayoutsMandateReferenceRecord::foreign_from(record)))
                .collect(),
        )
    }
}

impl ForeignFrom<PayoutsMandateReference> for diesel_models::PayoutsMandateReference {
    fn foreign_from(from: PayoutsMandateReference) -> Self {
        use crate::transformers::ForeignFrom;
        Self(
            from.0
                .into_iter()
                .map(|(key, record)| {
                    (
                        key,
                        diesel_models::PayoutsMandateReferenceRecord::foreign_from(record),
                    )
                })
                .collect(),
        )
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<diesel_models::PaymentsMandateReference> for PaymentsMandateReference {
    fn foreign_from(from: diesel_models::PaymentsMandateReference) -> Self {
        use crate::transformers::ForeignFrom;
        Self(
            from.0
                .into_iter()
                .map(|(key, record)| (key, PaymentsMandateReferenceRecord::foreign_from(record)))
                .collect(),
        )
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<PaymentsMandateReference> for diesel_models::PaymentsMandateReference {
    fn foreign_from(from: PaymentsMandateReference) -> Self {
        use crate::transformers::ForeignFrom;
        Self(
            from.0
                .into_iter()
                .map(|(key, record)| {
                    (
                        key,
                        diesel_models::PaymentsMandateReferenceRecord::foreign_from(record),
                    )
                })
                .collect(),
        )
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<diesel_models::PaymentsTokenReference> for PaymentsTokenReference {
    fn foreign_from(from: diesel_models::PaymentsTokenReference) -> Self {
        use crate::transformers::ForeignFrom;
        Self(
            from.0
                .into_iter()
                .map(|(key, record)| (key, ConnectorTokenReferenceRecord::foreign_from(record)))
                .collect(),
        )
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<PaymentsTokenReference> for diesel_models::PaymentsTokenReference {
    fn foreign_from(from: PaymentsTokenReference) -> Self {
        use crate::transformers::ForeignFrom;
        Self(
            from.0
                .into_iter()
                .map(|(key, record)| {
                    (
                        key,
                        diesel_models::ConnectorTokenReferenceRecord::foreign_from(record),
                    )
                })
                .collect(),
        )
    }
}

impl ForeignFrom<diesel_models::PayoutsMandateReferenceRecord> for PayoutsMandateReferenceRecord {
    fn foreign_from(from: diesel_models::PayoutsMandateReferenceRecord) -> Self {
        Self {
            transfer_method_id: from.transfer_method_id,
            connector_customer_id: from.connector_customer_id,
        }
    }
}

impl ForeignFrom<PayoutsMandateReferenceRecord> for diesel_models::PayoutsMandateReferenceRecord {
    fn foreign_from(from: PayoutsMandateReferenceRecord) -> Self {
        Self {
            transfer_method_id: from.transfer_method_id,
            connector_customer_id: from.connector_customer_id,
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<diesel_models::PaymentsMandateReferenceRecord> for PaymentsMandateReferenceRecord {
    fn foreign_from(from: diesel_models::PaymentsMandateReferenceRecord) -> Self {
        Self {
            connector_mandate_id: from.connector_mandate_id,
            payment_method_type: from.payment_method_type,
            original_payment_authorized_amount: from.original_payment_authorized_amount,
            original_payment_authorized_currency: from.original_payment_authorized_currency,
            mandate_metadata: from.mandate_metadata,
            connector_mandate_status: from.connector_mandate_status,
            connector_mandate_request_reference_id: from.connector_mandate_request_reference_id,
            connector_customer_id: from.connector_customer_id,
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<PaymentsMandateReferenceRecord> for diesel_models::PaymentsMandateReferenceRecord {
    fn foreign_from(from: PaymentsMandateReferenceRecord) -> Self {
        Self {
            connector_mandate_id: from.connector_mandate_id,
            payment_method_type: from.payment_method_type,
            original_payment_authorized_amount: from.original_payment_authorized_amount,
            original_payment_authorized_currency: from.original_payment_authorized_currency,
            mandate_metadata: from.mandate_metadata,
            connector_mandate_status: from.connector_mandate_status,
            connector_mandate_request_reference_id: from.connector_mandate_request_reference_id,
            connector_customer_id: from.connector_customer_id,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<diesel_models::ConnectorTokenReferenceRecord> for ConnectorTokenReferenceRecord {
    fn foreign_from(from: diesel_models::ConnectorTokenReferenceRecord) -> Self {
        let diesel_models::ConnectorTokenReferenceRecord {
            connector_token,
            payment_method_subtype,
            original_payment_authorized_amount,
            original_payment_authorized_currency,
            metadata,
            connector_token_status,
            connector_token_request_reference_id,
            connector_customer_id,
        } = from;
        Self {
            connector_token,
            payment_method_subtype,
            original_payment_authorized_amount,
            original_payment_authorized_currency,
            metadata,
            connector_token_status,
            connector_token_request_reference_id,
            connector_customer_id,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<ConnectorTokenReferenceRecord> for diesel_models::ConnectorTokenReferenceRecord {
    fn foreign_from(from: PaymentsMandateReferenceRecord) -> Self {
        Self {
            connector_mandate_id: from.connector_mandate_id,
            payment_method_type: from.payment_method_type,
            original_payment_authorized_amount: from.original_payment_authorized_amount,
            original_payment_authorized_currency: from.original_payment_authorized_currency,
            mandate_metadata: from.mandate_metadata,
            connector_mandate_status: from.connector_mandate_status,
            connector_mandate_request_reference_id: from.connector_mandate_request_reference_id,
            connector_customer_id: from.connector_customer_id,
        }
    }
}
