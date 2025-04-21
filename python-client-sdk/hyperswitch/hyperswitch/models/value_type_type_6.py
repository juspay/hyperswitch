from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.value_type_type_6_type import ValueTypeType6Type

if TYPE_CHECKING:
    from ..models.number_comparison import NumberComparison


T = TypeVar("T", bound="ValueTypeType6")


@_attrs_define
class ValueTypeType6:
    """
    Attributes:
        type_ (ValueTypeType6Type):
        value (list['NumberComparison']): Like a number array but can include comparisons. Useful for
            conditions like "500 < amount < 1000"
            eg: payment.amount = (> 500, < 1000)
    """

    type_: ValueTypeType6Type
    value: list["NumberComparison"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_.value

        value = []
        for value_item_data in self.value:
            value_item = value_item_data.to_dict()
            value.append(value_item)

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
        from ..models.number_comparison import NumberComparison

        d = dict(src_dict)
        type_ = ValueTypeType6Type(d.pop("type"))

        value = []
        _value = d.pop("value")
        for value_item_data in _value:
            value_item = NumberComparison.from_dict(value_item_data)

            value.append(value_item)

        value_type_type_6 = cls(
            type_=type_,
            value=value,
        )

        value_type_type_6.additional_properties = d
        return value_type_type_6

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
