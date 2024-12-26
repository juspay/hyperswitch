use scylla::{
    deserialize::DeserializeValue,
    frame::response::result::ColumnType,
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
        typ: &ColumnType<'_>,
        writer: CellWriter<'b>,
    ) -> Result<WrittenCellProof<'b>, SerializationError> {
        self.peek().serialize(typ, writer)
    }
}

impl<'frame, 'metadata, T> DeserializeValue<'frame, 'metadata> for StrongSecret<T>
where
    T: DeserializeValue<'frame, 'metadata> + zeroize::Zeroize + Clone,
{
    fn type_check(typ: &ColumnType<'_>) -> Result<(), scylla::deserialize::TypeCheckError> {
        T::type_check(typ)
    }

    fn deserialize(
        typ: &'metadata ColumnType<'metadata>,
        v: Option<scylla::deserialize::FrameSlice<'frame>>,
    ) -> Result<Self, scylla::deserialize::DeserializationError> {
        Ok(Self::new(T::deserialize(typ, v)?))
    }
}
