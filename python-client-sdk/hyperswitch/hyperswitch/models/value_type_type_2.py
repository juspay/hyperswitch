from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.value_type_type_2_type import ValueTypeType2Type

if TYPE_CHECKING:
    from ..models.metadata_value import MetadataValue


T = TypeVar("T", bound="ValueTypeType2")


@_attrs_define
class ValueTypeType2:
    """
    Attributes:
        type_ (ValueTypeType2Type):
        value (MetadataValue):
    """

    type_: ValueTypeType2Type
    value: "MetadataValue"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_.value

        value = self.value.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type_,
                "value": value,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.metadata_value import MetadataValue

        d = dict(src_dict)
        type_ = ValueTypeType2Type(d.pop("type"))

        value = MetadataValue.from_dict(d.pop("value"))

        value_type_type_2 = cls(
            type_=type_,
            value=value,
        )

        value_type_type_2.additional_properties = d
        return value_type_type_2

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
