from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="NoonData")


@_attrs_define
class NoonData:
    """
    Attributes:
        order_category (Union[None, Unset, str]): Information about the order category that merchant wants to specify at
            connector level. (e.g. In Noon Payments it can take values like "pay", "food", or any other custom string set by
            the merchant in Noon's Dashboard)
    """

    order_category: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        order_category: Union[None, Unset, str]
        if isinstance(self.order_category, Unset):
            order_category = UNSET
        else:
            order_category = self.order_category

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if order_category is not UNSET:
            field_dict["order_category"] = order_category

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_order_category(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        order_category = _parse_order_category(d.pop("order_category", UNSET))

        noon_data = cls(
            order_category=order_category,
        )

        noon_data.additional_properties = d
        return noon_data

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
