from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.value_type_type_5_type import ValueTypeType5Type

T = TypeVar("T", bound="ValueTypeType5")


@_attrs_define
class ValueTypeType5:
    """
    Attributes:
        type_ (ValueTypeType5Type):
        value (list[str]): Similar to NumberArray but for enum variants
            eg: payment.method.cardtype = (debit, credit)
    """

    type_: ValueTypeType5Type
    value: list[str]
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
        type_ = ValueTypeType5Type(d.pop("type"))

        value = cast(list[str], d.pop("value"))

        value_type_type_5 = cls(
            type_=type_,
            value=value,
        )

        value_type_type_5.additional_properties = d
        return value_type_type_5

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
