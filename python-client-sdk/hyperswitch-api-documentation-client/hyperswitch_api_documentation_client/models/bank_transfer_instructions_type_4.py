from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.multibanco_transfer_instructions import MultibancoTransferInstructions


T = TypeVar("T", bound="BankTransferInstructionsType4")


@_attrs_define
class BankTransferInstructionsType4:
    """
    Attributes:
        multibanco (MultibancoTransferInstructions):
    """

    multibanco: "MultibancoTransferInstructions"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        multibanco = self.multibanco.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "multibanco": multibanco,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.multibanco_transfer_instructions import MultibancoTransferInstructions

        d = dict(src_dict)
        multibanco = MultibancoTransferInstructions.from_dict(d.pop("multibanco"))

        bank_transfer_instructions_type_4 = cls(
            multibanco=multibanco,
        )

        bank_transfer_instructions_type_4.additional_properties = d
        return bank_transfer_instructions_type_4

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
