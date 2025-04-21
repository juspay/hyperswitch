from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="DokuBankTransferInstructions")


@_attrs_define
class DokuBankTransferInstructions:
    """
    Attributes:
        expires_at (str):  Example: 1707091200000.
        reference (str):  Example: 122385736258.
        instructions_url (str):
    """

    expires_at: str
    reference: str
    instructions_url: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        expires_at = self.expires_at

        reference = self.reference

        instructions_url = self.instructions_url

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "expires_at": expires_at,
                "reference": reference,
                "instructions_url": instructions_url,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        expires_at = d.pop("expires_at")

        reference = d.pop("reference")

        instructions_url = d.pop("instructions_url")

        doku_bank_transfer_instructions = cls(
            expires_at=expires_at,
            reference=reference,
            instructions_url=instructions_url,
        )

        doku_bank_transfer_instructions.additional_properties = d
        return doku_bank_transfer_instructions

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
