from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.bank_names import BankNames

T = TypeVar("T", bound="BankCodeResponse")


@_attrs_define
class BankCodeResponse:
    """
    Attributes:
        bank_name (list[BankNames]):
        eligible_connectors (list[str]):
    """

    bank_name: list[BankNames]
    eligible_connectors: list[str]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bank_name = []
        for bank_name_item_data in self.bank_name:
            bank_name_item = bank_name_item_data.value
            bank_name.append(bank_name_item)

        eligible_connectors = self.eligible_connectors

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "bank_name": bank_name,
                "eligible_connectors": eligible_connectors,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        bank_name = []
        _bank_name = d.pop("bank_name")
        for bank_name_item_data in _bank_name:
            bank_name_item = BankNames(bank_name_item_data)

            bank_name.append(bank_name_item)

        eligible_connectors = cast(list[str], d.pop("eligible_connectors"))

        bank_code_response = cls(
            bank_name=bank_name,
            eligible_connectors=eligible_connectors,
        )

        bank_code_response.additional_properties = d
        return bank_code_response

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
