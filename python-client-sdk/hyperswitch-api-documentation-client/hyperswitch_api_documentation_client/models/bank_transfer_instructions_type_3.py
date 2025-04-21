from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.bacs_bank_transfer_instructions import BacsBankTransferInstructions


T = TypeVar("T", bound="BankTransferInstructionsType3")


@_attrs_define
class BankTransferInstructionsType3:
    """
    Attributes:
        bacs_bank_instructions (BacsBankTransferInstructions):
    """

    bacs_bank_instructions: "BacsBankTransferInstructions"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bacs_bank_instructions = self.bacs_bank_instructions.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "bacs_bank_instructions": bacs_bank_instructions,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.bacs_bank_transfer_instructions import BacsBankTransferInstructions

        d = dict(src_dict)
        bacs_bank_instructions = BacsBankTransferInstructions.from_dict(d.pop("bacs_bank_instructions"))

        bank_transfer_instructions_type_3 = cls(
            bacs_bank_instructions=bacs_bank_instructions,
        )

        bank_transfer_instructions_type_3.additional_properties = d
        return bank_transfer_instructions_type_3

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
