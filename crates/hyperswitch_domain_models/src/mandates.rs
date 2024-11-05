use api_models::payments::{
    AcceptanceType as ApiAcceptanceType, CustomerAcceptance as ApiCustomerAcceptance,
    MandateAmountData as ApiMandateAmountData, MandateData as ApiMandateData, MandateType,
    OnlineMandate as ApiOnlineMandate,
};
use common_enums::Currency;
use common_utils::{date_time, errors::ParsingError, pii, types::MinorUnit};
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
    /// A concent from the customer to store the payment method
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
