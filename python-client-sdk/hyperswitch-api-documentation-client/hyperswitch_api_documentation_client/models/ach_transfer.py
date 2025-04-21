from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="AchTransfer")


@_attrs_define
class AchTransfer:
    """
    Attributes:
        account_number (str):  Example: 122385736258.
        bank_name (str):
        routing_number (str):  Example: 012.
        swift_code (str):  Example: 234.
    """

    account_number: str
    bank_name: str
    routing_number: str
    swift_code: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        account_number = self.account_number

        bank_name = self.bank_name

        routing_number = self.routing_number

        swift_code = self.swift_code

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "account_number": account_number,
                "bank_name": bank_name,
                "routing_number": routing_number,
                "swift_code": swift_code,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        account_number = d.pop("account_number")

        bank_name = d.pop("bank_name")

        routing_number = d.pop("routing_number")

        swift_code = d.pop("swift_code")

        ach_transfer = cls(
            account_number=account_number,
            bank_name=bank_name,
            routing_number=routing_number,
            swift_code=swift_code,
        )

        ach_transfer.additional_properties = d
        return ach_transfer

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
