//! Transformer traits for converting between foreign types and native types.
use common_enums::enums::{CanadaStatesAbbreviation, UsStatesAbbreviation};
use common_utils::ext_traits::StringExt;
use hyperswitch_interfaces::errors;

/// ForeignInto trait
pub trait ForeignInto<T> {
    /// Convert from a native type to a foreign type.
    fn foreign_into(self) -> T;
}

/// ForeignTryInto trait
pub trait ForeignTryInto<T> {
    /// The error type that can be returned when converting.
    type Error;
    /// Convert from a foreign type to a native type.
    fn foreign_try_into(self) -> Result<T, Self::Error>;
}

/// ForeignFrom trait
pub trait ForeignFrom<F> {
    /// Convert from a foreign type to a native type.
    fn foreign_from(from: F) -> Self;
}

/// ForeignTryFrom trait
pub trait ForeignTryFrom<F>: Sized {
    /// The error type that can be returned when converting.
    type Error;
    /// Convert from a foreign type to a native type.
    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

/// ForeignInto implementation
impl<F, T> ForeignInto<T> for F
where
    T: ForeignFrom<F>,
{
    fn foreign_into(self) -> T {
        T::foreign_from(self)
    }
}

/// ForeignTryInto implementation
impl<F, T> ForeignTryInto<T> for F
where
    T: ForeignTryFrom<F>,
{
    type Error = <T as ForeignTryFrom<F>>::Error;

    fn foreign_try_into(self) -> Result<T, Self::Error> {
        T::foreign_try_from(self)
    }
}

impl ForeignTryFrom<String> for UsStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "UsStatesAbbreviation");

        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "alabama" => Ok(Self::AL),
                    "alaska" => Ok(Self::AK),
                    "american samoa" => Ok(Self::AS),
                    "arizona" => Ok(Self::AZ),
                    "arkansas" => Ok(Self::AR),
                    "california" => Ok(Self::CA),
                    "colorado" => Ok(Self::CO),
                    "connecticut" => Ok(Self::CT),
                    "delaware" => Ok(Self::DE),
                    "district of columbia" | "columbia" => Ok(Self::DC),
                    "federated states of micronesia" | "micronesia" => Ok(Self::FM),
                    "florida" => Ok(Self::FL),
                    "georgia" => Ok(Self::GA),
                    "guam" => Ok(Self::GU),
                    "hawaii" => Ok(Self::HI),
                    "idaho" => Ok(Self::ID),
                    "illinois" => Ok(Self::IL),
                    "indiana" => Ok(Self::IN),
                    "iowa" => Ok(Self::IA),
                    "kansas" => Ok(Self::KS),
                    "kentucky" => Ok(Self::KY),
                    "louisiana" => Ok(Self::LA),
                    "maine" => Ok(Self::ME),
                    "marshall islands" => Ok(Self::MH),
                    "maryland" => Ok(Self::MD),
                    "massachusetts" => Ok(Self::MA),
                    "michigan" => Ok(Self::MI),
                    "minnesota" => Ok(Self::MN),
                    "mississippi" => Ok(Self::MS),
                    "missouri" => Ok(Self::MO),
                    "montana" => Ok(Self::MT),
                    "nebraska" => Ok(Self::NE),
                    "nevada" => Ok(Self::NV),
                    "new hampshire" => Ok(Self::NH),
                    "new jersey" => Ok(Self::NJ),
                    "new mexico" => Ok(Self::NM),
                    "new york" => Ok(Self::NY),
                    "north carolina" => Ok(Self::NC),
                    "north dakota" => Ok(Self::ND),
                    "northern mariana islands" => Ok(Self::MP),
                    "ohio" => Ok(Self::OH),
                    "oklahoma" => Ok(Self::OK),
                    "oregon" => Ok(Self::OR),
                    "palau" => Ok(Self::PW),
                    "pennsylvania" => Ok(Self::PA),
                    "puerto rico" => Ok(Self::PR),
                    "rhode island" => Ok(Self::RI),
                    "south carolina" => Ok(Self::SC),
                    "south dakota" => Ok(Self::SD),
                    "tennessee" => Ok(Self::TN),
                    "texas" => Ok(Self::TX),
                    "utah" => Ok(Self::UT),
                    "vermont" => Ok(Self::VT),
                    "virgin islands" => Ok(Self::VI),
                    "virginia" => Ok(Self::VA),
                    "washington" => Ok(Self::WA),
                    "west virginia" => Ok(Self::WV),
                    "wisconsin" => Ok(Self::WI),
                    "wyoming" => Ok(Self::WY),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}

impl ForeignTryFrom<String> for CanadaStatesAbbreviation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let state_abbreviation_check =
            StringExt::<Self>::parse_enum(value.to_uppercase().clone(), "CanadaStatesAbbreviation");
        match state_abbreviation_check {
            Ok(state_abbreviation) => Ok(state_abbreviation),
            Err(_) => {
                let binding = value.as_str().to_lowercase();
                let state = binding.as_str();
                match state {
                    "alberta" => Ok(Self::AB),
                    "british columbia" => Ok(Self::BC),
                    "manitoba" => Ok(Self::MB),
                    "new brunswick" => Ok(Self::NB),
                    "newfoundland and labrador" | "newfoundland & labrador" => Ok(Self::NL),
                    "northwest territories" => Ok(Self::NT),
                    "nova scotia" => Ok(Self::NS),
                    "nunavut" => Ok(Self::NU),
                    "ontario" => Ok(Self::ON),
                    "prince edward island" => Ok(Self::PE),
                    "quebec" => Ok(Self::QC),
                    "saskatchewan" => Ok(Self::SK),
                    "yukon" => Ok(Self::YT),
                    _ => Err(errors::ConnectorError::InvalidDataFormat {
                        field_name: "address.state",
                    }
                    .into()),
                }
            }
        }
    }
}
