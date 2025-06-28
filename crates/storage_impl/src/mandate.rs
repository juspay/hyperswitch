use diesel_models::Mandate;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for Mandate {}

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


#[cfg(feature = "v2")]
impl From<diesel_models::PaymentsTokenReference> for PaymentsTokenReference {
    fn from(value: diesel_models::PaymentsTokenReference) -> Self {
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
impl From<PaymentsTokenReference> for diesel_models::PaymentsTokenReference {
    fn from(value: PaymentsTokenReference) -> Self {
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

#[cfg(feature = "v2")]
impl From<diesel_models::ConnectorTokenReferenceRecord> for ConnectorTokenReferenceRecord {
    fn from(value: diesel_models::ConnectorTokenReferenceRecord) -> Self {
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
impl From<ConnectorTokenReferenceRecord> for diesel_models::ConnectorTokenReferenceRecord {
    fn from(value: ConnectorTokenReferenceRecord) -> Self {
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
