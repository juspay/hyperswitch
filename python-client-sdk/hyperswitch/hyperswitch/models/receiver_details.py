from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ReceiverDetails")


@_attrs_define
class ReceiverDetails:
    """
    Attributes:
        amount_received (int): The amount received by receiver
        amount_charged (Union[None, Unset, int]): The amount charged by ACH
        amount_remaining (Union[None, Unset, int]): The amount remaining to be sent via ACH
    """

    amount_received: int
    amount_charged: Union[None, Unset, int] = UNSET
    amount_remaining: Union[None, Unset, int] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        amount_received = self.amount_received

        amount_charged: Union[None, Unset, int]
        if isinstance(self.amount_charged, Unset):
            amount_charged = UNSET
        else:
            amount_charged = self.amount_charged

        amount_remaining: Union[None, Unset, int]
        if isinstance(self.amount_remaining, Unset):
            amount_remaining = UNSET
        else:
            amount_remaining = self.amount_remaining

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "amount_received": amount_received,
            }
        )
        if amount_charged is not UNSET:
            field_dict["amount_charged"] = amount_charged
        if amount_remaining is not UNSET:
            field_dict["amount_remaining"] = amount_remaining

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        amount_received = d.pop("amount_received")

        def _parse_amount_charged(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        amount_charged = _parse_amount_charged(d.pop("amount_charged", UNSET))

        def _parse_amount_remaining(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        amount_remaining = _parse_amount_remaining(d.pop("amount_remaining", UNSET))

        receiver_details = cls(
            amount_received=amount_received,
            amount_charged=amount_charged,
            amount_remaining=amount_remaining,
        )

        receiver_details.additional_properties = d
        return receiver_details

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
