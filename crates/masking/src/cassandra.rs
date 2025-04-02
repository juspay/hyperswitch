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
        column_type: &ColumnType<'_>,
        writer: CellWriter<'b>,
    ) -> Result<WrittenCellProof<'b>, SerializationError> {
        self.peek().serialize(column_type, writer)
    }
}

impl<'frame, 'metadata, T> DeserializeValue<'frame, 'metadata> for StrongSecret<T>
where
    T: DeserializeValue<'frame, 'metadata> + zeroize::Zeroize + Clone,
{
    fn type_check(column_type: &ColumnType<'_>) -> Result<(), scylla::deserialize::TypeCheckError> {
        T::type_check(column_type)
    }

    fn deserialize(
        column_type: &'metadata ColumnType<'metadata>,
        v: Option<scylla::deserialize::FrameSlice<'frame>>,
    ) -> Result<Self, scylla::deserialize::DeserializationError> {
        Ok(Self::new(T::deserialize(column_type, v)?))
    }
}
