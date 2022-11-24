use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};

use crate::errors::{self, CustomResult};

pub trait Encode<'e, P>
where
    Self: 'e + std::fmt::Debug,
{
    // If needed get type information/custom error implementation.
    fn convert_and_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: error_stack::ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize;

    fn convert_and_url_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: error_stack::ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize;

    fn encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize;

    fn encode_to_string_of_json(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize;

    fn encode_to_value(&'e self) -> CustomResult<serde_json::Value, errors::ParsingError>
    where
        Self: Serialize;

    fn encode_to_vec(&'e self) -> CustomResult<Vec<u8>, errors::ParsingError>
    where
        Self: Serialize;
}

impl<'e, P, A> Encode<'e, P> for A
where
    Self: 'e + std::fmt::Debug,
{
    fn convert_and_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: error_stack::ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize,
    {
        serde_json::to_string(&P::try_from(self).change_context(errors::ParsingError)?)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {:?} to a request", self))
    }

    fn convert_and_url_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: error_stack::ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize,
    {
        serde_urlencoded::to_string(&P::try_from(self).change_context(errors::ParsingError)?)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {:?} to a request", self))
    }

    // Check without two functions can we combine this
    fn encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_urlencoded::to_string(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {:?} to a request", self))
    }

    fn encode_to_string_of_json(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_string(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {:?} to a request", self))
    }

    fn encode_to_value(&'e self) -> CustomResult<serde_json::Value, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_value(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {:?} to a value", self))
    }

    fn encode_to_vec(&'e self) -> CustomResult<Vec<u8>, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_vec(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {:?} to a value", self))
    }
}

pub trait BytesExt<T> {
    fn parse_struct<'de>(&'de self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>;
}

impl<T> BytesExt<T> for bytes::Bytes {
    fn parse_struct<'de>(&'de self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        use bytes::Buf;

        serde_json::from_slice::<T>(self.chunk())
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to parse {type_name} from bytes"))
    }
}

pub trait ByteSliceExt<T> {
    fn parse_struct<'de>(&'de self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>;
}

impl<T> ByteSliceExt<T> for [u8] {
    fn parse_struct<'de>(&'de self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        serde_json::from_slice(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to parse {type_name} from &[u8]"))
    }
}
