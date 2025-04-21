from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.doku_bank_transfer_instructions import DokuBankTransferInstructions


T = TypeVar("T", bound="BankTransferInstructionsType0")


@_attrs_define
class BankTransferInstructionsType0:
    """
    Attributes:
        doku_bank_transfer_instructions (DokuBankTransferInstructions):
    """

    doku_bank_transfer_instructions: "DokuBankTransferInstructions"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        doku_bank_transfer_instructions = self.doku_bank_transfer_instructions.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "doku_bank_transfer_instructions": doku_bank_transfer_instructions,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.doku_bank_transfer_instructions import DokuBankTransferInstructions

        d = dict(src_dict)
        doku_bank_transfer_instructions = DokuBankTransferInstructions.from_dict(
            d.pop("doku_bank_transfer_instructions")
        )

        bank_transfer_instructions_type_0 = cls(
            doku_bank_transfer_instructions=doku_bank_transfer_instructions,
        )

        bank_transfer_instructions_type_0.additional_properties = d
        return bank_transfer_instructions_type_0

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
