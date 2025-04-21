from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.sepa_bank_transfer_instructions import SepaBankTransferInstructions


T = TypeVar("T", bound="BankTransferInstructionsType2")


@_attrs_define
class BankTransferInstructionsType2:
    """
    Attributes:
        sepa_bank_instructions (SepaBankTransferInstructions):
    """

    sepa_bank_instructions: "SepaBankTransferInstructions"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        sepa_bank_instructions = self.sepa_bank_instructions.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "sepa_bank_instructions": sepa_bank_instructions,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.sepa_bank_transfer_instructions import SepaBankTransferInstructions

        d = dict(src_dict)
        sepa_bank_instructions = SepaBankTransferInstructions.from_dict(d.pop("sepa_bank_instructions"))

        bank_transfer_instructions_type_2 = cls(
            sepa_bank_instructions=sepa_bank_instructions,
        )

        bank_transfer_instructions_type_2.additional_properties = d
        return bank_transfer_instructions_type_2

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
