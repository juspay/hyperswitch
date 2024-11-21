use scylla::{
    cql_to_rust::FromCqlVal,
    deserialize::DeserializeValue,
    frame::response::result::{ColumnType, CqlValue},
    serialize::{
        value::SerializeValue,
        writers::{CellWriter, WrittenCellProof},
        SerializationError,
    },
};

use crate::{abs::PeekInterface, StrongSecret};

impl<T> SerializeValue for StrongSecret<T>
where
    T: SerializeValue + zeroize::Zeroize + Clone,
{
    fn serialize<'b>(
        &self,
        typ: &ColumnType,
        writer: CellWriter<'b>,
    ) -> Result<WrittenCellProof<'b>, SerializationError> {
        self.peek().serialize(typ, writer)
    }
}

impl<'frame, T> DeserializeValue<'frame> for StrongSecret<T>
where
    T: DeserializeValue<'frame> + zeroize::Zeroize + Clone,
{
    fn type_check(typ: &ColumnType) -> Result<(), scylla::deserialize::TypeCheckError> {
        T::type_check(typ)
    }

    fn deserialize(
        typ: &'frame ColumnType,
        v: Option<scylla::deserialize::FrameSlice<'frame>>,
    ) -> Result<Self, scylla::deserialize::DeserializationError> {
        Ok(Self::new(T::deserialize(typ, v)?))
    }
}

impl<T> FromCqlVal<CqlValue> for StrongSecret<T>
where
    T: FromCqlVal<CqlValue> + zeroize::Zeroize + Clone,
{
    fn from_cql(cql_val: CqlValue) -> Result<Self, scylla::cql_to_rust::FromCqlValError> {
        Ok(Self::new(T::from_cql(cql_val)?))
    }
}
