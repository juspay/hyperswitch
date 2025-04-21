from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.accepted_currencies_type_1_type import AcceptedCurrenciesType1Type
from ..models.currency import Currency

T = TypeVar("T", bound="AcceptedCurrenciesType1")


@_attrs_define
class AcceptedCurrenciesType1:
    """
    Attributes:
        type_ (AcceptedCurrenciesType1Type):
        list_ (list[Currency]):
    """

    type_: AcceptedCurrenciesType1Type
    list_: list[Currency]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_.value

        list_ = []
        for list_item_data in self.list_:
            list_item = list_item_data.value
            list_.append(list_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type_,
                "list": list_,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        type_ = AcceptedCurrenciesType1Type(d.pop("type"))

        list_ = []
        _list_ = d.pop("list")
        for list_item_data in _list_:
            list_item = Currency(list_item_data)

            list_.append(list_item)

        accepted_currencies_type_1 = cls(
            type_=type_,
            list_=list_,
        )

        accepted_currencies_type_1.additional_properties = d
        return accepted_currencies_type_1

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
