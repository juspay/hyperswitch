from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="BankTransferTypes")


@_attrs_define
class BankTransferTypes:
    """
    Attributes:
        eligible_connectors (list[str]): The list of eligible connectors for a given payment experience Example:
            ['stripe', 'adyen'].
    """

    eligible_connectors: list[str]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        eligible_connectors = self.eligible_connectors

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "eligible_connectors": eligible_connectors,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        eligible_connectors = cast(list[str], d.pop("eligible_connectors"))

        bank_transfer_types = cls(
            eligible_connectors=eligible_connectors,
        )

        bank_transfer_types.additional_properties = d
        return bank_transfer_types

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
