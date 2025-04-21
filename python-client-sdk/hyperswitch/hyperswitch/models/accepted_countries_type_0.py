from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.accepted_countries_type_0_type import AcceptedCountriesType0Type
from ..models.country_alpha_2 import CountryAlpha2

T = TypeVar("T", bound="AcceptedCountriesType0")


@_attrs_define
class AcceptedCountriesType0:
    """
    Attributes:
        type_ (AcceptedCountriesType0Type):
        list_ (list[CountryAlpha2]):
    """

    type_: AcceptedCountriesType0Type
    list_: list[CountryAlpha2]
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
        type_ = AcceptedCountriesType0Type(d.pop("type"))

        list_ = []
        _list_ = d.pop("list")
        for list_item_data in _list_:
            list_item = CountryAlpha2(list_item_data)

            list_.append(list_item)

        accepted_countries_type_0 = cls(
            type_=type_,
            list_=list_,
        )

        accepted_countries_type_0.additional_properties = d
        return accepted_countries_type_0

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
