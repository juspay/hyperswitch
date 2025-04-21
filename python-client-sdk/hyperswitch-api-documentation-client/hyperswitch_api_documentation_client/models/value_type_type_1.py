from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.value_type_type_1_type import ValueTypeType1Type

T = TypeVar("T", bound="ValueTypeType1")


@_attrs_define
class ValueTypeType1:
    """
    Attributes:
        type_ (ValueTypeType1Type):
        value (str): Represents an enum variant
    """

    type_: ValueTypeType1Type
    value: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_.value

        value = self.value

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
        d = dict(src_dict)
        type_ = ValueTypeType1Type(d.pop("type"))

        value = d.pop("value")

        value_type_type_1 = cls(
            type_=type_,
            value=value,
        )

        value_type_type_1.additional_properties = d
        return value_type_type_1

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
