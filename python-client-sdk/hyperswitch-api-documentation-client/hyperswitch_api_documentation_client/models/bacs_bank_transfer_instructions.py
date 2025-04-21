from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="BacsBankTransferInstructions")


@_attrs_define
class BacsBankTransferInstructions:
    """
    Attributes:
        account_holder_name (str):  Example: Jane Doe.
        account_number (str):  Example: 10244123908.
        sort_code (str):  Example: 012.
    """

    account_holder_name: str
    account_number: str
    sort_code: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        account_holder_name = self.account_holder_name

        account_number = self.account_number

        sort_code = self.sort_code

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "account_holder_name": account_holder_name,
                "account_number": account_number,
                "sort_code": sort_code,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        account_holder_name = d.pop("account_holder_name")

        account_number = d.pop("account_number")

        sort_code = d.pop("sort_code")

        bacs_bank_transfer_instructions = cls(
            account_holder_name=account_holder_name,
            account_number=account_number,
            sort_code=sort_code,
        )

        bacs_bank_transfer_instructions.additional_properties = d
        return bacs_bank_transfer_instructions

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
