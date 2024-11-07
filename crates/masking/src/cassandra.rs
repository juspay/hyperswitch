use crate::{abs::PeekInterface, StrongSecret};
use scylla::{
    cql_to_rust::FromCqlVal,
    frame::response::result::{ColumnType, CqlValue},
    serialize::{
        value::SerializeValue,
        writers::{CellWriter, WrittenCellProof},
        SerializationError,
    },
};

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

impl<T> FromCqlVal<CqlValue> for StrongSecret<T>
where
    T: FromCqlVal<CqlValue> + zeroize::Zeroize + Clone,
{
    fn from_cql(cql_val: CqlValue) -> Result<Self, scylla::cql_to_rust::FromCqlValError> {
        Ok(Self::new(T::from_cql(cql_val)?))
    }
}
