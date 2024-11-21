use masking::{PeekInterface, Secret};

#[derive(Default, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Address {
    pub address: Option<AddressDetails>,
    pub phone: Option<PhoneDetails>,
    pub email: Option<common_utils::pii::Email>,
}

impl masking::SerializableSecret for Address {}

impl Address {
    /// Unify the address, giving priority to `self` when details are present in both
    pub fn unify_address(&self, other: Option<&Self>) -> Self {
        let other_address_details = other.and_then(|address| address.address.as_ref());
        Self {
            address: self
                .address
                .as_ref()
                .map(|address| address.unify_address_details(other_address_details))
                .or(other_address_details.cloned()),
            email: self
                .email
                .clone()
                .or(other.and_then(|other| other.email.clone())),
            phone: self
                .phone
                .clone()
                .or(other.and_then(|other| other.phone.clone())),
        }
    }
}

#[derive(Clone, Default, Debug, Eq, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct AddressDetails {
    pub city: Option<String>,
    pub country: Option<common_enums::CountryAlpha2>,
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub line3: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
}

impl AddressDetails {
    pub fn get_optional_full_name(&self) -> Option<Secret<String>> {
        match (self.first_name.as_ref(), self.last_name.as_ref()) {
            (Some(first_name), Some(last_name)) => Some(Secret::new(format!(
                "{} {}",
                first_name.peek(),
                last_name.peek()
            ))),
            (Some(name), None) | (None, Some(name)) => Some(name.to_owned()),
            _ => None,
        }
    }

    /// Unify the address details, giving priority to `self` when details are present in both
    pub fn unify_address_details(&self, other: Option<&Self>) -> Self {
        if let Some(other) = other {
            let (first_name, last_name) = if self
                .first_name
                .as_ref()
                .is_some_and(|first_name| !first_name.peek().trim().is_empty())
            {
                (self.first_name.clone(), self.last_name.clone())
            } else {
                (other.first_name.clone(), other.last_name.clone())
            };

            Self {
                first_name,
                last_name,
                city: self.city.clone().or(other.city.clone()),
                country: self.country.or(other.country),
                line1: self.line1.clone().or(other.line1.clone()),
                line2: self.line2.clone().or(other.line2.clone()),
                line3: self.line3.clone().or(other.line3.clone()),
                zip: self.zip.clone().or(other.zip.clone()),
                state: self.state.clone().or(other.state.clone()),
            }
        } else {
            self.clone()
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PhoneDetails {
    pub number: Option<Secret<String>>,
    pub country_code: Option<String>,
}

impl From<api_models::payments::Address> for Address {
    fn from(address: api_models::payments::Address) -> Self {
        Self {
            address: address.address.map(AddressDetails::from),
            phone: address.phone.map(PhoneDetails::from),
            email: address.email,
        }
    }
}

impl From<api_models::payments::AddressDetails> for AddressDetails {
    fn from(address: api_models::payments::AddressDetails) -> Self {
        Self {
            city: address.city,
            country: address.country,
            line1: address.line1,
            line2: address.line2,
            line3: address.line3,
            zip: address.zip,
            state: address.state,
            first_name: address.first_name,
            last_name: address.last_name,
        }
    }
}

impl From<api_models::payments::PhoneDetails> for PhoneDetails {
    fn from(phone: api_models::payments::PhoneDetails) -> Self {
        Self {
            number: phone.number,
            country_code: phone.country_code,
        }
    }
}

impl From<Address> for api_models::payments::Address {
    fn from(address: Address) -> Self {
        Self {
            address: address
                .address
                .map(api_models::payments::AddressDetails::from),
            phone: address.phone.map(api_models::payments::PhoneDetails::from),
            email: address.email,
        }
    }
}

impl From<AddressDetails> for api_models::payments::AddressDetails {
    fn from(address: AddressDetails) -> Self {
        Self {
            city: address.city,
            country: address.country,
            line1: address.line1,
            line2: address.line2,
            line3: address.line3,
            zip: address.zip,
            state: address.state,
            first_name: address.first_name,
            last_name: address.last_name,
        }
    }
}

impl From<PhoneDetails> for api_models::payments::PhoneDetails {
    fn from(phone: PhoneDetails) -> Self {
        Self {
            number: phone.number,
            country_code: phone.country_code,
        }
    }
}
