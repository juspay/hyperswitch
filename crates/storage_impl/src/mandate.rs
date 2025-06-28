use diesel_models::Mandate;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::mandates::{ConnectorTokenReferenceRecord, PaymentsTokenReference};
use hyperswitch_domain_models::mandates::{CommonMandateReference, MandateAmountData, MandateDataType, MandateDetails, PayoutsMandateReference, PayoutsMandateReferenceRecord};

use crate::{redis::kv_store::KvStorePartition, utils::ForeignFrom};

impl KvStorePartition for Mandate {}

impl ForeignFrom<MandateDetails> for diesel_models::enums::MandateDetails {
    fn foreign_from(value: MandateDetails) -> Self {
        Self {
            update_mandate_id: value.update_mandate_id,
        }
    }
}

impl ForeignFrom<diesel_models::enums::MandateDetails> for MandateDetails {
    fn foreign_from(value: diesel_models::enums::MandateDetails) -> Self {
        Self {
            update_mandate_id: value.update_mandate_id,
        }
    }
}

impl ForeignFrom<MandateDataType> for diesel_models::enums::MandateDataType {
    fn foreign_from(value: MandateDataType) -> Self {
        match value {
            MandateDataType::SingleUse(data) => Self::SingleUse(data.into()),
            MandateDataType::MultiUse(None) => Self::MultiUse(None),
            MandateDataType::MultiUse(Some(data)) => Self::MultiUse(Some(data.into())),
        }
    }
}

impl ForeignFrom<diesel_models::enums::MandateDataType> for MandateDataType {
    fn foreign_from(value: diesel_models::enums::MandateDataType) -> Self {
        use diesel_models::enums::MandateDataType as DieselMandateDataType;

        match value {
            DieselMandateDataType::SingleUse(data) => Self::SingleUse(data.into()),
            DieselMandateDataType::MultiUse(None) => Self::MultiUse(None),
            DieselMandateDataType::MultiUse(Some(data)) => Self::MultiUse(Some(data.into())),
        }
    }
}

impl ForeignFrom<MandateAmountData> for diesel_models::enums::MandateAmountData {
    fn foreign_from(value: MandateAmountData) -> Self {
        Self {
            amount: value.amount,
            currency: value.currency,
            start_date: value.start_date,
            end_date: value.end_date,
            metadata: value.metadata,
        }
    }
}

impl ForeignFrom<diesel_models::enums::MandateAmountData> for MandateAmountData {
    fn foreign_from(value: diesel_models::enums::MandateAmountData) -> Self {
        Self {
            amount: value.amount,
            currency: value.currency,
            start_date: value.start_date,
            end_date: value.end_date,
            metadata: value.metadata,
        }
    }
}


impl ForeignFrom<diesel_models::CommonMandateReference> for CommonMandateReference {
    fn foreign_from(value: diesel_models::CommonMandateReference) -> Self {
        Self {
            payments: value.payments.map(|payments| payments.into()),
            payouts: value.payouts.map(|payouts| payouts.into()),
        }
    }
}

impl ForeignFrom<CommonMandateReference> for diesel_models::CommonMandateReference {
    fn foreign_from(value: CommonMandateReference) -> Self {
        Self {
            payments: value.payments.map(|payments| payments.into()),
            payouts: value.payouts.map(|payouts| payouts.into()),
        }
    }
}

impl ForeignFrom<diesel_models::PayoutsMandateReference> for PayoutsMandateReference {
    fn foreign_from(value: diesel_models::PayoutsMandateReference) -> Self {
        Self(
            value
                .0
                .into_iter()
                .map(|(key, record)| (key, record.into()))
                .collect(),
        )
    }
}

impl ForeignFrom<PayoutsMandateReference> for diesel_models::PayoutsMandateReference {
    fn foreign_from(value: PayoutsMandateReference) -> Self {
        Self(
            value
                .0
                .into_iter()
                .map(|(key, record)| (key, record.into()))
                .collect(),
        )
    }
}


#[cfg(feature = "v2")]
impl ForeignFrom<diesel_models::PaymentsTokenReference> for PaymentsTokenReference {
    fn foreign_from(value: diesel_models::PaymentsTokenReference) -> Self {
        Self(
            value
                .0
                .into_iter()
                .map(|(key, record)| (key, record.into()))
                .collect(),
        )
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<PaymentsTokenReference> for diesel_models::PaymentsTokenReference {
    fn foreign_from(value: PaymentsTokenReference) -> Self {
        Self(
            value
                .0
                .into_iter()
                .map(|(key, record)| (key, record.into()))
                .collect(),
        )
    }
}

impl ForeignFrom<diesel_models::PayoutsMandateReferenceRecord> for PayoutsMandateReferenceRecord {
    fn foreign_from(value: diesel_models::PayoutsMandateReferenceRecord) -> Self {
        Self {
            transfer_method_id: value.transfer_method_id,
        }
    }
}

impl ForeignFrom<PayoutsMandateReferenceRecord> for diesel_models::PayoutsMandateReferenceRecord {
    fn foreign_from(value: PayoutsMandateReferenceRecord) -> Self {
        Self {
            transfer_method_id: value.transfer_method_id,
        }
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<diesel_models::ConnectorTokenReferenceRecord> for ConnectorTokenReferenceRecord {
    fn foreign_from(value: diesel_models::ConnectorTokenReferenceRecord) -> Self {
        let diesel_models::ConnectorTokenReferenceRecord {
            connector_token,
            payment_method_subtype,
            original_payment_authorized_amount,
            original_payment_authorized_currency,
            metadata,
            connector_token_status,
            connector_token_request_reference_id,
        } = value;
        Self {
            connector_token,
            payment_method_subtype,
            original_payment_authorized_amount,
            original_payment_authorized_currency,
            metadata,
            connector_token_status,
            connector_token_request_reference_id,
        }
    }
}


#[cfg(feature = "v2")]
impl ForeignFrom<ConnectorTokenReferenceRecord> for diesel_models::ConnectorTokenReferenceRecord {
    fn foreign_from(value: ConnectorTokenReferenceRecord) -> Self {
        let ConnectorTokenReferenceRecord {
            connector_token,
            payment_method_subtype,
            original_payment_authorized_amount,
            original_payment_authorized_currency,
            metadata,
            connector_token_status,
            connector_token_request_reference_id,
        } = value;
        Self {
            connector_token,
            payment_method_subtype,
            original_payment_authorized_amount,
            original_payment_authorized_currency,
            metadata,
            connector_token_status,
            connector_token_request_reference_id,
        }
    }
}
