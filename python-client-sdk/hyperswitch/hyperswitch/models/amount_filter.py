from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="AmountFilter")


@_attrs_define
class AmountFilter:
    """
    Attributes:
        start_amount (Union[None, Unset, int]): The start amount to filter list of transactions which are greater than
            or equal to the start amount
        end_amount (Union[None, Unset, int]): The end amount to filter list of transactions which are less than or equal
            to the end amount
    """

    start_amount: Union[None, Unset, int] = UNSET
    end_amount: Union[None, Unset, int] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        start_amount: Union[None, Unset, int]
        if isinstance(self.start_amount, Unset):
            start_amount = UNSET
        else:
            start_amount = self.start_amount

        end_amount: Union[None, Unset, int]
        if isinstance(self.end_amount, Unset):
            end_amount = UNSET
        else:
            end_amount = self.end_amount

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if start_amount is not UNSET:
            field_dict["start_amount"] = start_amount
        if end_amount is not UNSET:
            field_dict["end_amount"] = end_amount

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_start_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        start_amount = _parse_start_amount(d.pop("start_amount", UNSET))

        def _parse_end_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        end_amount = _parse_end_amount(d.pop("end_amount", UNSET))

        amount_filter = cls(
            start_amount=start_amount,
            end_amount=end_amount,
        )

        amount_filter.additional_properties = d
        return amount_filter

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
