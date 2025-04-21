from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.pix_bank_transfer_additional_data import PixBankTransferAdditionalData


T = TypeVar("T", bound="BankTransferAdditionalDataType11")


@_attrs_define
class BankTransferAdditionalDataType11:
    """
    Attributes:
        pix (PixBankTransferAdditionalData):
    """

    pix: "PixBankTransferAdditionalData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        pix = self.pix.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "pix": pix,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.pix_bank_transfer_additional_data import PixBankTransferAdditionalData

        d = dict(src_dict)
        pix = PixBankTransferAdditionalData.from_dict(d.pop("pix"))

        bank_transfer_additional_data_type_11 = cls(
            pix=pix,
        )

        bank_transfer_additional_data_type_11.additional_properties = d
        return bank_transfer_additional_data_type_11

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
